use burn::tensor::activation::silu;
use burn::tensor::ops::ConvOptions;
use burn::tensor::Tensor;

use crate::{device, weights::WeightedTokens, B};

struct ConvBnSilu {
    conv_weight: Tensor<B, 4>,
    conv_bias: Option<Tensor<B, 1>>,
    bn_weight: Tensor<B, 1>,
    bn_bias: Tensor<B, 1>,
    bn_running_mean: Tensor<B, 1>,
    bn_running_var: Tensor<B, 1>,
    stride: usize,
    padding: usize,
}

impl ConvBnSilu {
    fn load(
        weights: &WeightedTokens,
        prefix: &str,
        in_ch: usize,
        out_ch: usize,
        kernel: usize,
        stride: usize,
        padding: usize,
    ) -> anyhow::Result<Self> {
        let dev = device();

        let conv_weight =
            if let Ok(data) = weights.get_float_tensor(&format!("{}.conv.weight", prefix)) {
                let td = burn::tensor::TensorData::new(data, vec![out_ch, in_ch, kernel, kernel]);
                Tensor::<B, 4>::from_data(td, &dev)
            } else {
                Tensor::<B, 4>::zeros([out_ch, in_ch, kernel, kernel], &dev)
            };

        let bn_weight = if let Ok(data) = weights.get_float_tensor(&format!("{}.bn.weight", prefix))
        {
            let td = burn::tensor::TensorData::new(data, vec![out_ch]);
            Tensor::<B, 1>::from_data(td, &dev)
        } else {
            Tensor::<B, 1>::ones([out_ch], &dev)
        };

        let bn_bias = if let Ok(data) = weights.get_float_tensor(&format!("{}.bn.bias", prefix)) {
            let td = burn::tensor::TensorData::new(data, vec![out_ch]);
            Tensor::<B, 1>::from_data(td, &dev)
        } else {
            Tensor::<B, 1>::zeros([out_ch], &dev)
        };

        let bn_running_mean =
            if let Ok(data) = weights.get_float_tensor(&format!("{}.bn.running_mean", prefix)) {
                let td = burn::tensor::TensorData::new(data, vec![out_ch]);
                Tensor::<B, 1>::from_data(td, &dev)
            } else {
                Tensor::<B, 1>::zeros([out_ch], &dev)
            };

        let bn_running_var =
            if let Ok(data) = weights.get_float_tensor(&format!("{}.bn.running_var", prefix)) {
                let td = burn::tensor::TensorData::new(data, vec![out_ch]);
                Tensor::<B, 1>::from_data(td, &dev)
            } else {
                Tensor::<B, 1>::ones([out_ch], &dev)
            };

        Ok(Self {
            conv_weight,
            conv_bias: None,
            bn_weight,
            bn_bias,
            bn_running_mean,
            bn_running_var,
            stride,
            padding,
        })
    }

    fn forward(&self, x: Tensor<B, 4>) -> Tensor<B, 4> {
        let options = ConvOptions::new(
            [self.stride, self.stride],
            [self.padding, self.padding],
            [1, 1],
            1,
        );

        let x = burn::tensor::module::conv2d(
            x,
            self.conv_weight.clone(),
            self.conv_bias.clone(),
            options,
        );

        let [batch, channels, h, w] = x.dims();

        // BatchNorm: x_norm = (x - mean) / sqrt(var + eps) * weight + bias
        let x_flat = x.reshape([batch, channels, h * w]);

        let eps = 1e-3;

        let mean = self.bn_running_mean.clone().reshape([1, channels, 1]);
        let var = self.bn_running_var.clone().reshape([1, channels, 1]);
        let weight = self.bn_weight.clone().reshape([1, channels, 1]);
        let bias = self.bn_bias.clone().reshape([1, channels, 1]);

        let x_norm = (x_flat - mean) / (var + eps).sqrt();
        let x_norm = x_norm * weight + bias;

        let x_norm = x_norm.reshape([batch, channels, h, w]);
        silu(x_norm)
    }
}

struct Bottleneck {
    cv1: ConvBnSilu,
    cv2: ConvBnSilu,
    residual: bool,
}

