#![warn(missing_docs)]
//! Deep learning support for NAHPU.
//!
//! The crate runs inference, extracts embeddings, evaluates, and fine-tunes models stored as
//! model packages: a directory holding a [`ModelManifest`] and a weight file. It is built on
//! Burn, so model code is backend-agnostic and the backend is chosen at runtime with
//! [`DeviceKind`].
//!
//! ```no_run
//! use nahpu_dl::{DeviceKind, DlModel, Predictor, Sample, SampleInput};
//!
//! # fn main() -> Result<(), nahpu_dl::DlError> {
//! let device = DeviceKind::Cpu.to_device()?;
//! let model = DlModel::load("models/skull_size", &device)?;
//! let samples = vec![Sample::new("MVZ-1", SampleInput::Tabular(vec![24.1, 9.3]))];
//! let predictions = Predictor::new(&model).predict(&samples)?;
//! # Ok(())
//! # }
//! ```
//!
//! Fine-tuning needs the `train` feature.

pub mod data;
mod device;
mod embed;
mod error;
mod evaluate;
#[cfg(feature = "train")]
mod finetune;
#[cfg(feature = "hub")]
mod hub;
mod manifest;
pub mod models;
mod predict;
mod pretrained;
mod progress;
mod store;

pub use data::{LabeledDataset, Sample, SampleInput, Target};
pub use device::DeviceKind;
pub use embed::{Embedder, Embedding};
pub use error::DlError;
pub use evaluate::{ClassMetrics, ClassificationMetrics, EvalReport, Evaluator, RegressionMetrics};
#[cfg(feature = "train")]
pub use finetune::{EpochStats, FineTuneConfig, FineTuneOutcome, FineTuneStrategy, FineTuner};
#[cfg(feature = "hub")]
pub use hub::{PRETRAINED_MODELS, PretrainedModel, download};
pub use manifest::{
    Architecture, DEFAULT_IMAGE_SIZE, ImagePreprocessing, MANIFEST_FILE_NAME, Modality,
    ModelManifest, Preprocessing, Provenance, TabularPreprocessing, TargetScaling, Task,
    WeightFormat, WeightsInfo,
};
pub use models::DlModel;
pub use predict::{
    ClassScore, DEFAULT_BATCH_SIZE, DEFAULT_TOP_K, Prediction, PredictionOutput, Predictor,
};
pub use pretrained::{IMAGENET_CLASSES, PretrainedImport};
pub use progress::{NoProgress, ProgressReporter};

/// Version of the compiled `nahpu_dl` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Result type used across the crate.
pub type Result<T> = std::result::Result<T, DlError>;
