use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{DlError, Result};

/// File name of the manifest inside a model package.
pub const MANIFEST_FILE_NAME: &str = "manifest.toml";

/// Base name of the weight file inside a model package.
const WEIGHTS_BASE_NAME: &str = "model";

/// Describes a model package: what the model is and how to feed it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelManifest {
    /// Human-readable model name.
    pub name: String,
    /// Version of `nahpu_dl` that wrote the package.
    pub nahpu_dl_version: String,
    /// Kind of input the model takes.
    pub modality: Modality,
    /// What the model predicts.
    pub task: Task,
    /// Network architecture and its hyperparameters.
    pub architecture: Architecture,
    /// Class labels, in output order. Empty for regression and embedding models.
    #[serde(default)]
    pub labels: Vec<String>,
    /// How raw inputs are turned into tensors.
    #[serde(default)]
    pub preprocessing: Preprocessing,
    /// Weight file of the package.
    pub weights: WeightsInfo,
    /// Size of the embedding vector produced by the backbone.
    pub embedding_dim: usize,
    /// Summary metrics from the last evaluation, keyed by metric name.
    #[serde(default)]
    pub metrics: BTreeMap<String, f64>,
    /// Where the model came from.
    #[serde(default)]
    pub provenance: Provenance,
}

impl ModelManifest {
    /// Reads `manifest.toml` from a model package directory.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read or parsed.
    pub fn from_dir<P: AsRef<Path>>(dir: P) -> Result<Self> {
        Self::from_path(dir.as_ref().join(MANIFEST_FILE_NAME))
    }

    /// Reads a manifest file.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read or parsed, or when it is inconsistent.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let text = fs::read_to_string(path).map_err(|e| DlError::io(path, e))?;
        let manifest: Self = toml::from_str(&text).map_err(|e| DlError::Manifest {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?;
        manifest.validate().map_err(|message| DlError::Manifest {
            path: path.to_path_buf(),
            message,
        })?;
        Ok(manifest)
    }

    /// Writes `manifest.toml` into a model package directory, creating it if needed.
    ///
    /// # Errors
    ///
    /// Returns an error when the directory or file cannot be written.
    pub fn write_to_dir<P: AsRef<Path>>(&self, dir: P) -> Result<()> {
        let dir = dir.as_ref();
        fs::create_dir_all(dir).map_err(|e| DlError::io(dir, e))?;
        let path = dir.join(MANIFEST_FILE_NAME);
        let text = toml::to_string_pretty(self).map_err(|e| DlError::Manifest {
            path: path.clone(),
            message: e.to_string(),
        })?;
        fs::write(&path, text).map_err(|e| DlError::io(&path, e))
    }

    /// Number of values the model head outputs, or `None` for embedding-only models.
    pub fn output_dim(&self) -> Option<usize> {
        match self.task {
            Task::Classification => Some(self.labels.len()),
            Task::Regression => Some(1),
            Task::Embedding => None,
        }
    }

    fn validate(&self) -> std::result::Result<(), String> {
        if self.task == Task::Classification && self.labels.len() < 2 {
            return Err("classification models need at least two labels".to_string());
        }
        if self.modality == Modality::Tabular && self.preprocessing.tabular.is_none() {
            return Err("tabular models need a [preprocessing.tabular] section".to_string());
        }
        Ok(())
    }
}

/// Kind of input a model takes.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Modality {
    /// Numeric features, such as specimen measurements.
    #[default]
    Tabular,
    /// Images, such as specimen photos.
    Image,
    /// Text, such as remarks and narratives.
    Text,
}

/// What a model predicts.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Task {
    /// Assigns one of [`ModelManifest::labels`].
    #[default]
    Classification,
    /// Predicts one numeric value.
    Regression,
    /// Produces embeddings only; there is no prediction head.
    Embedding,
}

/// Network architecture and its hyperparameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Architecture {
    /// Multilayer perceptron for tabular data.
    Mlp {
        /// Number of input features.
        input_dim: usize,
        /// Width of each hidden layer. The last one is the embedding size.
        hidden: Vec<usize>,
        /// Dropout probability applied after each hidden layer during training.
        dropout: f64,
    },
}

impl Architecture {
    /// Size of the embedding the backbone produces.
    pub fn embedding_dim(&self) -> usize {
        match self {
            Self::Mlp {
                input_dim, hidden, ..
            } => hidden.last().copied().unwrap_or(*input_dim),
        }
    }

    /// The modality this architecture accepts.
    pub fn modality(&self) -> Modality {
        match self {
            Self::Mlp { .. } => Modality::Tabular,
        }
    }
}

/// How raw inputs are turned into tensors. Only the section matching the modality is set.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Preprocessing {
    /// Feature order and standardization for tabular models.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tabular: Option<TabularPreprocessing>,
    /// Scaling of regression targets. Predictions are mapped back to the original units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetScaling>,
}

/// Standardization of a regression target.
///
/// Training on standardized targets keeps the loss on a similar scale whatever the units.
#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TargetScaling {
    /// Training-set mean of the target.
    pub mean: f32,
    /// Training-set standard deviation of the target.
    pub std: f32,
}

impl TargetScaling {
    /// Maps a value in original units to the scale the model is trained on.
    pub fn scale(&self, value: f32) -> f32 {
        if self.std > f32::EPSILON {
            (value - self.mean) / self.std
        } else {
            value - self.mean
        }
    }

    /// Maps a model output back to original units.
    pub fn unscale(&self, value: f32) -> f32 {
        if self.std > f32::EPSILON {
            value * self.std + self.mean
        } else {
            value + self.mean
        }
    }
}