impl Bottleneck {
    fn load(
        weights: &WeightedTokens,
        prefix: &str,
        c1: usize,
        c2: usize,
        shortcut: bool,
        expansion: f32,
    ) -> anyhow::Result<Self> {
        let hidden = (c2 as f32 * expansion) as usize;
        let cv1 = ConvBnSilu::load(weights, &format!("{}.cv1", prefix), c1, hidden, 1, 1, 0)?;
        let cv2 = ConvBnSilu::load(weights, &format!("{}.cv2", prefix), hidden, c2, 3, 1, 1)?;

        Ok(Self {
            cv1,
            cv2,
            residual: shortcut && c1 == c2,
        })
    }

    fn forward(&self, x: Tensor<B, 4>) -> Tensor<B, 4> {
        let y = self.cv2.forward(self.cv1.forward(x.clone()));
        if self.residual {
            x + y
        } else {
            y
        }
    }
}

struct C3 {
    cv1: ConvBnSilu,
    cv2: ConvBnSilu,
    cv3: ConvBnSilu,
    m: Vec<Bottleneck>,
}

impl C3 {
    fn load(
        weights: &WeightedTokens,
        prefix: &str,
        c1: usize,
        c2: usize,
        n: usize,
        shortcut: bool,
        expansion: f32,
    ) -> anyhow::Result<Self> {
        let hidden = (c2 as f32 * expansion) as usize;
        let cv1 = ConvBnSilu::load(weights, &format!("{}.cv1", prefix), c1, hidden, 1, 1, 0)?;
        let cv2 = ConvBnSilu::load(weights, &format!("{}.cv2", prefix), c1, hidden, 1, 1, 0)?;
        let cv3 = ConvBnSilu::load(weights, &format!("{}.cv3", prefix), 2 * hidden, c2, 1, 1, 0)?;

        let mut m = Vec::new();
        for i in 0..n {
            let b = Bottleneck::load(
                weights,
                &format!("{}.m.{}", prefix, i),
                hidden,
                hidden,
                shortcut,
                1.0, // Bottleneck uses expansion=1.0, not C3's expansion
            )?;
            m.push(b);
        }

        Ok(Self { cv1, cv2, cv3, m })
    }

    fn forward(&self, x: Tensor<B, 4>) -> Tensor<B, 4> {
        let y1 = self.cv1.forward(x.clone());
        let y2 = self.cv2.forward(x);

        let mut y = y1;
        for b in &self.m {
            y = b.forward(y);
        }

        let y = Tensor::cat(vec![y, y2], 1);
        self.cv3.forward(y)
    }
}

struct Sppf {
    cv1: ConvBnSilu,
    cv2: ConvBnSilu,
    kernel: usize,
}

impl Sppf {
    fn load(
        weights: &WeightedTokens,
        prefix: &str,
        c1: usize,
        c2: usize,
        kernel: usize,
    ) -> anyhow::Result<Self> {
        let hidden = c1 / 2;
        let cv1 = ConvBnSilu::load(weights, &format!("{}.cv1", prefix), c1, hidden, 1, 1, 0)?;
        let cv2 = ConvBnSilu::load(weights, &format!("{}.cv2", prefix), hidden * 4, c2, 1, 1, 0)?;

        Ok(Self { cv1, cv2, kernel })
    }

    fn forward(&self, x: Tensor<B, 4>) -> Tensor<B, 4> {
        let x = self.cv1.forward(x);
        let y1 = max_pool2d(x.clone(), self.kernel, 1, self.kernel / 2);
        let y2 = max_pool2d(y1.clone(), self.kernel, 1, self.kernel / 2);
        let y3 = max_pool2d(y2.clone(), self.kernel, 1, self.kernel / 2);

        let y = Tensor::cat(vec![x, y1, y2, y3], 1);
        self.cv2.forward(y)
    }
}

