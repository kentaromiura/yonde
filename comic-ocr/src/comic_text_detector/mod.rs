mod yolo_v5;

use burn::tensor::Tensor;
use image::{DynamicImage, GenericImageView};
//use std::path::PathBuf;
use tracing::instrument;

use crate::{device, weights::WeightedTokens, B};

#[derive(Debug, Clone)]
pub struct Bbox<T> {
    pub xmin: T,
    pub xmax: T,
    pub ymin: T,
    pub ymax: T,
    pub confidence: f32,
    pub data: usize,
}

pub struct ComicTextDetector {
    yolo: yolo_v5::YoloV5,
}

// async fn download_detector_weights(filename: &str) -> anyhow::Result<PathBuf> {
//     let repo = "mayocream/comic-text-detector";
//     crate::hf_hub::hf_download(repo, filename).await
// }

const YOLOV5: &'static [u8] = include_bytes!("./yolo-v5.safetensor");

impl ComicTextDetector {
    pub async fn load(_use_cpu: bool) -> anyhow::Result<Self> {
        tracing::info!("Downloading YOLO weights...");
        //let yolo_path = download_detector_weights("yolo-v5.safetensors").await?;
        //let yolo_path = PathBuf::from("./yolo-v5.safetensor");
        let yolo_weights = WeightedTokens::load_safetensors_from_bytes(YOLOV5)?;
        tracing::info!("Loaded {} YOLO tensors", yolo_weights.list_tensors().len());

        let yolo = yolo_v5::YoloV5::load(&yolo_weights, 2, 3)?;
        tracing::info!("YOLO model initialized");

        Ok(Self { yolo })
    }

    #[instrument(level = "debug", skip_all)]
    pub fn inference(&self, image: &DynamicImage) -> anyhow::Result<Vec<Bbox<usize>>> {
        let original_dimensions = image.dimensions();
        let (image_tensor, resized_dimensions) = preprocess(image)?;

        let (predictions, _features) = self.yolo.forward(image_tensor)?;

        let bboxes = postprocess_yolo(&predictions, original_dimensions, resized_dimensions)?;

        Ok(bboxes)
    }
}

fn preprocess(image: &DynamicImage) -> anyhow::Result<(Tensor<B, 4>, (u32, u32))> {
    let (orig_w, orig_h) = image.dimensions();
    let image_size: u32 = 640;

    let (new_w, new_h) = if orig_w >= orig_h {
        (image_size, image_size * orig_h / orig_w)
    } else {
        (image_size * orig_w / orig_h, image_size)
    };

    let resized = image.resize_exact(new_w, new_h, image::imageops::FilterType::Triangle);
    let rgb = resized.to_rgb8();

    let dev = device();
    // Create data in NCHW format: all R values, then all G values, then all B values
    let mut data = vec![0.0f32; (image_size * image_size * 3) as usize];
    let channel_size = (image_size * image_size) as usize;

    for y in 0..new_h {
        for x in 0..new_w {
            let pixel = rgb.get_pixel(x, y);
            let idx = (y * image_size + x) as usize;
            data[idx] = pixel[0] as f32 / 255.0;
            data[channel_size + idx] = pixel[1] as f32 / 255.0;
            data[2 * channel_size + idx] = pixel[2] as f32 / 255.0;
        }
    }

    let shape = [1, 3, image_size as usize, image_size as usize];
    let td = burn::tensor::TensorData::new(data, shape.to_vec());
    let tensor = Tensor::<B, 4>::from_data(td, &dev);

    Ok((tensor, (new_w, new_h)))
}

