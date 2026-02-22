use serde::Deserialize;

use burn::tensor::activation::{gelu, softmax};
use burn::tensor::Tensor;

pub type B = crate::B;

struct WeightedConv2d {
    weight: Tensor<B, 4>,
    bias: Option<Tensor<B, 1>>,
    stride: usize,
}

impl WeightedConv2d {
    fn from_weights(
        weights: &crate::weights::WeightedTokens,
        name: &str,
        in_channels: usize,
        out_channels: usize,
        kernel_size: usize,
        stride: usize,
    ) -> anyhow::Result<Self> {
        let dev = crate::device();

        let weight_name = format!("{}.weight", name);
        let bias_name = format!("{}.bias", name);

        let weight_data = weights.get_float_tensor(&weight_name)?;
        let td = burn::tensor::TensorData::new(
            weight_data,
            vec![out_channels, in_channels, kernel_size, kernel_size],
        );
        let weight = Tensor::<B, 4>::from_data(td, &dev);
        tracing::info!("Loaded conv weight: {}", weight_name);

        let bias = if let Ok(data) = weights.get_float_tensor(&bias_name) {
            let td = burn::tensor::TensorData::new(data, vec![out_channels]);
            tracing::info!("Loaded conv bias: {}", bias_name);
            Some(Tensor::<B, 1>::from_data(td, &dev))
        } else {
            tracing::warn!("Missing conv bias: {}", bias_name);
            None
        };

        Ok(Self {
            weight,
            bias,
            stride,
        })
    }

    fn forward(&self, x: Tensor<B, 4>) -> Tensor<B, 4> {
        use burn::tensor::ops::ConvOptions;
        let options = ConvOptions::new([self.stride, self.stride], [0, 0], [1, 1], 1);
        burn::tensor::module::conv2d(x, self.weight.clone(), self.bias.clone(), options)
    }
}

pub struct WeightedLinear {
    weight: Tensor<B, 2>,
    bias: Option<Tensor<B, 1>>,
}

impl WeightedLinear {
    fn from_weights(
        weights: &crate::weights::WeightedTokens,
        name: &str,
        in_dim: usize,
        out_dim: usize,
    ) -> anyhow::Result<Self> {
        let dev = crate::device();

        let weight_name = format!("{}.weight", name);
        let bias_name = format!("{}.bias", name);

        let weight_data_vec: Vec<f32> = if let Ok(data) = weights.get_float_tensor(&weight_name) {
            let copy_len = data.len().min(in_dim * out_dim);
            let mut w = vec![0.0f32; in_dim * out_dim];
            w[..copy_len].copy_from_slice(&data[..copy_len]);
            tracing::info!("Loaded weight {}: {} values", weight_name, copy_len);
            w
        } else {
            tracing::warn!("Missing weight: {}", weight_name);
            vec![0.0f32; in_dim * out_dim]
        };

        let bias = if let Ok(data) = weights.get_float_tensor(&bias_name) {
            let copy_len = data.len().min(out_dim);
            let mut b = vec![0.0f32; out_dim];
            b[..copy_len].copy_from_slice(&data[..copy_len]);
            tracing::info!("Loaded bias {}: {} values", bias_name, copy_len);
            let td = burn::tensor::TensorData::new(b, vec![out_dim]);
            Some(Tensor::<B, 1>::from_data(td, &dev))
        } else {
            tracing::warn!("Missing bias: {}", bias_name);
            None
        };

        let td = burn::tensor::TensorData::new(weight_data_vec, vec![out_dim, in_dim]);
        let weight = Tensor::<B, 2>::from_data(td, &dev);

        Ok(Self { weight, bias })
    }

    fn forward(&self, input: Tensor<B, 2>) -> Tensor<B, 2> {
        let output = input.matmul(self.weight.clone().transpose());
        if let Some(bias) = &self.bias {
            output + bias.clone().reshape([1, bias.dims()[0]])
        } else {
            output
        }
    }
}

#[allow(unused)]
#[derive(Debug, Deserialize, Clone)]
pub struct VisionEncoderDecoderConfig {
    pub decoder_start_token_id: u32,
    pub eos_token_id: u32,
    pub pad_token_id: u32,
    pub max_length: usize,
    pub encoder: VitConfig,
    pub decoder: BertConfig,
}

#[allow(unused)]
#[derive(Debug, Deserialize, Clone)]
pub struct PreprocessorConfig {
    pub size: u32,
    pub image_mean: [f32; 3],
    pub image_std: [f32; 3],
    pub do_resize: bool,
    pub do_normalize: bool,
}