fn max_pool2d(x: Tensor<B, 4>, kernel: usize, stride: usize, padding: usize) -> Tensor<B, 4> {
    let batch = x.dims()[0];
    let channels = x.dims()[1];
    let h = x.dims()[2];
    let w = x.dims()[3];

    let out_h = (h + 2 * padding - kernel) / stride + 1;
    let out_w = (w + 2 * padding - kernel) / stride + 1;

    let x_padded = {
        let mut padded = Tensor::<B, 4>::zeros(
            [batch, channels, h + 2 * padding, w + 2 * padding],
            &device(),
        );
        padded = padded.slice_assign(
            [
                0..batch,
                0..channels,
                padding..h + padding,
                padding..w + padding,
            ],
            x,
        );
        padded
    };

    let mut output = Tensor::<B, 4>::zeros([batch, channels, out_h, out_w], &device());

    for i in 0..out_h {
        for j in 0..out_w {
            let h_start = i * stride;
            let w_start = j * stride;
            let patch = x_padded.clone().slice([
                0..batch,
                0..channels,
                h_start..h_start + kernel,
                w_start..w_start + kernel,
            ]);
            let max_val = patch
                .reshape([batch, channels, kernel * kernel])
                .max_dim(2)
                .reshape([batch, channels, 1, 1]);
            output = output.slice_assign([0..batch, 0..channels, i..i + 1, j..j + 1], max_val);
        }
    }

    output
}

struct Upsample {
    scale: usize,
}

impl Upsample {
    fn new(scale: usize) -> Self {
        Self { scale }
    }

    fn forward(&self, x: Tensor<B, 4>) -> Tensor<B, 4> {
        let [batch, channels, h, w] = x.dims();
        let new_h = h * self.scale;
        let new_w = w * self.scale;

        // Simple nearest neighbor upsampling
        let x_reshaped = x.reshape([batch, channels, h, 1, w, 1]);
        let x_upsampled = x_reshaped.repeat(&[1, 1, 1, self.scale, 1, self.scale]);
        x_upsampled.reshape([batch, channels, new_h, new_w])
    }
}

pub struct YoloV5 {
    model0: ConvBnSilu,
    model1: ConvBnSilu,
    model2: C3,
    model3: ConvBnSilu,
    model4: C3,
    model5: ConvBnSilu,
    model6: C3,
    model7: ConvBnSilu,
    model8: C3,
    model9: Sppf,
    model10: ConvBnSilu,
    model13: C3,
    model14: ConvBnSilu,
    model17: C3,
    model18: ConvBnSilu,
    model20: C3,
    model21: ConvBnSilu,
    model23: C3,
    model24: DetectHead,
}

struct DetectHead {
    conv0_weight: Tensor<B, 4>,
    conv0_bias: Tensor<B, 1>,
    conv1_weight: Tensor<B, 4>,
    conv1_bias: Tensor<B, 1>,
    conv2_weight: Tensor<B, 4>,
    conv2_bias: Tensor<B, 1>,
    anchors: Tensor<B, 3>,
    num_outputs: usize,
    num_anchors: usize,
    strides: [f32; 3],
}

impl DetectHead {
    fn load(
        weights: &WeightedTokens,
        prefix: &str,
        ch: &[usize],
        num_classes: usize,
        num_anchors: usize,
    ) -> anyhow::Result<Self> {
        let dev = device();
        let num_outputs = num_classes + 5;

        let conv0_weight = {
            let data = weights.get_float_tensor(&format!("{}.m.0.weight", prefix))?;
            let td =
                burn::tensor::TensorData::new(data, vec![num_outputs * num_anchors, ch[0], 1, 1]);
            Tensor::<B, 4>::from_data(td, &dev)
        };
        let conv0_bias = {
            let data = weights.get_float_tensor(&format!("{}.m.0.bias", prefix))?;
            let td = burn::tensor::TensorData::new(data, vec![num_outputs * num_anchors]);
            Tensor::<B, 1>::from_data(td, &dev)
        };

        let conv1_weight = {
            let data = weights.get_float_tensor(&format!("{}.m.1.weight", prefix))?;
            Tensor::<B, 4>::from_data(
                burn::tensor::TensorData::new(data, vec![num_outputs * num_anchors, ch[1], 1, 1]),
                &dev,
            )
        };
        let conv1_bias = {
            let data = weights.get_float_tensor(&format!("{}.m.1.bias", prefix))?;
            Tensor::<B, 1>::from_data(
                burn::tensor::TensorData::new(data, vec![num_outputs * num_anchors]),
                &dev,
            )
        };

        let conv2_weight = {
            let data = weights.get_float_tensor(&format!("{}.m.2.weight", prefix))?;
            Tensor::<B, 4>::from_data(
                burn::tensor::TensorData::new(data, vec![num_outputs * num_anchors, ch[2], 1, 1]),
                &dev,
            )
        };
        let conv2_bias = {
            let data = weights.get_float_tensor(&format!("{}.m.2.bias", prefix))?;
            Tensor::<B, 1>::from_data(
                burn::tensor::TensorData::new(data, vec![num_outputs * num_anchors]),
                &dev,
            )
        };

        let anchors = {
            let data = weights.get_float_tensor(&format!("{}.anchors", prefix))?;
            let td = burn::tensor::TensorData::new(data, vec![num_anchors, 3, 2]);
            Tensor::<B, 3>::from_data(td, &dev)
        };

        Ok(Self {
            conv0_weight,
            conv0_bias,
            conv1_weight,
            conv1_bias,
            conv2_weight,
            conv2_bias,
            anchors,
            num_outputs,
            num_anchors,
            strides: [8.0, 16.0, 32.0],
        })
    }

