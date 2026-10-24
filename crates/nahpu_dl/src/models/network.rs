//! The network: a backbone and an optional head.

use burn::module::Module;
use burn::tensor::{Device, Tensor};

use crate::data::BatchInput;
use crate::manifest::{Architecture, ModelManifest};
use crate::{DlError, Result};

use super::heads::Head;
use super::tabular::MlpBackbone;

/// A backbone that turns inputs into embeddings, plus an optional prediction head.
#[derive(Module, Debug)]
pub struct Network {
    /// Feature extractor.
    pub backbone: Backbone,
    /// Prediction head. `None` for embedding-only models.
    pub head: Option<Head>,
}

impl Network {
    /// Builds a network with fresh weights for a manifest.
    ///
    /// # Errors
    ///
    /// Returns [`DlError::Config`] when the architecture and manifest disagree.
    pub fn init(manifest: &ModelManifest, device: &Device) -> Result<Self> {
        if manifest.architecture.modality() != manifest.modality {
            return Err(DlError::Config(format!(
                "architecture expects {:?} input but the manifest says {:?}",
                manifest.architecture.modality(),
                manifest.modality
            )));
        }
        let backbone = Backbone::init(&manifest.architecture, device)?;
        let head = manifest
            .output_dim()
            .map(|output_dim| Head::new(backbone.embedding_dim(), output_dim, device));
        Ok(Self { backbone, head })
    }

    /// Runs the backbone and the head. Returns `None` when there is no head.
    pub fn forward(&self, input: BatchInput) -> Option<Tensor<2>> {
        let head = self.head.as_ref()?;
        Some(head.forward(self.backbone.forward_features(input)))
    }
}

/// Feature extractors, one variant per architecture family.
#[derive(Module, Debug)]
pub enum Backbone {
    /// Multilayer perceptron for tabular data.
    Mlp(MlpBackbone),
}

impl Backbone {
    /// Builds a backbone with fresh weights.
    ///
    /// # Errors
    ///
    /// Returns [`DlError::Config`] when the hyperparameters are invalid.
    pub fn init(architecture: &Architecture, device: &Device) -> Result<Self> {
        match architecture {
            Architecture::Mlp {
                input_dim,
                hidden,
                dropout,
            } => MlpBackbone::new(*input_dim, hidden, *dropout, device).map(Self::Mlp),
        }
    }

    /// Size of the embedding [`Backbone::forward_features`] returns.
    pub fn embedding_dim(&self) -> usize {
        match self {
            Self::Mlp(mlp) => mlp.embedding_dim(),
        }
    }

    /// Computes one embedding per input row: `[batch, embedding_dim]`.
    pub fn forward_features(&self, input: BatchInput) -> Tensor<2> {
        match (self, input) {
            (Self::Mlp(mlp), BatchInput::Tabular(features)) => mlp.forward(features),
        }
    }
}
