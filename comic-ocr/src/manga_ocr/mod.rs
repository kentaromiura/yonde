mod model;
mod tokenizer;

use std::path::Path;

use anyhow::{Context, Result};
use burn::tensor::Tensor;
use serde::de::DeserializeOwned;
use tokenizers::Tokenizer;
use tracing::instrument;

use crate::{manga_ocr::tokenizer::load_tokenizer_from_buf, weights::WeightedTokens};
use model::{PreprocessorConfig, VisionEncoderDecoder, VisionEncoderDecoderConfig};

use crate::{device, B};

pub struct MangaOcr {
    model: VisionEncoderDecoder,
    tokenizer: Tokenizer,
    preprocessor: PreprocessorConfig,
}

// NB: Weights were converted to f16 from f32;
const WEIGHTS: &'static [u8] = include_bytes!("./weight.safetensors");
const CONFIG: &'static [u8] = include_bytes!("./config.json");
const PREPROCESSORCONFIG: &'static [u8] = include_bytes!("./preprocessor_config.json");
const VOCAB: &'static [u8] = include_bytes!("./vocab.txt");
const SPECIALTOKENSMAP: &'static [u8] = include_bytes!("./special_tokens_map.json");

impl MangaOcr {
    pub async fn load(_use_cpu: bool) -> Result<Self> {
        let config: VisionEncoderDecoderConfig =
            load_json_from_bytes(CONFIG).context("failed to parse model config")?;
        let preprocessor: PreprocessorConfig = load_json_from_bytes(PREPROCESSORCONFIG)
            .context("failed to parse preprocessor config")?;

        let tokenizer = load_tokenizer_from_buf(VOCAB, SPECIALTOKENSMAP)?;

        let weights = WeightedTokens::load_safetensors_from_bytes(WEIGHTS)?;
        let tensor_names = weights.list_tensors();
        tracing::info!("Loaded {} tensors from weights file", tensor_names.len());

        for name in &tensor_names {
            tracing::debug!("Tensor: {}", name);
        }

        let mut enc_layers = 0;
        let mut dec_layers = 0;
        let mut embeddings = 0;
        let mut predictions = 0;

        for name in &tensor_names {
            if name.starts_with("encoder.encoder.layer.") {
                enc_layers += 1;
            } else if name.starts_with("decoder.bert.encoder.layer.") {
                dec_layers += 1;
            } else if name.contains("embeddings") {
                embeddings += 1;
            } else if name.contains("predictions") {
                predictions += 1;
            }
        }

        tracing::info!(
            "Encoder layers: {}, Decoder layers: {}, Embeddings: {}, Predictions: {}",
            enc_layers,
            dec_layers,
            embeddings,
            predictions
        );

        let model = VisionEncoderDecoder::from_config(&config, &weights)?;

        Ok(Self {
            model,
            tokenizer,
            preprocessor,
        })
    }