    fn forward(&self, inputs: &[Tensor<B, 4>; 3]) -> anyhow::Result<Tensor<B, 3>> {
        let mut outputs = Vec::with_capacity(3);

        let weights = [
            self.conv0_weight.clone(),
            self.conv1_weight.clone(),
            self.conv2_weight.clone(),
        ];
        let biases = [
            self.conv0_bias.clone(),
            self.conv1_bias.clone(),
            self.conv2_bias.clone(),
        ];

        for (idx, (weight, bias)) in weights.iter().zip(biases.iter()).enumerate() {
            let xs = &inputs[idx];
            let options = ConvOptions::new([1, 1], [0, 0], [1, 1], 1);
            let xs = burn::tensor::module::conv2d(
                xs.clone(),
                weight.clone(),
                Some(bias.clone()),
                options,
            );

            let [b, _, h, w] = xs.dims();
            let xs = xs
                .reshape([b, self.num_anchors, self.num_outputs, h, w])
                .permute([0, 1, 3, 4, 2]);

            let y = burn::tensor::activation::sigmoid(xs.clone());

            let grid_x = Tensor::<B, 1, burn::tensor::Int>::arange(0..w as i64, &device())
                .float()
                .reshape([1, 1, 1, w])
                .repeat(&[1, 1, h, 1]);
            let grid_y = Tensor::<B, 1, burn::tensor::Int>::arange(0..h as i64, &device())
                .float()
                .reshape([1, 1, h, 1])
                .repeat(&[1, 1, 1, w]);
            let grid = Tensor::stack(vec![grid_x, grid_y], 4);

            let anchor = self
                .anchors
                .clone()
                .slice([idx..idx + 1, 0..self.num_anchors, 0..2]);
            // anchor shape: [1, 3, 2] -> reshape to [1, 3, 1, 1, 2]
            let anchor_grid = anchor
                .reshape([1, self.num_anchors, 1, 1, 2])
                .repeat(&[1, 1, h, w, 1])
                * self.strides[idx];

            let xy = y
                .clone()
                .slice([0..b, 0..self.num_anchors, 0..h, 0..w, 0..2]);
            let xy = (xy * 2.0 - 0.5 + grid) * self.strides[idx];

            let wh = y
                .clone()
                .slice([0..b, 0..self.num_anchors, 0..h, 0..w, 2..4]);
            let wh = ((wh * 2.0).powi_scalar(2.0)) * anchor_grid;

            let rest =
                y.clone()
                    .slice([0..b, 0..self.num_anchors, 0..h, 0..w, 4..self.num_outputs]);

            let y = Tensor::cat(vec![xy, wh, rest], 4);

            outputs.push(y.reshape([b, self.num_anchors * h * w, self.num_outputs]));
        }

        let pred = Tensor::cat(outputs, 1);
        Ok(pred)
    }
}

