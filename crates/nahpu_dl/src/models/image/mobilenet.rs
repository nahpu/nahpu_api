//! MobileNetV2 (Sandler et al. 2018), timm layout.

use burn::module::Module;
use burn::nn::BatchNorm;
use burn::nn::conv::Conv2d;
use burn::tensor::{Device, Tensor};

use super::{batch_norm, conv, global_average_pool};

/// Channels of the stem convolution.
const STEM_CHANNELS: usize = 32;

/// Channels of the final 1x1 convolution, which is the embedding size.
pub const HEAD_CHANNELS: usize = 1280;

/// Stages as (expansion, output channels, blocks, first stride), from the paper's Table 2.
const STAGES: [(usize, usize, usize, usize); 7] = [
    (1, 16, 1, 1),
    (6, 24, 2, 2),
    (6, 32, 3, 2),
    (6, 64, 4, 2),
    (6, 96, 3, 1),
    (6, 160, 3, 2),
    (6, 320, 1, 1),
];

/// MobileNetV2 backbone (width multiplier 1.0) without the classifier.
#[derive(Module, Debug)]
pub struct MobileNetV2 {
    conv_stem: Conv2d,
    bn1: BatchNorm,
    blocks: Vec<Vec<MobileBlock>>,
    conv_head: Conv2d,
    bn2: BatchNorm,
}

impl MobileNetV2 {
    /// Creates the network with fresh weights.
    pub fn new(device: &Device) -> Self {
        let mut in_channels = STEM_CHANNELS;
        let blocks = STAGES
            .iter()
            .map(|&(expansion, out_channels, n_blocks, stride)| {
                (0..n_blocks)
                    .map(|i| {
                        let stride = if i == 0 { stride } else { 1 };
                        let block =
                            MobileBlock::new(in_channels, out_channels, expansion, stride, device);
                        in_channels = out_channels;
                        block
                    })
                    .collect()
            })
            .collect();
        Self {
            conv_stem: conv([3, STEM_CHANNELS], 3, 2, 1, device),
            bn1: batch_norm(STEM_CHANNELS, device),
            blocks,
            conv_head: conv([in_channels, HEAD_CHANNELS], 1, 1, 1, device),
            bn2: batch_norm(HEAD_CHANNELS, device),
        }
    }

    /// Size of the pooled embedding.
    pub fn embedding_dim(&self) -> usize {
        HEAD_CHANNELS
    }

    /// Maps `[batch, 3, h, w]` images to `[batch, 1280]` embeddings.
    pub fn forward(&self, images: Tensor<4>) -> Tensor<2> {
        let x = relu6(self.bn1.forward(self.conv_stem.forward(images)));
        let x = self
            .blocks
            .iter()
            .flatten()
            .fold(x, |x, block| block.forward(x));
        let x = relu6(self.bn2.forward(self.conv_head.forward(x)));
        global_average_pool(x)
    }
}

/// ReLU capped at 6, as in the paper. Burn does not export its `Relu6` module.
fn relu6(x: Tensor<4>) -> Tensor<4> {
    x.clamp(0.0, 6.0)
}

/// The two block types of MobileNetV2.
// Burn modules cannot be boxed, and blocks are built once per model, so the size gap
// between variants costs nothing that matters.
#[allow(clippy::large_enum_variant)]
#[derive(Module, Debug)]
pub enum MobileBlock {
    /// First block: depthwise then pointwise convolution, no expansion.
    DepthwiseSeparable(DepthwiseSeparable),
    /// Expand, depthwise, project, with a residual connection when shapes match.
    InvertedResidual(InvertedResidual),
}

impl MobileBlock {
    fn new(
        in_channels: usize,
        out_channels: usize,
        expansion: usize,
        stride: usize,
        device: &Device,
    ) -> Self {
        let residual = stride == 1 && in_channels == out_channels;
        if expansion == 1 {
            return Self::DepthwiseSeparable(DepthwiseSeparable {
                conv_dw: conv([in_channels, in_channels], 3, stride, in_channels, device),
                bn1: batch_norm(in_channels, device),
                conv_pw: conv([in_channels, out_channels], 1, 1, 1, device),
                bn2: batch_norm(out_channels, device),
                residual,
            });
        }
        let mid = in_channels * expansion;
        Self::InvertedResidual(InvertedResidual {
            conv_pw: conv([in_channels, mid], 1, 1, 1, device),
            bn1: batch_norm(mid, device),
            conv_dw: conv([mid, mid], 3, stride, mid, device),
            bn2: batch_norm(mid, device),
            conv_pwl: conv([mid, out_channels], 1, 1, 1, device),
            bn3: batch_norm(out_channels, device),
            residual,
        })
    }

    fn forward(&self, input: Tensor<4>) -> Tensor<4> {
        match self {
            Self::DepthwiseSeparable(block) => block.forward(input),
            Self::InvertedResidual(block) => block.forward(input),
        }
    }
}

/// Depthwise separable block.
#[derive(Module, Debug)]
pub struct DepthwiseSeparable {
    conv_dw: Conv2d,
    bn1: BatchNorm,
    conv_pw: Conv2d,
    bn2: BatchNorm,
    residual: bool,
}

impl DepthwiseSeparable {
    fn forward(&self, input: Tensor<4>) -> Tensor<4> {
        let x = relu6(self.bn1.forward(self.conv_dw.forward(input.clone())));
        let x = self.bn2.forward(self.conv_pw.forward(x));
        if self.residual { x + input } else { x }
    }
}

/// Inverted residual block with a linear bottleneck.
#[derive(Module, Debug)]
pub struct InvertedResidual {
    conv_pw: Conv2d,
    bn1: BatchNorm,
    conv_dw: Conv2d,
    bn2: BatchNorm,
    conv_pwl: Conv2d,
    bn3: BatchNorm,
    residual: bool,
}

impl InvertedResidual {
    fn forward(&self, input: Tensor<4>) -> Tensor<4> {
        let x = relu6(self.bn1.forward(self.conv_pw.forward(input.clone())));
        let x = relu6(self.bn2.forward(self.conv_dw.forward(x)));
        let x = self.bn3.forward(self.conv_pwl.forward(x));
        if self.residual { x + input } else { x }
    }
}

#[cfg(all(test, feature = "flex"))]
mod tests {
    use super::*;

    #[test]
    fn test_mobilenet_shape() {
        let device = Device::flex();
        let net = MobileNetV2::new(&device);
        let out = net.valid().forward(Tensor::zeros([2, 3, 32, 32], &device));
        assert_eq!(out.dims(), [2, HEAD_CHANNELS]);
    }

    #[test]
    fn test_mobilenet_parameter_count_matches_torchvision() {
        // torchvision mobilenet_v2 has 3,504,872 parameters, of which 1,281,000 are in the
        // classifier. Burn also counts the batch norm running statistics: 2 x 17,056 channels.
        let net = MobileNetV2::new(&Device::flex());
        assert_eq!(net.num_params(), 3_504_872 - 1_281_000 + 2 * 17_056);
    }
}
