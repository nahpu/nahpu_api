//! Tabular backbones.

use burn::module::Module;
use burn::nn::{Dropout, DropoutConfig, Linear, LinearConfig, Relu};
use burn::tensor::{Device, Tensor};

use crate::{DlError, Result};

/// Multilayer perceptron backbone for tabular data.
///
/// Each hidden layer is linear, ReLU, then dropout. The last hidden layer is the embedding.
#[derive(Module, Debug)]
pub struct MlpBackbone {
    layers: Vec<Linear>,
    activation: Relu,
    dropout: Dropout,
}

impl MlpBackbone {
    /// Creates the backbone with fresh weights.
    ///
    /// # Errors
    ///
    /// Returns [`DlError::Config`] when a dimension is zero or dropout is outside `[0, 1)`.
    pub fn new(input_dim: usize, hidden: &[usize], dropout: f64, device: &Device) -> Result<Self> {
        if input_dim == 0 || hidden.is_empty() || hidden.contains(&0) {
            return Err(DlError::Config(
                "an MLP needs at least one input feature and one non-empty hidden layer"
                    .to_string(),
            ));
        }
        if !(0.0..1.0).contains(&dropout) {
            return Err(DlError::Config(format!(
                "dropout must be in [0, 1), found {dropout}"
            )));
        }
        let dims: Vec<usize> = std::iter::once(input_dim)
            .chain(hidden.iter().copied())
            .collect();
        let layers = dims
            .windows(2)
            .map(|pair| LinearConfig::new(pair[0], pair[1]).init(device))
            .collect();
        Ok(Self {
            layers,
            activation: Relu::new(),
            dropout: DropoutConfig::new(dropout).init(),
        })
    }

    /// Size of the embedding the backbone returns.
    pub fn embedding_dim(&self) -> usize {
        self.layers
            .last()
            .map(|layer| layer.weight.dims()[1])
            .unwrap_or_default()
    }

    /// Maps `[batch, input_dim]` features to `[batch, embedding_dim]` embeddings.
    pub fn forward(&self, features: Tensor<2>) -> Tensor<2> {
        self.layers.iter().fold(features, |x, layer| {
            self.dropout
                .forward(self.activation.forward(layer.forward(x)))
        })
    }
}

#[cfg(all(test, feature = "flex"))]
mod tests {
    use super::*;

    #[test]
    fn test_mlp_output_shape() {
        let device = Device::flex();
        let mlp = MlpBackbone::new(3, &[8, 4], 0.0, &device).unwrap();
        assert_eq!(mlp.embedding_dim(), 4);
        let out = mlp.forward(Tensor::zeros([5, 3], &device));
        assert_eq!(out.dims(), [5, 4]);
    }

    #[test]
    fn test_mlp_rejects_bad_config() {
        let device = Device::flex();
        assert!(MlpBackbone::new(0, &[4], 0.0, &device).is_err());
        assert!(MlpBackbone::new(3, &[], 0.0, &device).is_err());
        assert!(MlpBackbone::new(3, &[4], 1.0, &device).is_err());
    }
}
