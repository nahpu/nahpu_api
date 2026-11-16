//! ResNet (He et al. 2016), torchvision layout.

use burn::module::Module;
use burn::nn::conv::Conv2d;
use burn::nn::pool::{MaxPool2d, MaxPool2dConfig};
use burn::nn::{BatchNorm, PaddingConfig2d, Relu};
use burn::tensor::{Device, Tensor};

use super::{batch_norm, conv, global_average_pool};
use crate::{DlError, Result};

/// Channel widths of the four stages, before bottleneck expansion.
const STAGE_WIDTHS: [usize; 4] = [64, 128, 256, 512];

/// Channel expansion of bottleneck blocks.
const BOTTLENECK_EXPANSION: usize = 4;

/// ResNet backbone without the classifier.
#[derive(Module, Debug)]
pub struct ResNet {
    conv1: Conv2d,
    bn1: BatchNorm,
    relu: Relu,
    maxpool: MaxPool2d,
    layer1: Vec<ResidualBlock>,
    layer2: Vec<ResidualBlock>,
    layer3: Vec<ResidualBlock>,
    layer4: Vec<ResidualBlock>,
}

impl ResNet {
    /// Creates a ResNet with fresh weights.
    ///
    /// # Errors
    ///
    /// Returns [`DlError::Config`] for depths other than 18, 34, 50, 101, and 152.
    pub fn new(depth: usize, device: &Device) -> Result<Self> {
        let (bottleneck, blocks) = match depth {
            18 => (false, [2, 2, 2, 2]),
            34 => (false, [3, 4, 6, 3]),
            50 => (true, [3, 4, 6, 3]),
            101 => (true, [3, 4, 23, 3]),
            152 => (true, [3, 8, 36, 3]),
            _ => {
                return Err(DlError::Config(format!(
                    "ResNet depth must be 18, 34, 50, 101, or 152, found {depth}"
                )));
            }
        };
        let mut in_channels = 64;
        let layers: Vec<Vec<ResidualBlock>> = STAGE_WIDTHS
            .iter()
            .zip(blocks)
            .enumerate()
            .map(|(stage, (&width, n_blocks))| {
                let stride = if stage == 0 { 1 } else { 2 };
                (0..n_blocks)
                    .map(|i| {
                        let block_stride = if i == 0 { stride } else { 1 };
                        let block = ResidualBlock::new(
                            in_channels,
                            width,
                            block_stride,
                            bottleneck,
                            device,
                        );
                        in_channels = block.out_channels;
                        block
                    })
                    .collect()
            })
            .collect();
        let [layer1, layer2, layer3, layer4]: [Vec<ResidualBlock>; 4] = layers
            .try_into()
            .expect("STAGE_WIDTHS has four stages, so there are four layers");
        Ok(Self {
            conv1: conv([3, 64], 7, 2, 1, device),
            bn1: batch_norm(64, device),
            relu: Relu::new(),
            maxpool: MaxPool2dConfig::new([3, 3])
                .with_strides([2, 2])
                .with_padding(PaddingConfig2d::Explicit(1, 1, 1, 1))
                .init(),
            layer1,
            layer2,
            layer3,
            layer4,
        })
    }

    /// Size of the pooled embedding.
    pub fn embedding_dim(&self) -> usize {
        self.layer4.last().map_or(0, |block| block.out_channels)
    }

    /// Maps `[batch, 3, h, w]` images to `[batch, embedding_dim]` embeddings.
    pub fn forward(&self, images: Tensor<4>) -> Tensor<2> {
        let x = self
            .relu
            .forward(self.bn1.forward(self.conv1.forward(images)));
        let x = self.maxpool.forward(x);
        let x = [&self.layer1, &self.layer2, &self.layer3, &self.layer4]
            .into_iter()
            .flatten()
            .fold(x, |x, block| block.forward(x));
        global_average_pool(x)
    }
}

/// Basic (two 3x3 convolutions) or bottleneck (1x1, 3x3, 1x1) residual block.
#[derive(Module, Debug)]
pub struct ResidualBlock {
    conv1: Conv2d,
    bn1: BatchNorm,
    conv2: Conv2d,
    bn2: BatchNorm,
    conv3: Option<Conv2d>,
    bn3: Option<BatchNorm>,
    downsample: Option<Downsample>,
    relu: Relu,
    out_channels: usize,
}

impl ResidualBlock {
    fn new(
        in_channels: usize,
        width: usize,
        stride: usize,
        bottleneck: bool,
        device: &Device,
    ) -> Self {
        let out_channels = if bottleneck {
            width * BOTTLENECK_EXPANSION
        } else {
            width
        };
        let downsample = (stride != 1 || in_channels != out_channels).then(|| Downsample {
            conv: conv([in_channels, out_channels], 1, stride, 1, device),
            bn: batch_norm(out_channels, device),
        });
        // torchvision ResNet v1.5 puts the stride on the 3x3 convolution.
        let (conv1, conv2, conv3) = if bottleneck {
            (
                conv([in_channels, width], 1, 1, 1, device),
                conv([width, width], 3, stride, 1, device),
                Some(conv([width, out_channels], 1, 1, 1, device)),
            )
        } else {
            (
                conv([in_channels, width], 3, stride, 1, device),
                conv([width, width], 3, 1, 1, device),
                None,
            )
        };
        Self {
            conv1,
            bn1: batch_norm(width, device),
            conv2,
            bn2: batch_norm(width, device),
            bn3: conv3.as_ref().map(|_| batch_norm(out_channels, device)),
            conv3,
            downsample,
            relu: Relu::new(),
            out_channels,
        }
    }

    fn forward(&self, input: Tensor<4>) -> Tensor<4> {
        let identity = match &self.downsample {
            Some(downsample) => downsample.forward(input.clone()),
            None => input.clone(),
        };
        let x = self
            .relu
            .forward(self.bn1.forward(self.conv1.forward(input)));
        let x = self.bn2.forward(self.conv2.forward(x));
        let x = match (&self.conv3, &self.bn3) {
            (Some(conv3), Some(bn3)) => bn3.forward(conv3.forward(self.relu.forward(x))),
            _ => x,
        };
        self.relu.forward(x + identity)
    }
}

/// 1x1 convolution that matches the shortcut to the block output.
#[derive(Module, Debug)]
pub struct Downsample {
    conv: Conv2d,
    bn: BatchNorm,
}

impl Downsample {
    fn forward(&self, input: Tensor<4>) -> Tensor<4> {
        self.bn.forward(self.conv.forward(input))
    }
}

#[cfg(all(test, feature = "flex"))]
mod tests {
    use super::*;

    #[test]
    fn test_resnet_shapes() {
        let device = Device::flex();
        for (depth, dim) in [(18, 512), (50, 2048)] {
            let resnet = ResNet::new(depth, &device).unwrap();
            assert_eq!(resnet.embedding_dim(), dim);
            let out = resnet
                .valid()
                .forward(Tensor::zeros([2, 3, 32, 32], &device));
            assert_eq!(out.dims(), [2, dim]);
        }
    }

    #[test]
    fn test_resnet_rejects_unknown_depth() {
        assert!(ResNet::new(20, &Device::flex()).is_err());
    }

    #[test]
    fn test_resnet18_parameter_count_matches_torchvision() {
        // torchvision resnet18 has 11,689,512 parameters, of which 513,000 are in `fc`.
        // Burn also counts the batch norm running mean and variance: 2 x 4,800 channels.
        let resnet = ResNet::new(18, &Device::flex()).unwrap();
        assert_eq!(resnet.num_params(), 11_689_512 - 513_000 + 2 * 4_800);
    }
}