/// Feature order and standardization statistics for tabular inputs.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabularPreprocessing {
    /// Feature names, in input order.
    pub feature_names: Vec<String>,
    /// Training-set mean of each feature. Missing values are replaced by it.
    pub means: Vec<f32>,
    /// Training-set standard deviation of each feature.
    pub stds: Vec<f32>,
}

impl TabularPreprocessing {
    /// Standardizes one row in place. `NaN` marks a missing value and becomes zero, the mean.
    ///
    /// # Errors
    ///
    /// Returns [`DlError::Data`] when the row length does not match the feature count.
    pub fn standardize(&self, row: &[f32]) -> Result<Vec<f32>> {
        if row.len() != self.feature_names.len() {
            return Err(DlError::Data(format!(
                "expected {} features ({}), found {}",
                self.feature_names.len(),
                self.feature_names.join(", "),
                row.len()
            )));
        }
        let values = row
            .iter()
            .zip(self.means.iter().zip(&self.stds))
            .map(|(value, (mean, std))| standardize_value(*value, *mean, *std))
            .collect();
        Ok(values)
    }
}

fn standardize_value(value: f32, mean: f32, std: f32) -> f32 {
    if value.is_nan() {
        return 0.0;
    }
    // A constant feature has zero spread; centering it is enough.
    if std > f32::EPSILON {
        (value - mean) / std
    } else {
        value - mean
    }
}

/// Weight file of a model package.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeightsInfo {
    /// File name relative to the package directory.
    pub file: String,
    /// Storage format of the file.
    pub format: WeightFormat,
}

impl WeightsInfo {
    /// The default weight file name for a format.
    pub fn for_format(format: WeightFormat) -> Self {
        Self {
            file: format!("{WEIGHTS_BASE_NAME}.{}", format.extension()),
            format,
        }
    }
}

/// Storage format of model weights.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeightFormat {
    /// Burn's native format.
    #[default]
    Burnpack,
    /// Hugging Face SafeTensors.
    Safetensors,
    /// Open Neural Network Exchange.
    Onnx,
}

impl WeightFormat {
    /// File extension, without the dot.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Burnpack => "bpk",
            Self::Safetensors => "safetensors",
            Self::Onnx => "onnx",
        }
    }
}

impl std::str::FromStr for WeightFormat {
    type Err = DlError;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "burnpack" | "bpk" => Ok(Self::Burnpack),
            "safetensors" => Ok(Self::Safetensors),
            "onnx" => Ok(Self::Onnx),
            other => Err(DlError::Config(format!(
                "unknown weight format `{other}`; expected burnpack, safetensors, or onnx"
            ))),
        }
    }
}

/// Where a model came from.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// Model package the weights were fine-tuned from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_model: Option<String>,
    /// Training data source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dataset: Option<String>,
    /// Random seed used for splitting and initialization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    /// Creation time, as written by the caller.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest() -> ModelManifest {
        let architecture = Architecture::Mlp {
            input_dim: 2,
            hidden: vec![8, 4],
            dropout: 0.1,
        };
        ModelManifest {
            name: "skull".to_string(),
            nahpu_dl_version: crate::VERSION.to_string(),
            modality: Modality::Tabular,
            task: Task::Classification,
            embedding_dim: architecture.embedding_dim(),
            architecture,
            labels: vec!["a".to_string(), "b".to_string()],
            preprocessing: Preprocessing {
                tabular: Some(TabularPreprocessing {
                    feature_names: vec!["x".to_string(), "y".to_string()],
                    means: vec![1.0, 2.0],
                    stds: vec![0.5, 0.0],
                }),
                target: None,
            },
            weights: WeightsInfo::for_format(WeightFormat::Burnpack),
            metrics: BTreeMap::from([("accuracy".to_string(), 0.9)]),
            provenance: Provenance {
                seed: Some(42),
                ..Default::default()
            },
        }
    }

    #[test]
    fn test_manifest_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = sample_manifest();
        manifest.write_to_dir(dir.path()).unwrap();
        let read = ModelManifest::from_dir(dir.path()).unwrap();
        assert_eq!(read, manifest);
        assert_eq!(read.weights.file, "model.bpk");
        assert_eq!(read.output_dim(), Some(2));
    }

    #[test]
    fn test_manifest_rejects_single_label_classifier() {
        let dir = tempfile::tempdir().unwrap();
        let mut manifest = sample_manifest();
        manifest.labels.truncate(1);
        manifest.write_to_dir(dir.path()).unwrap();
        assert!(ModelManifest::from_dir(dir.path()).is_err());
    }

    #[test]
    fn test_standardize_handles_missing_and_constant_features() {
        let tabular = sample_manifest().preprocessing.tabular.unwrap();
        let row = tabular.standardize(&[2.0, f32::NAN]).unwrap();
        assert_eq!(row, vec![2.0, 0.0]);
        let row = tabular.standardize(&[1.0, 5.0]).unwrap();
        assert_eq!(row, vec![0.0, 3.0]);
        assert!(tabular.standardize(&[1.0]).is_err());
    }

    #[test]
    fn test_parse_weight_format() {
        assert_eq!(
            "bpk".parse::<WeightFormat>().unwrap(),
            WeightFormat::Burnpack
        );
        assert_eq!(
            "SafeTensors".parse::<WeightFormat>().unwrap(),
            WeightFormat::Safetensors
        );
        assert!("pt".parse::<WeightFormat>().is_err());
    }
}
