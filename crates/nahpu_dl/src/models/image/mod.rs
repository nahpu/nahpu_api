//! Image backbones.
//!
//! Field names follow torchvision (ResNet) and timm (MobileNetV2) so pretrained weights
//! load with little or no key remapping.

use burn::nn::conv::{Conv2d, Conv2dConfig};
use burn::nn::{BatchNorm, BatchNormConfig, PaddingConfig2d};
use burn::tensor::{Device, Tensor};

pub mod mobilenet;
pub mod resnet;

pub use mobilenet::MobileNetV2;
pub use resnet::ResNet;

/// Bias-free convolution with symmetric `(kernel - 1) / 2` padding, as in torchvision.
pub(crate) fn conv(
    channels: [usize; 2],
    kernel: usize,
    stride: usize,
    groups: usize,
    device: &Device,
) -> Conv2d {
    let pad = (kernel - 1) / 2;
    Conv2dConfig::new(channels, [kernel, kernel])
        .with_stride([stride, stride])
        .with_padding(PaddingConfig2d::Explicit(pad, pad, pad, pad))
        .with_groups(groups)
        .with_bias(false)
        .init(device)
}

pub(crate) fn batch_norm(features: usize, device: &Device) -> BatchNorm {
    BatchNormConfig::new(features).init(device)
}

/// Global average pooling: `[batch, channels, h, w]` to `[batch, channels]`.
pub(crate) fn global_average_pool(features: Tensor<4>) -> Tensor<2> {
    features.mean_dims(&[2, 3]).flatten(1, 3)
}
