//! Network definitions and the [`DlModel`] that pairs a network with its manifest.

use std::path::{Path, PathBuf};

use burn::tensor::{Device, Tensor};

use crate::data::{self, BatchInput, Sample};
use crate::manifest::{ModelManifest, WeightFormat, WeightsInfo};
use crate::progress::ProgressReporter;
use crate::{DlError, Result, store};

pub mod heads;
pub mod image;
pub mod network;
pub mod tabular;

pub use heads::Head;
pub use network::{Backbone, Network};

/// Which network output [`DlModel::run_rows`] returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputKind {
    /// Backbone embeddings.
    Embeddings,
    /// Head outputs: logits or regression values.
    Outputs,
}

/// A model package loaded on a device: the manifest plus the network weights.
#[derive(Debug, Clone)]
pub struct DlModel {
    manifest: ModelManifest,
    network: Network,
    device: Device,
}

impl DlModel {
    /// Creates a model with freshly initialized weights from a manifest.
    ///
    /// # Errors
    ///
    /// Returns an error when the manifest describes an unusable network.
    pub fn new(manifest: ModelManifest, device: &Device) -> Result<Self> {
        let network = Network::init(&manifest, device)?;
        Ok(Self {
            manifest,
            network,
            device: device.clone(),
        })
    }

    /// Loads a model package directory.
    ///
    /// # Errors
    ///
    /// Returns an error when the manifest or weights cannot be read, or when the weights do
    /// not match the architecture.
    pub fn load<P: AsRef<Path>>(dir: P, device: &Device) -> Result<Self> {
        let dir = dir.as_ref();
        let manifest = ModelManifest::from_dir(dir)?;
        let mut network = Network::init(&manifest, device)?;
        let weights_path = dir.join(&manifest.weights.file);
        store::load_weights(&mut network, &weights_path, manifest.weights.format)?;
        Ok(Self {
            manifest,
            network,
            device: device.clone(),
        })
    }

    /// Saves the model as a package directory and returns the weight file path.
    ///
    /// The manifest's weight entry is updated to the chosen format.
    ///
    /// # Errors
    ///
    /// Returns an error when the directory or files cannot be written, or when the format
    /// is not supported yet.
    pub fn save<P: AsRef<Path>>(&mut self, dir: P, format: WeightFormat) -> Result<PathBuf> {
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir).map_err(|e| DlError::io(dir, e))?;
        let weights = WeightsInfo::for_format(format);
        let weights_path = dir.join(&weights.file);
        store::save_weights(&self.network, &weights_path, format)?;
        self.manifest.weights = weights;
        self.manifest.write_to_dir(dir)?;
        Ok(weights_path)
    }

    /// The model manifest.
    pub fn manifest(&self) -> &ModelManifest {
        &self.manifest
    }

    /// Mutable access to the manifest, for example to record evaluation metrics.
    pub fn manifest_mut(&mut self) -> &mut ModelManifest {
        &mut self.manifest
    }

    /// The device the weights live on.
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// The Burn network.
    pub fn network(&self) -> &Network {
        &self.network
    }

    pub(crate) fn from_parts(manifest: ModelManifest, network: Network, device: &Device) -> Self {
        Self {
            manifest,
            network,
            device: device.clone(),
        }
    }

    fn encode(&self, samples: &[Sample]) -> Result<BatchInput> {
        data::encode_batch(&self.manifest, samples, &self.device, None)
    }

    fn embeddings(&self, input: BatchInput) -> Result<Tensor<2>> {
        self.network.backbone.forward_features(input)
    }

    fn outputs(&self, input: BatchInput) -> Result<Tensor<2>> {
        if self.network.head.is_none() {
            return Err(DlError::Unsupported(format!(
                "model `{}` has no prediction head; use embeddings instead",
                self.manifest.name
            )));
        }
        self.network.forward(input)
    }

    /// Runs the model over samples in batches and returns one output row per sample.
    pub(crate) fn run_rows(
        &self,
        samples: &[Sample],
        batch_size: usize,
        kind: OutputKind,
        progress: &dyn ProgressReporter,
    ) -> Result<Vec<Vec<f32>>> {
        let batch_size = batch_size.max(1);
        progress.start(&self.manifest.name, samples.len().div_ceil(batch_size));
        let mut rows = Vec::with_capacity(samples.len());
        for batch in samples.chunks(batch_size) {
            let input = self.encode(batch)?;
            let output = match kind {
                OutputKind::Embeddings => self.embeddings(input)?,
                OutputKind::Outputs => self.outputs(input)?,
            };
            rows.extend(tensor_rows(output)?);
            progress.advance(1);
        }
        progress.finish();
        Ok(rows)
    }
}

/// Copies a `[rows, columns]` tensor into row vectors.
pub(crate) fn tensor_rows(tensor: Tensor<2>) -> Result<Vec<Vec<f32>>> {
    let [_, columns] = tensor.dims();
    let values = tensor
        .into_data()
        .try_into_vec_as::<f32>()
        .map_err(|e| DlError::Data(format!("cannot read tensor values: {e:?}")))?;
    Ok(values.chunks(columns.max(1)).map(<[f32]>::to_vec).collect())
}