impl YoloV5 {
    pub fn load(
        weights: &WeightedTokens,
        num_classes: usize,
        num_anchors: usize,
    ) -> anyhow::Result<Self> {
        let model0 = ConvBnSilu::load(weights, "model.0", 3, 32, 6, 2, 2)?;
        let model1 = ConvBnSilu::load(weights, "model.1", 32, 64, 3, 2, 1)?;
        let model2 = C3::load(weights, "model.2", 64, 64, 1, true, 0.5)?;
        let model3 = ConvBnSilu::load(weights, "model.3", 64, 128, 3, 2, 1)?;
        let model4 = C3::load(weights, "model.4", 128, 128, 2, true, 0.5)?;
        let model5 = ConvBnSilu::load(weights, "model.5", 128, 256, 3, 2, 1)?;
        let model6 = C3::load(weights, "model.6", 256, 256, 3, true, 0.5)?;
        let model7 = ConvBnSilu::load(weights, "model.7", 256, 512, 3, 2, 1)?;
        let model8 = C3::load(weights, "model.8", 512, 512, 1, true, 0.5)?;
        let model9 = Sppf::load(weights, "model.9", 512, 512, 5)?;
        let model10 = ConvBnSilu::load(weights, "model.10", 512, 256, 1, 1, 0)?;
        let model13 = C3::load(weights, "model.13", 512, 256, 1, false, 0.5)?;
        let model14 = ConvBnSilu::load(weights, "model.14", 256, 128, 1, 1, 0)?;
        let model17 = C3::load(weights, "model.17", 256, 128, 1, false, 0.5)?;
        let model18 = ConvBnSilu::load(weights, "model.18", 128, 128, 3, 2, 1)?;
        let model20 = C3::load(weights, "model.20", 256, 256, 1, false, 0.5)?;
        let model21 = ConvBnSilu::load(weights, "model.21", 256, 256, 3, 2, 1)?;
        let model23 = C3::load(weights, "model.23", 512, 512, 1, false, 0.5)?;
        let model24 = DetectHead::load(
            weights,
            "model.24",
            &[128, 256, 512],
            num_classes,
            num_anchors,
        )?;

        Ok(Self {
            model0,
            model1,
            model2,
            model3,
            model4,
            model5,
            model6,
            model7,
            model8,
            model9,
            model10,
            model13,
            model14,
            model17,
            model18,
            model20,
            model21,
            model23,
            model24,
        })
    }

    pub fn forward(&self, x: Tensor<B, 4>) -> anyhow::Result<(Tensor<B, 3>, Vec<Tensor<B, 4>>)> {
        // Backbone
        let x = self.model0.forward(x);
        let x = self.model1.forward(x);
        let x = self.model2.forward(x);
        let x = self.model3.forward(x);
        let p3 = self.model4.forward(x);
        let x = self.model5.forward(p3.clone());
        let p4 = self.model6.forward(x);
        let x = self.model7.forward(p4.clone());
        let p5 = self.model8.forward(x);
        let p5 = self.model9.forward(p5);

        // Neck (PANet)
        let x10 = self.model10.forward(p5.clone());
        let x11 = Upsample::new(2).forward(x10.clone());
        let x12 = Tensor::cat(vec![x11, p4.clone()], 1);
        let x13 = self.model13.forward(x12);
        let x14 = self.model14.forward(x13.clone());
        let x15 = Upsample::new(2).forward(x14.clone());
        let x16 = Tensor::cat(vec![x15, p3.clone()], 1);
        let x17 = self.model17.forward(x16);
        let x18 = self.model18.forward(x17.clone());
        let x19 = Tensor::cat(vec![x18, x14.clone()], 1);
        let x20 = self.model20.forward(x19);
        let x21 = self.model21.forward(x20.clone());
        let x22 = Tensor::cat(vec![x21, x10.clone()], 1);
        let x23 = self.model23.forward(x22);

        // Detection head
        let predictions = self.model24.forward(&[x17.clone(), x20.clone(), x23])?;

        // Features for UNet/DbNet
        let features = vec![p5, p4, p3, x17, x20];

        Ok((predictions, features))
    }
}
