//! The prediction head.

use burn::module::Module;
use burn::nn::{Linear, LinearConfig};
use burn::tensor::{Device, Tensor};

/// Linear prediction head on top of a backbone embedding.
///
/// It outputs class logits for classification and a single value for regression.
#[derive(Module, Debug)]
pub struct Head {
    linear: Linear,
}

impl Head {
    /// Creates a head with fresh weights.
    pub fn new(embedding_dim: usize, output_dim: usize, device: &Device) -> Self {
        Self {
            linear: LinearConfig::new(embedding_dim, output_dim).init(device),
        }
    }

    /// Maps `[batch, embedding_dim]` embeddings to `[batch, output_dim]` outputs.
    pub fn forward(&self, embeddings: Tensor<2>) -> Tensor<2> {
        self.linear.forward(embeddings)
    }
}