fn postprocess_yolo(
    predictions: &Tensor<B, 3>,
    original_dimensions: (u32, u32),
    resized_dimensions: (u32, u32),
) -> anyhow::Result<Vec<Bbox<usize>>> {
    const CONFIDENCE_THRESHOLD: f32 = 0.4;
    const NMS_THRESHOLD: f32 = 0.35;
    const BBOX_DILATION: f32 = 1.0;

    let dims = predictions.dims();

    if dims[0] == 0 || dims[1] == 0 || dims[2] == 0 {
        return Ok(Vec::new());
    }

    let _batch = dims[0];
    let num_boxes = dims[1];
    let num_outputs = dims[2];

    if num_outputs < 6 {
        anyhow::bail!(
            "invalid prediction shape: expected at least 6 outputs, got {}",
            num_outputs
        );
    }

    let num_classes = num_outputs - 5;

    let (orig_w, orig_h) = original_dimensions;
    let (resized_w, resized_h) = resized_dimensions;
    let w_ratio = orig_w as f32 / resized_w as f32;
    let h_ratio = orig_h as f32 / resized_h as f32;

    let flat_data: Vec<f32> = predictions.clone().to_data().to_vec().unwrap_or_default();

    let mut boxes_by_class: Vec<Vec<Bbox<usize>>> = (0..num_classes).map(|_| Vec::new()).collect();

    // Process each prediction
    for i in 0..num_boxes {
        let offset = i * num_outputs;
        let cx = flat_data[offset];
        let cy = flat_data[offset + 1];
        let w = flat_data[offset + 2];
        let h = flat_data[offset + 3];
        let objectness = flat_data[offset + 4];

        // Find max class score
        let mut max_class_score = 0.0f32;
        let mut class_idx = 0usize;
        for c in 0..num_classes {
            let score = flat_data[offset + 5 + c];
            if score > max_class_score {
                max_class_score = score;
                class_idx = c;
            }
        }

        let confidence = objectness * max_class_score;

        if confidence < CONFIDENCE_THRESHOLD {
            continue;
        }

        let xmin = ((cx - w / 2.0) * w_ratio - BBOX_DILATION).clamp(0.0, orig_w as f32) as usize;
        let xmax = ((cx + w / 2.0) * w_ratio + BBOX_DILATION).clamp(0.0, orig_w as f32) as usize;
        let ymin = ((cy - h / 2.0) * h_ratio - BBOX_DILATION).clamp(0.0, orig_h as f32) as usize;
        let ymax = ((cy + h / 2.0) * h_ratio + BBOX_DILATION).clamp(0.0, orig_h as f32) as usize;

        let bbox = Bbox {
            xmin,
            xmax,
            ymin,
            ymax,
            confidence,
            data: class_idx,
        };

        boxes_by_class[class_idx].push(bbox);
    }

    // Apply NMS per class
    for boxes in &mut boxes_by_class {
        non_maximum_suppression(boxes, NMS_THRESHOLD);
    }

    let result: Vec<Bbox<usize>> = boxes_by_class.into_iter().flatten().collect();

    tracing::debug!("Returning {} detections", result.len());
    Ok(result)
}

fn non_maximum_suppression(boxes: &mut Vec<Bbox<usize>>, threshold: f32) {
    // Sort by confidence descending
    boxes.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut keep = vec![true; boxes.len()];

    for i in 0..boxes.len() {
        if !keep[i] {
            continue;
        }

        for j in (i + 1)..boxes.len() {
            if !keep[j] {
                continue;
            }

            let iou = calculate_iou(&boxes[i], &boxes[j]);
            if iou > threshold {
                keep[j] = false;
            }
        }
    }

    // Remove suppressed boxes
    let mut i = 0;
    boxes.retain(|_| {
        let k = keep[i];
        i += 1;
        k
    });
}

fn calculate_iou(a: &Bbox<usize>, b: &Bbox<usize>) -> f32 {
    let x_left = a.xmin.max(b.xmin);
    let y_top = a.ymin.max(b.ymin);
    let x_right = a.xmax.min(b.xmax);
    let y_bottom = a.ymax.min(b.ymax);

    if x_right < x_left || y_bottom < y_top {
        return 0.0;
    }

    let intersection_area = (x_right - x_left) * (y_bottom - y_top);
    let box_a_area = (a.xmax - a.xmin) * (a.ymax - a.ymin);
    let box_b_area = (b.xmax - b.xmin) * (b.ymax - b.ymin);

    intersection_area as f32 / (box_a_area + box_b_area - intersection_area) as f32
}