#[allow(unused)]
#[derive(Debug, Deserialize, Clone)]
pub struct VitConfig {
    pub image_size: usize,
    pub patch_size: usize,
    pub hidden_size: usize,
    pub num_hidden_layers: usize,
    pub num_attention_heads: usize,
    pub intermediate_size: usize,
    pub hidden_act: HiddenAct,
    pub hidden_dropout_prob: f64,
    pub attention_probs_dropout_prob: f64,
    pub layer_norm_eps: f64,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum HiddenAct {
    Gelu,
    #[serde(other)]
    GeluApproximate,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum DecoderHiddenAct {
    Gelu,
    #[serde(other)]
    GeluApproximate,
}

#[allow(unused)]
#[derive(Debug, Deserialize, Clone)]
pub struct BertConfig {
    pub vocab_size: usize,
    pub hidden_size: usize,
    pub num_hidden_layers: usize,
    pub num_attention_heads: usize,
    pub intermediate_size: usize,
    pub hidden_act: DecoderHiddenAct,
    pub hidden_dropout_prob: f64,
    pub attention_probs_dropout_prob: f64,
    pub max_position_embeddings: usize,
    pub type_vocab_size: usize,
    pub layer_norm_eps: f64,
    pub pad_token_id: Option<u32>,
}

struct LayerNorm {
    weight: Tensor<B, 1>,
    bias: Tensor<B, 1>,
    eps: f32,
}

impl LayerNorm {
    fn from_weights(
        weights: &crate::weights::WeightedTokens,
        name: &str,
        hidden_size: usize,
    ) -> anyhow::Result<Self> {
        let dev = crate::device();

        let weight_name = format!("{}.weight", name);
        let bias_name = format!("{}.bias", name);

        let mut weight_data = vec![1.0f32; hidden_size];
        if let Ok(data) = weights.get_float_tensor(&weight_name) {
            let copy_len = data.len().min(hidden_size);
            weight_data[..copy_len].copy_from_slice(&data[..copy_len]);
        }

        let mut bias_data = vec![0.0f32; hidden_size];
        if let Ok(data) = weights.get_float_tensor(&bias_name) {
            let copy_len = data.len().min(hidden_size);
            bias_data[..copy_len].copy_from_slice(&data[..copy_len]);
        }

        let td_w = burn::tensor::TensorData::new(weight_data, vec![hidden_size]);
        let td_b = burn::tensor::TensorData::new(bias_data, vec![hidden_size]);

        Ok(Self {
            weight: Tensor::<B, 1>::from_data(td_w, &dev),
            bias: Tensor::<B, 1>::from_data(td_b, &dev),
            eps: 1e-12,
        })
    }

    fn forward(&self, x: &Tensor<B, 3>) -> Tensor<B, 3> {
        let hidden_size = self.weight.dims()[0];
        let batch_size = x.dims()[0];
        let seq_len = x.dims()[1];

        let x_2d = x.clone().reshape([batch_size * seq_len, hidden_size]);
        let mean = x_2d.clone().mean_dim(1);
        let mean_reshaped = mean.clone().reshape([batch_size * seq_len, 1]);
        let diff = x_2d.sub(mean_reshaped.clone());
        let var = diff.clone().mul(diff.clone()).mean_dim(1);
        let std = (var + self.eps).sqrt();
        let std_reshaped = std.reshape([batch_size * seq_len, 1]);
        let normalized = diff.div(std_reshaped);

        let weight = self.weight.clone().reshape([1, 1, hidden_size]);
        let bias = self.bias.clone().reshape([1, 1, hidden_size]);

        normalized.reshape([batch_size, seq_len, hidden_size]) * weight + bias
    }
}

struct MultiHeadAttention {
    num_heads: usize,
    head_dim: usize,
    query: WeightedLinear,
    key: WeightedLinear,
    value: WeightedLinear,
    output: WeightedLinear,
    output_layernorm: LayerNorm,
}

impl MultiHeadAttention {
    fn from_weights(
        weights: &crate::weights::WeightedTokens,
        name: &str,
        hidden_size: usize,
        num_heads: usize,
    ) -> anyhow::Result<Self> {
        let head_dim = hidden_size / num_heads;

        let query = WeightedLinear::from_weights(
            weights,
            &format!("{}.query", name),
            hidden_size,
            hidden_size,
        )
        .unwrap_or_else(|_| WeightedLinear {
            weight: Tensor::<B, 2>::zeros([hidden_size, hidden_size], &crate::device()),
            bias: None,
        });

        let key = WeightedLinear::from_weights(
            weights,
            &format!("{}.key", name),
            hidden_size,
            hidden_size,
        )
        .unwrap_or_else(|_| WeightedLinear {
            weight: Tensor::<B, 2>::zeros([hidden_size, hidden_size], &crate::device()),
            bias: None,
        });

        let value = WeightedLinear::from_weights(
            weights,
            &format!("{}.value", name),
            hidden_size,
            hidden_size,
        )
        .unwrap_or_else(|_| WeightedLinear {
            weight: Tensor::<B, 2>::zeros([hidden_size, hidden_size], &crate::device()),
            bias: None,
        });

        // Load output projection from attention.output.dense
        // Handle both encoder pattern (attention.attention) and decoder pattern (attention.self or crossattention.self)
        let output_name = name
            .replace(".attention.attention", ".attention.output")
            .replace(".attention.self", ".attention.output")
            .replace(".crossattention.self", ".crossattention.output");
        let output = WeightedLinear::from_weights(
            weights,
            &format!("{}.dense", output_name),
            hidden_size,
            hidden_size,
        )
        .unwrap_or_else(|_| WeightedLinear {
            weight: Tensor::<B, 2>::zeros([hidden_size, hidden_size], &crate::device()),
            bias: None,
        });

        // Load output layer norm
        let output_layernorm =
            LayerNorm::from_weights(weights, &format!("{}.LayerNorm", output_name), hidden_size)
                .unwrap_or_else(|_| LayerNorm {
                    weight: Tensor::<B, 1>::ones([hidden_size], &crate::device()),
                    bias: Tensor::<B, 1>::zeros([hidden_size], &crate::device()),
                    eps: 1e-12,
                });

        Ok(Self {
            num_heads,
            head_dim,
            query,
            key,
            value,
            output,
            output_layernorm,
        })
    }

    fn forward(
        &self,
        hidden_states: &Tensor<B, 3>,
        encoder_hidden_states: Option<&Tensor<B, 3>>,
    ) -> Tensor<B, 3> {
        let batch_size = hidden_states.dims()[0];
        let seq_len = hidden_states.dims()[1];
        let hidden_size = hidden_states.dims()[2];

        let q = {
            let flat = hidden_states
                .clone()
                .reshape([batch_size * seq_len, hidden_size]);
            self.query
                .forward(flat)
                .reshape([batch_size, seq_len, self.num_heads, self.head_dim])
        };

        let (k, v, _kv_seq_len) = if let Some(enc_states) = encoder_hidden_states {
            let enc_batch = enc_states.dims()[0];
            let enc_seq = enc_states.dims()[1];

            let k = {
                let flat = enc_states
                    .clone()
                    .reshape([enc_batch * enc_seq, hidden_size]);
                self.key
                    .forward(flat)
                    .reshape([enc_batch, enc_seq, self.num_heads, self.head_dim])
            };
            let v = {
                let flat = enc_states
                    .clone()
                    .reshape([enc_batch * enc_seq, hidden_size]);
                self.value.forward(flat).reshape([
                    enc_batch,
                    enc_seq,
                    self.num_heads,
                    self.head_dim,
                ])
            };
            (k, v, enc_seq)
        } else {
            let k = {
                let flat = hidden_states
                    .clone()
                    .reshape([batch_size * seq_len, hidden_size]);
                self.key
                    .forward(flat)
                    .reshape([batch_size, seq_len, self.num_heads, self.head_dim])
            };
            let v = {
                let flat = hidden_states
                    .clone()
                    .reshape([batch_size * seq_len, hidden_size]);
                self.value.forward(flat).reshape([
                    batch_size,
                    seq_len,
                    self.num_heads,
                    self.head_dim,
                ])
            };
            (k, v, seq_len)
        };

        let scale = (self.head_dim as f32).sqrt();

        let q_perm = q.permute([0, 2, 1, 3]);
        let k_perm = k.permute([0, 2, 3, 1]);

        let attn_scores = q_perm.matmul(k_perm) / scale;

        let attn_weights = softmax(attn_scores, 3);

        let v_perm = v.permute([0, 2, 1, 3]);
        let context = attn_weights.matmul(v_perm);

        let context_t = context.permute([0, 2, 1, 3]);

        let context_flat = context_t.reshape([batch_size * seq_len, hidden_size]);

        // Apply output projection
        let output = self
            .output
            .forward(context_flat)
            .reshape([batch_size, seq_len, hidden_size]);

        // Add residual and apply layer norm (BertSelfOutput pattern)
        let output_with_residual = output + hidden_states.clone();
        self.output_layernorm.forward(&output_with_residual)
    }
}

struct FeedForward {
    dense1: WeightedLinear,
    dense2: WeightedLinear,
}

impl FeedForward {
    fn from_weights(
        weights: &crate::weights::WeightedTokens,
        name: &str,
        hidden_size: usize,
        intermediate_size: usize,
    ) -> anyhow::Result<Self> {
        let dense1 = WeightedLinear::from_weights(
            weights,
            &format!("{}.intermediate.dense", name),
            hidden_size,
            intermediate_size,
        )
        .unwrap_or_else(|_| WeightedLinear {
            weight: Tensor::<B, 2>::zeros([intermediate_size, hidden_size], &crate::device()),
            bias: None,
        });

        let dense2 = WeightedLinear::from_weights(
            weights,
            &format!("{}.output.dense", name),
            intermediate_size,
            hidden_size,
        )
        .unwrap_or_else(|_| WeightedLinear {
            weight: Tensor::<B, 2>::zeros([hidden_size, intermediate_size], &crate::device()),
            bias: None,
        });

        Ok(Self { dense1, dense2 })
    }

    fn forward(&self, hidden_states: &Tensor<B, 3>) -> Tensor<B, 3> {
        let batch_size = hidden_states.dims()[0];
        let seq_len = hidden_states.dims()[1];
        let hidden_size = hidden_states.dims()[2];

        let intermediate = {
            let flat = hidden_states
                .clone()
                .reshape([batch_size * seq_len, hidden_size]);
            let activated = self.dense1.forward(flat);
            let activated = gelu(activated);
            let hidden_dim = activated.dims()[1];
            activated.reshape([batch_size, seq_len, hidden_dim])
        };

        let intermediate_batch = intermediate.dims()[0];
        let intermediate_seq = intermediate.dims()[1];
        let intermediate_hidden = intermediate.dims()[2];

        let output = {
            let flat =
                intermediate.reshape([intermediate_batch * intermediate_seq, intermediate_hidden]);
            self.dense2
                .forward(flat)
                .reshape([batch_size, seq_len, hidden_size])
        };

        output
    }
}

struct ViTAttention {
    num_heads: usize,
    head_dim: usize,
    query: WeightedLinear,
    key: WeightedLinear,
    value: WeightedLinear,
    output: WeightedLinear,
}

impl ViTAttention {
    fn from_weights(
        weights: &crate::weights::WeightedTokens,
        name: &str,
        hidden_size: usize,
        num_heads: usize,
    ) -> anyhow::Result<Self> {
        let head_dim = hidden_size / num_heads;

        let query = WeightedLinear::from_weights(
            weights,
            &format!("{}.query", name),
            hidden_size,
            hidden_size,
        )
        .unwrap_or_else(|_| WeightedLinear {
            weight: Tensor::<B, 2>::zeros([hidden_size, hidden_size], &crate::device()),
            bias: None,
        });

        let key = WeightedLinear::from_weights(
            weights,
            &format!("{}.key", name),
            hidden_size,
            hidden_size,
        )
        .unwrap_or_else(|_| WeightedLinear {
            weight: Tensor::<B, 2>::zeros([hidden_size, hidden_size], &crate::device()),
            bias: None,
        });

        let value = WeightedLinear::from_weights(
            weights,
            &format!("{}.value", name),
            hidden_size,
            hidden_size,
        )
        .unwrap_or_else(|_| WeightedLinear {
            weight: Tensor::<B, 2>::zeros([hidden_size, hidden_size], &crate::device()),
            bias: None,
        });

        let output_name = name.replace(".attention.attention", ".attention.output");
        let output = WeightedLinear::from_weights(
            weights,
            &format!("{}.dense", output_name),
            hidden_size,
            hidden_size,
        )
        .unwrap_or_else(|_| WeightedLinear {
            weight: Tensor::<B, 2>::zeros([hidden_size, hidden_size], &crate::device()),
            bias: None,
        });

        Ok(Self {
            num_heads,
            head_dim,
            query,
            key,
            value,
            output,
        })
    }

    fn forward(&self, hidden_states: &Tensor<B, 3>) -> Tensor<B, 3> {
        let batch_size = hidden_states.dims()[0];
        let seq_len = hidden_states.dims()[1];
        let hidden_size = hidden_states.dims()[2];

        let q = {
            let flat = hidden_states
                .clone()
                .reshape([batch_size * seq_len, hidden_size]);
            self.query
                .forward(flat)
                .reshape([batch_size, seq_len, self.num_heads, self.head_dim])
        };

        let k = {
            let flat = hidden_states
                .clone()
                .reshape([batch_size * seq_len, hidden_size]);
            self.key
                .forward(flat)
                .reshape([batch_size, seq_len, self.num_heads, self.head_dim])
        };

        let v = {
            let flat = hidden_states
                .clone()
                .reshape([batch_size * seq_len, hidden_size]);
            self.value
                .forward(flat)
                .reshape([batch_size, seq_len, self.num_heads, self.head_dim])
        };

        let scale = (self.head_dim as f32).sqrt();

        let q_perm = q.permute([0, 2, 1, 3]);
        let k_perm = k.permute([0, 2, 3, 1]);

        let attn_scores = q_perm.matmul(k_perm) / scale;

        let attn_weights = softmax(attn_scores, 3);

        let v_perm = v.permute([0, 2, 1, 3]);
        let context = attn_weights.matmul(v_perm);

        let context_t = context.permute([0, 2, 1, 3]);

        let context_flat = context_t.reshape([batch_size * seq_len, hidden_size]);

        self.output
            .forward(context_flat)
            .reshape([batch_size, seq_len, hidden_size])
    }
}

struct TransformerEncoderLayer {
    attention: ViTAttention,
    feed_forward: FeedForward,
    layernorm_before: LayerNorm,
    layernorm_after: LayerNorm,
}

impl TransformerEncoderLayer {
    fn from_weights(
        weights: &crate::weights::WeightedTokens,
        layer_idx: usize,
        hidden_size: usize,
        num_heads: usize,
        intermediate_size: usize,
    ) -> anyhow::Result<Self> {
        let name = format!("encoder.encoder.layer.{}.attention.attention", layer_idx);

        let attention = ViTAttention::from_weights(weights, &name, hidden_size, num_heads)?;

        let feed_forward = FeedForward::from_weights(
            weights,
            &format!("encoder.encoder.layer.{}", layer_idx),
            hidden_size,
            intermediate_size,
        )?;

        let layernorm_before = LayerNorm::from_weights(
            weights,
            &format!("encoder.encoder.layer.{}.layernorm_before", layer_idx),
            hidden_size,
        )?;

        let layernorm_after = LayerNorm::from_weights(
            weights,
            &format!("encoder.encoder.layer.{}.layernorm_after", layer_idx),
            hidden_size,
        )?;

        Ok(Self {
            attention,
            feed_forward,
            layernorm_before,
            layernorm_after,
        })
    }

    fn forward(&self, hidden_states: &Tensor<B, 3>) -> Tensor<B, 3> {
        // ViT Pre-LN pattern: Residual + Attention(LayerNorm(x))
        let residual = hidden_states.clone();
        let normed = self.layernorm_before.forward(hidden_states);
        let attn_output = self.attention.forward(&normed);
        let hidden_states = residual + attn_output;

        // ViT Pre-LN pattern: Residual + FFN(LayerNorm(x))
        let residual = hidden_states.clone();
        let normed = self.layernorm_after.forward(&hidden_states);
        let ff_output = self.feed_forward.forward(&normed);
        residual + ff_output
    }
}

struct TransformerDecoderLayer {
    self_attention: MultiHeadAttention,
    cross_attention: MultiHeadAttention,
    feed_forward: FeedForward,
    layernorm3: LayerNorm,
}

impl TransformerDecoderLayer {
    fn from_weights(
        weights: &crate::weights::WeightedTokens,
        layer_idx: usize,
        hidden_size: usize,
        num_heads: usize,
        intermediate_size: usize,
    ) -> anyhow::Result<Self> {
        let name = format!("decoder.bert.encoder.layer.{}.attention.self", layer_idx);

        let self_attention =
            MultiHeadAttention::from_weights(weights, &name, hidden_size, num_heads)?;

        let cross_attention = MultiHeadAttention::from_weights(
            weights,
            &format!(
                "decoder.bert.encoder.layer.{}.crossattention.self",
                layer_idx
            ),
            hidden_size,
            num_heads,
        )?;

        let feed_forward = FeedForward::from_weights(
            weights,
            &format!("decoder.bert.encoder.layer.{}", layer_idx),
            hidden_size,
            intermediate_size,
        )?;

        let layernorm3 = LayerNorm::from_weights(
            weights,
            &format!("decoder.bert.encoder.layer.{}.output.LayerNorm", layer_idx),
            hidden_size,
        )?;

        Ok(Self {
            self_attention,
            cross_attention,
            feed_forward,
            layernorm3,
        })
    }

    fn forward(
        &self,
        hidden_states: &Tensor<B, 3>,
        encoder_hidden_states: &Tensor<B, 3>,
    ) -> Tensor<B, 3> {
        // Self-attention (already includes residual + layer norm inside BertSelfOutput pattern)
        let hidden_states = self.self_attention.forward(hidden_states, None);

        // Cross-attention (already includes residual + layer norm inside)
        let hidden_states = self
            .cross_attention
            .forward(&hidden_states, Some(encoder_hidden_states));

        // Feed-forward with residual
        // Note: FeedForward only does dense+activation+dense, no residual/LN
        let residual = hidden_states.clone();
        let ff_output = self.feed_forward.forward(&hidden_states);
        self.layernorm3.forward(&(residual + ff_output))
    }
}

pub struct VitEncoder {
    patch_embed: WeightedConv2d,
    cls_token: Tensor<B, 3>,
    position_embeddings: Tensor<B, 2>,
    layernorm: LayerNorm,
    layers: Vec<TransformerEncoderLayer>,
    config: VitConfig,
}

impl VitEncoder {
    pub fn from_weights(
        config: &VitConfig,
        weights: &crate::weights::WeightedTokens,
    ) -> anyhow::Result<Self> {
        let dev = crate::device();

        let patch_embed = WeightedConv2d::from_weights(
            weights,
            "encoder.embeddings.patch_embeddings.projection",
            3,
            config.hidden_size,
            config.patch_size,
            config.patch_size,
        )?;

        let cls_token = if let Ok(data) = weights.get_float_tensor("encoder.embeddings.cls_token") {
            tracing::info!("Loaded cls_token");
            let td = burn::tensor::TensorData::new(data, vec![1, 1, config.hidden_size]);
            Tensor::<B, 3>::from_data(td, &dev)
        } else {
            tracing::warn!("Missing cls_token, using zeros");
            Tensor::<B, 3>::zeros([1, 1, config.hidden_size], &dev)
        };

        let position_embeddings =
            if let Ok(data) = weights.get_float_tensor("encoder.embeddings.position_embeddings") {
                let len = data.len();
                let seq_len = len / config.hidden_size;
                tracing::info!(
                    "Loaded position embeddings: {} x {}",
                    seq_len,
                    config.hidden_size
                );
                let td = burn::tensor::TensorData::new(data, vec![seq_len, config.hidden_size]);
                Tensor::<B, 2>::from_data(td, &dev)
            } else {
                tracing::warn!("Missing position embeddings, using zeros");
                let num_patches =
                    config.image_size * config.image_size / (config.patch_size * config.patch_size);
                Tensor::<B, 2>::zeros([num_patches + 1, config.hidden_size], &dev)
            };

        let layernorm = LayerNorm::from_weights(weights, "encoder.layernorm", config.hidden_size)?;

        let mut layers = Vec::new();
        for i in 0..config.num_hidden_layers {
            match TransformerEncoderLayer::from_weights(
                weights,
                i,
                config.hidden_size,
                config.num_attention_heads,
                config.intermediate_size,
            ) {
                Ok(layer) => layers.push(layer),
                Err(e) => tracing::warn!("Failed to load encoder layer {}: {}", i, e),
            }
        }
        tracing::info!("Loaded {} encoder layers", layers.len());

        Ok(Self {
            patch_embed,
            cls_token,
            position_embeddings,
            layernorm,
            layers,
            config: config.clone(),
        })
    }

    pub fn forward(&self, pixel_values: &Tensor<B, 4>) -> anyhow::Result<Tensor<B, 3>> {
        let batch_size = pixel_values.dims()[0];

        let patches = self.patch_embed.forward(pixel_values.clone());

        let channels = patches.dims()[1];
        let height = patches.dims()[2];
        let width = patches.dims()[3];
        let num_patches = height * width;

        // Reshape patches: [B, C, H, W] -> [B, num_patches, C]
        let patch_embeddings = patches
            .reshape([batch_size, channels, num_patches])
            .transpose();

        // Prepend cls token: [B, num_patches, C] -> [B, 1 + num_patches, C]
        let cls_tokens = self
            .cls_token
            .clone()
            .reshape([batch_size, 1, self.config.hidden_size]);
        let hidden_states = Tensor::cat(vec![cls_tokens, patch_embeddings], 1);

        // Add position embeddings (includes position for cls token + patches)
        let seq_len = 1 + num_patches;
        let pos_emb = self
            .position_embeddings
            .clone()
            .slice([0..seq_len, 0..self.config.hidden_size])
            .reshape([1, seq_len, self.config.hidden_size]);
        let hidden_states = hidden_states + pos_emb;

        let mut hidden_states = hidden_states;
        for layer in &self.layers {
            hidden_states = layer.forward(&hidden_states);
        }

        hidden_states = self.layernorm.forward(&hidden_states);

        Ok(hidden_states)
    }
}

pub struct BertDecoder {
    embeddings: Tensor<B, 2>,
    position_embeddings: Tensor<B, 2>,
    token_type_embeddings: Tensor<B, 2>,
    layernorm: LayerNorm,
    layers: Vec<TransformerDecoderLayer>,
    transform_dense: WeightedLinear,
    transform_layernorm: LayerNorm,
    lm_head: WeightedLinear,
    lm_bias: Tensor<B, 1>,
    config: BertConfig,
}

impl BertDecoder {
    pub fn from_weights(
        config: &BertConfig,
        weights: &crate::weights::WeightedTokens,
    ) -> anyhow::Result<Self> {
        let dev = crate::device();

        let embeddings = if let Ok(data) =
            weights.get_float_tensor("decoder.bert.embeddings.word_embeddings.weight")
        {
            let len = data.len();
            let vocab_size = len / config.hidden_size;
            tracing::info!(
                "Loaded token embeddings: {} x {}",
                vocab_size,
                config.hidden_size
            );
            let td = burn::tensor::TensorData::new(data, vec![vocab_size, config.hidden_size]);
            Tensor::<B, 2>::from_data(td, &dev)
        } else {
            tracing::warn!("Missing token embeddings, using zeros");
            Tensor::<B, 2>::zeros([config.vocab_size, config.hidden_size], &dev)
        };

        let position_embeddings = if let Ok(data) =
            weights.get_float_tensor("decoder.bert.embeddings.position_embeddings.weight")
        {
            let len = data.len();
            let max_positions = len / config.hidden_size;
            tracing::info!(
                "Loaded position embeddings: {} x {}",
                max_positions,
                config.hidden_size
            );
            let td = burn::tensor::TensorData::new(data, vec![max_positions, config.hidden_size]);
            Tensor::<B, 2>::from_data(td, &dev)
        } else {
            tracing::warn!("Missing position embeddings, using zeros");
            Tensor::<B, 2>::zeros([config.max_position_embeddings, config.hidden_size], &dev)
        };

        let token_type_embeddings = if let Ok(data) =
            weights.get_float_tensor("decoder.bert.embeddings.token_type_embeddings.weight")
        {
            tracing::info!("Loaded token_type_embeddings");
            let td = burn::tensor::TensorData::new(data, vec![2, config.hidden_size]);
            Tensor::<B, 2>::from_data(td, &dev)
        } else {
            tracing::warn!("Missing token_type_embeddings, using zeros");
            Tensor::<B, 2>::zeros([2, config.hidden_size], &dev)
        };

        let layernorm = LayerNorm::from_weights(
            weights,
            "decoder.bert.embeddings.LayerNorm",
            config.hidden_size,
        )
        .unwrap_or_else(|_| LayerNorm {
            weight: Tensor::<B, 1>::ones([config.hidden_size], &dev),
            bias: Tensor::<B, 1>::zeros([config.hidden_size], &dev),
            eps: 1e-12,
        });

        let mut layers = Vec::new();
        for i in 0..config.num_hidden_layers {
            match TransformerDecoderLayer::from_weights(
                weights,
                i,
                config.hidden_size,
                config.num_attention_heads,
                config.intermediate_size,
            ) {
                Ok(layer) => layers.push(layer),
                Err(e) => tracing::warn!("Failed to load decoder layer {}: {}", i, e),
            }
        }
        tracing::info!("Loaded {} decoder layers", layers.len());

        // Load prediction head transform (BertPredictionHeadTransform)
        let transform_dense = WeightedLinear::from_weights(
            weights,
            "decoder.cls.predictions.transform.dense",
            config.hidden_size,
            config.hidden_size,
        )
        .unwrap_or_else(|_| WeightedLinear {
            weight: Tensor::<B, 2>::zeros([config.hidden_size, config.hidden_size], &dev),
            bias: None,
        });

        let transform_layernorm = LayerNorm::from_weights(
            weights,
            "decoder.cls.predictions.transform.LayerNorm",
            config.hidden_size,
        )
        .unwrap_or_else(|_| LayerNorm {
            weight: Tensor::<B, 1>::ones([config.hidden_size], &dev),
            bias: Tensor::<B, 1>::zeros([config.hidden_size], &dev),
            eps: 1e-12,
        });

        let lm_head = WeightedLinear::from_weights(
            weights,
            "decoder.cls.predictions.decoder",
            config.hidden_size,
            config.vocab_size,
        )
        .unwrap_or_else(|_| WeightedLinear {
            weight: Tensor::<B, 2>::zeros([config.vocab_size, config.hidden_size], &dev),
            bias: None,
        });

        let lm_bias = if let Ok(data) = weights.get_float_tensor("decoder.cls.predictions.bias") {
            tracing::info!("Loaded lm_bias");
            let td = burn::tensor::TensorData::new(data, vec![config.vocab_size]);
            Tensor::<B, 1>::from_data(td, &dev)
        } else {
            Tensor::<B, 1>::zeros([config.vocab_size], &dev)
        };

        Ok(Self {
            embeddings,
            position_embeddings,
            token_type_embeddings,
            layernorm,
            layers,
            transform_dense,
            transform_layernorm,
            lm_head,
            lm_bias,
            config: config.clone(),
        })
    }

    pub fn forward(
        &self,
        input_ids: &[u32],
        encoder_hidden_states: &Tensor<B, 3>,
    ) -> anyhow::Result<Tensor<B, 3>> {
        let dev = crate::device();
        let seq_len = input_ids.len();
        let hidden_size = self.config.hidden_size;
        let vocab_size = self.config.vocab_size;

        // Get word embeddings
        let mut input_embeddings = Tensor::<B, 2>::zeros([seq_len, hidden_size], &dev);
        for (i, &token_id) in input_ids.iter().enumerate() {
            if token_id as usize >= self.embeddings.dims()[0] {
                continue;
            }
            let token_emb = self
                .embeddings
                .clone()
                .slice([token_id as usize..token_id as usize + 1, 0..hidden_size]);
            input_embeddings = input_embeddings
                .clone()
                .slice_assign([i..i + 1, 0..hidden_size], token_emb);
        }

        // Add position embeddings
        let positions: Vec<usize> = (0..seq_len).collect();
        let mut position_embeds = Tensor::<B, 2>::zeros([seq_len, hidden_size], &dev);
        for (i, &pos) in positions.iter().enumerate() {
            if pos >= self.position_embeddings.dims()[0] {
                break;
            }
            let pos_emb = self
                .position_embeddings
                .clone()
                .slice([pos..pos + 1, 0..hidden_size]);
            position_embeds = position_embeds
                .clone()
                .slice_assign([i..i + 1, 0..hidden_size], pos_emb);
        }

        // Get token_type_embeddings (all zeros since we have a single sequence)
        let token_type_emb = self
            .token_type_embeddings
            .clone()
            .slice([0..1, 0..hidden_size]);
        let mut token_type_embeds = Tensor::<B, 2>::zeros([seq_len, hidden_size], &dev);
        for i in 0..seq_len {
            token_type_embeds = token_type_embeds
                .clone()
                .slice_assign([i..i + 1, 0..hidden_size], token_type_emb.clone());
        }

        // Combine word, position, and token_type embeddings
        let combined_embeddings = input_embeddings + position_embeds + token_type_embeds;
        let mut hidden_states = combined_embeddings.reshape([1, seq_len, hidden_size]);

        hidden_states = self.layernorm.forward(&hidden_states);

        let mut hidden_states = hidden_states;
        for (_layer_idx, layer) in self.layers.iter().enumerate() {
            hidden_states = layer.forward(&hidden_states, encoder_hidden_states);
        }

        // Apply prediction head transform (BertPredictionHeadTransform)
        let flat = hidden_states.reshape([seq_len, hidden_size]);
        let transformed = self.transform_dense.forward(flat);
        let transformed = gelu(transformed);
        let transformed =
            self.transform_layernorm
                .forward(&transformed.reshape([1, seq_len, hidden_size]));
        let transformed = transformed.reshape([seq_len, hidden_size]);

        let logits = self.lm_head.forward(transformed);

        let logits_3d = logits.reshape([1, seq_len, vocab_size]);
        let logits_3d = logits_3d + self.lm_bias.clone().reshape([1, 1, vocab_size]);

        Ok(logits_3d)
    }
}

pub struct VisionEncoderDecoder {
    encoder: VitEncoder,
    decoder: BertDecoder,
    config: VisionEncoderDecoderConfig,
    max_length: usize,
    decoder_start_token_id: u32,
    eos_token_id: u32,
    pad_token_id: u32,
}

impl VisionEncoderDecoder {
    pub fn from_config(
        config: &VisionEncoderDecoderConfig,
        weights: &crate::weights::WeightedTokens,
    ) -> anyhow::Result<Self> {
        let encoder = VitEncoder::from_weights(&config.encoder, weights)?;
        let decoder = BertDecoder::from_weights(&config.decoder, weights)?;

        Ok(Self {
            encoder,
            decoder,
            config: config.clone(),
            max_length: config.max_length,
            decoder_start_token_id: config.decoder_start_token_id,
            eos_token_id: config.eos_token_id,
            pad_token_id: config.pad_token_id,
        })
    }

    pub fn forward(&self, pixel_values: &Tensor<B, 4>) -> anyhow::Result<Vec<Vec<u32>>> {
        let encoder_hidden_states = self.encoder.forward(pixel_values)?;

        let mut results = Vec::new();
        let batch_size = pixel_values.dims()[0];

        for _b in 0..batch_size {
            let enc_states_b = encoder_hidden_states.clone();

            let mut input_ids = vec![self.decoder_start_token_id];

            for _step in 0..self.max_length {
                let logits = self.decoder.forward(&input_ids, &enc_states_b)?;

                let vocab_size = self.config.decoder.vocab_size;
                let cur_seq_len = input_ids.len();

                let flat_logits = logits.reshape([cur_seq_len * vocab_size]);

                let start_idx = (cur_seq_len - 1) * vocab_size;
                let end_idx = cur_seq_len * vocab_size;

                let logits_vec: Vec<f32> = flat_logits.to_data().to_vec().unwrap_or_default();
                let slice_logits = &logits_vec[start_idx..end_idx];

                let next_token = slice_logits
                    .iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(idx, _)| idx as u32)
                    .unwrap_or(0);

                if next_token == self.eos_token_id || next_token == self.pad_token_id {
                    if input_ids.len() > 1 {
                        break;
                    }
                }

                input_ids.push(next_token);

                if input_ids.len() > 30 {
                    break;
                }
            }

            results.push(input_ids);
        }

        Ok(results)
    }
}