    #[instrument(level = "debug", skip_all)]
    pub fn inference(&self, images: &[image::DynamicImage]) -> Result<Vec<String>> {
        if images.is_empty() {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();

        for img in images {
            let tensor = self.preprocess_image(img)?;

            let token_ids = self.model.forward(&tensor)?;

            let text = self.decode_tokens(&token_ids[0]);
            results.push(text);
        }

        Ok(results)
    }

    fn preprocess_image(&self, img: &image::DynamicImage) -> Result<Tensor<B, 4>> {
        let size = self.preprocessor.size as usize;
        // Convert to grayscale first, then to RGB (all channels will have same value)
        let resized = img.grayscale().to_rgb8();
        let resized = image::DynamicImage::ImageRgb8(resized).resize_exact(
            size as u32,
            size as u32,
            image::imageops::FilterType::Triangle,
        );
        let rgb = resized.to_rgb8();

        let mean = self.preprocessor.image_mean;
        let std = self.preprocessor.image_std;

        let dev = device();

        let total = size * size * 3;

        let tensor = match total {
            3072 => {
                let mut flat_data = [0.0f32; 3072];
                for y in 0..size {
                    for x in 0..size {
                        let pixel = rgb.get_pixel(x as u32, y as u32);
                        let r_idx = 0 * size * size + y * size + x;
                        let g_idx = 1 * size * size + y * size + x;
                        let b_idx = 2 * size * size + y * size + x;
                        flat_data[r_idx] = (pixel[0] as f32 / 255.0 - mean[0]) / std[0];
                        flat_data[g_idx] = (pixel[1] as f32 / 255.0 - mean[1]) / std[1];
                        flat_data[b_idx] = (pixel[2] as f32 / 255.0 - mean[2]) / std[2];
                    }
                }
                Tensor::<B, 4>::from_data(flat_data, &dev).reshape([1, 3, size, size])
            }
            12288 => {
                let mut flat_data = [0.0f32; 12288];
                for y in 0..size {
                    for x in 0..size {
                        let pixel = rgb.get_pixel(x as u32, y as u32);
                        let r_idx = 0 * size * size + y * size + x;
                        let g_idx = 1 * size * size + y * size + x;
                        let b_idx = 2 * size * size + y * size + x;
                        flat_data[r_idx] = (pixel[0] as f32 / 255.0 - mean[0]) / std[0];
                        flat_data[g_idx] = (pixel[1] as f32 / 255.0 - mean[1]) / std[1];
                        flat_data[b_idx] = (pixel[2] as f32 / 255.0 - mean[2]) / std[2];
                    }
                }
                Tensor::<B, 4>::from_data(flat_data, &dev).reshape([1, 3, size, size])
            }
            n => {
                tracing::info!("Preprocessing image with {} pixels", n);
                // Create data with channels separated: [3, H, W]
                let mut flat_data = vec![0.0f32; total];
                for y in 0..size {
                    for x in 0..size {
                        let pixel = rgb.get_pixel(x as u32, y as u32);
                        let r_idx = 0 * size * size + y * size + x;
                        let g_idx = 1 * size * size + y * size + x;
                        let b_idx = 2 * size * size + y * size + x;
                        flat_data[r_idx] = (pixel[0] as f32 / 255.0 - mean[0]) / std[0];
                        flat_data[g_idx] = (pixel[1] as f32 / 255.0 - mean[1]) / std[1];
                        flat_data[b_idx] = (pixel[2] as f32 / 255.0 - mean[2]) / std[2];
                    }
                }
                // Calculate image statistics to verify it's not all zeros
                let sum: f32 = flat_data.iter().sum();
                let max_val = flat_data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                let min_val = flat_data.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                tracing::info!(
                    "Input tensor stats - sum: {:.2}, max: {:.2}, min: {:.2}",
                    sum,
                    max_val,
                    min_val
                );
                let td = burn::tensor::TensorData::new(flat_data, vec![1, 3, size, size]);
                Tensor::<B, 4>::from_data(td, &dev)
            }
        };

        Ok(tensor)
    }

    fn decode_tokens(&self, token_ids: &[u32]) -> String {
        let text = self.tokenizer.decode(token_ids, true).unwrap_or_default();
        post_process(&text)
    }
}

fn post_process(text: &str) -> String {
    let mut clean = text
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>();
    clean = clean.replace('\u{2026}', "...");
    clean = collapse_dots(&clean);
    halfwidth_to_fullwidth(&clean)
}

fn collapse_dots(text: &str) -> String {
    let mut out = String::new();
    let mut count = 0usize;
    for ch in text.chars() {
        if ch == '.' || ch == '\u{30fb}' {
            count += 1;
        } else {
            if count > 0 {
                for _ in 0..count {
                    out.push('.');
                }
                count = 0;
            }
            out.push(ch);
        }
    }
    if count > 0 {
        for _ in 0..count {
            out.push('.');
        }
    }
    out
}

fn halfwidth_to_fullwidth(text: &str) -> String {
    text.chars()
        .map(|ch| match ch {
            '!'..='~' => char::from_u32(ch as u32 + 0xFEE0).unwrap_or(ch),
            ' ' => '\u{3000}',
            _ => ch,
        })
        .collect()
}

fn load_json_from_bytes<T: DeserializeOwned>(data: &'static [u8]) -> Result<T> {
    let str = str::from_utf8(data)?;
    let parsed = serde_json::from_str(str).with_context(|| format!("failed to parse json"))?;
    Ok(parsed)
}

#[allow(dead_code)]
fn load_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let data = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let parsed = serde_json::from_str(&data)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(parsed)
}
