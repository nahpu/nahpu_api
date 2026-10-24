//! The network: a backbone and an optional head.

use burn::module::Module;
use burn::tensor::{Device, Tensor};

use crate::data::BatchInput;
use crate::manifest::{Architecture, ModelManifest};
use crate::{DlError, Result};

use super::heads::Head;
use super::image::{MobileNetV2, ResNet};
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

    /// Runs the backbone and the head.
    ///
    /// # Errors
    ///
    /// Returns an error when there is no head or the input does not fit the backbone.
    pub fn forward(&self, input: BatchInput) -> Result<Tensor<2>> {
        let head = self.head.as_ref().ok_or_else(|| {
            DlError::Unsupported("the network has no prediction head".to_string())
        })?;
        Ok(head.forward(self.backbone.forward_features(input)?))
    }
}

/// Feature extractors, one variant per architecture family.
// Burn modules cannot be boxed, and there is one backbone per model, so the size gap
// between variants costs nothing that matters.
#[allow(clippy::large_enum_variant)]
#[derive(Module, Debug)]
pub enum Backbone {
    /// Multilayer perceptron for tabular data.
    Mlp(MlpBackbone),
    /// ResNet for images.
    ResNet(ResNet),
    /// MobileNetV2 for images.
    MobileNetV2(MobileNetV2),
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
            Architecture::ResNet { depth } => ResNet::new(*depth, device).map(Self::ResNet),
            Architecture::MobileNetV2 => Ok(Self::MobileNetV2(MobileNetV2::new(device))),
        }
    }

    /// Size of the embedding [`Backbone::forward_features`] returns.
    pub fn embedding_dim(&self) -> usize {
        match self {
            Self::Mlp(mlp) => mlp.embedding_dim(),
            Self::ResNet(resnet) => resnet.embedding_dim(),
            Self::MobileNetV2(mobilenet) => mobilenet.embedding_dim(),
        }
    }

    /// Computes one embedding per input: `[batch, embedding_dim]`.
    ///
    /// # Errors
    ///
    /// Returns [`DlError::Data`] when the input modality does not fit the backbone.
    pub fn forward_features(&self, input: BatchInput) -> Result<Tensor<2>> {
        match (self, input) {
            (Self::Mlp(mlp), BatchInput::Tabular(features)) => Ok(mlp.forward(features)),
            (Self::ResNet(resnet), BatchInput::Image(images)) => Ok(resnet.forward(images)),
            (Self::MobileNetV2(mobilenet), BatchInput::Image(images)) => {
                Ok(mobilenet.forward(images))
            }
            (Self::Mlp(_), BatchInput::Image(_))
            | (Self::ResNet(_) | Self::MobileNetV2(_), BatchInput::Tabular(_)) => Err(
                DlError::Data("the input modality does not match the backbone".to_string()),
            ),
        }
    }
}
