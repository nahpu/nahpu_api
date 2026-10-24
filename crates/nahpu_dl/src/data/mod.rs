//! Samples, datasets, and conversion of raw inputs into tensors.

use std::path::PathBuf;

use burn::tensor::{Device, Tensor, TensorData};
use serde::{Deserialize, Serialize};

use crate::manifest::{Modality, ModelManifest, TabularPreprocessing};
use crate::{DlError, Result};

mod image;
mod image_folder;
mod nahpu;
pub(crate) mod split;
mod tabular;

pub use image::{IMAGE_EXTENSIONS, is_image_path};
pub use image_folder::ImageFolder;
pub use nahpu::{NahpuPackage, PROJECT_FILE_NAME, TaxonRank};
pub use split::DataSplit;
pub use tabular::TabularCsv;

/// One input to a model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sample {
    /// Identifier carried through to predictions and embeddings, such as a catalog number.
    pub id: String,
    /// The raw input.
    pub input: SampleInput,
}

impl Sample {
    /// Creates a sample.
    pub fn new(id: impl Into<String>, input: SampleInput) -> Self {
        Self {
            id: id.into(),
            input,
        }
    }
}

/// Raw model input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SampleInput {
    /// Numeric features in the model's feature order. `NaN` marks a missing value.
    Tabular(Vec<f32>),
    /// Path to an image file.
    ImagePath(PathBuf),
    /// Encoded image bytes (JPEG, PNG, or WebP).
    ImageBytes(Vec<u8>),
    /// Free text.
    Text(String),
}

impl SampleInput {
    fn modality(&self) -> Modality {
        match self {
            Self::Tabular(_) => Modality::Tabular,
            Self::ImagePath(_) | Self::ImageBytes(_) => Modality::Image,
            Self::Text(_) => Modality::Text,
        }
    }
}

/// Ground truth for a sample.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Target {
    /// Class label.
    Class(String),
    /// Numeric value.
    Value(f32),
}

/// Samples with ground truth, used for fine-tuning and evaluation.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct LabeledDataset {
    /// The inputs.
    pub samples: Vec<Sample>,
    /// Ground truth, one per sample.
    pub targets: Vec<Target>,
    /// Feature names for tabular data, in input order. Empty for other modalities.
    pub feature_names: Vec<String>,
    /// Rows skipped while reading because their target was missing.
    pub skipped_rows: usize,
}

impl LabeledDataset {
    /// Number of samples.
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Whether the dataset has no samples.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Copies the samples at `indices` into a new dataset.
    pub fn subset(&self, indices: &[usize]) -> Self {
        Self {
            samples: indices.iter().map(|&i| self.samples[i].clone()).collect(),
            targets: indices.iter().map(|&i| self.targets[i].clone()).collect(),
            feature_names: self.feature_names.clone(),
            skipped_rows: 0,
        }
    }

    /// Sorted, unique class labels. Empty when the targets are numeric.
    pub fn class_labels(&self) -> Vec<String> {
        let mut labels: Vec<String> = self
            .targets
            .iter()
            .filter_map(|target| match target {
                Target::Class(label) => Some(label.clone()),
                Target::Value(_) => None,
            })
            .collect();
        labels.sort();
        labels.dedup();
        labels
    }

    /// Computes per-feature mean and standard deviation, ignoring missing values.
    ///
    /// # Errors
    ///
    /// Returns [`DlError::Data`] when a sample is not tabular or has the wrong length.
    pub fn tabular_preprocessing(&self) -> Result<TabularPreprocessing> {
        let n_features = self.feature_names.len();
        let rows = self.tabular_rows()?;
        let (means, stds) = (0..n_features)
            .map(|column| column_stats(rows.iter().map(|row| row[column])))
            .unzip();
        Ok(TabularPreprocessing {
            feature_names: self.feature_names.clone(),
            means,
            stds,
        })
    }

    fn tabular_rows(&self) -> Result<Vec<&[f32]>> {
        self.samples
            .iter()
            .map(|sample| match &sample.input {
                SampleInput::Tabular(row) if row.len() == self.feature_names.len() => {
                    Ok(row.as_slice())
                }
                SampleInput::Tabular(row) => Err(DlError::Data(format!(
                    "sample `{}` has {} features, expected {}",
                    sample.id,
                    row.len(),
                    self.feature_names.len()
                ))),
                _ => Err(DlError::Data(format!(
                    "sample `{}` is not tabular",
                    sample.id
                ))),
            })
            .collect()
    }
}

/// A batch converted to tensors, one variant per modality.
#[derive(Debug, Clone)]
pub enum BatchInput {
    /// Standardized features: `[batch, features]`.
    Tabular(Tensor<2>),
    /// Normalized images: `[batch, 3, height, width]`.
    Image(Tensor<4>),
}

/// Converts raw samples into a tensor batch using the manifest's preprocessing.
///
/// `augment_seed` turns on random training augmentation for image models.
pub(crate) fn encode_batch(
    manifest: &ModelManifest,
    samples: &[Sample],
    device: &Device,
    augment_seed: Option<u64>,
) -> Result<BatchInput> {
    if let Some(sample) = samples
        .iter()
        .find(|sample| sample.input.modality() != manifest.modality)
    {
        return Err(DlError::Data(format!(
            "sample `{}` is {:?} input but the model expects {:?}",
            sample.id,
            sample.input.modality(),
            manifest.modality
        )));
    }
    match manifest.modality {
        Modality::Tabular => encode_tabular(manifest, samples, device),
        Modality::Image => encode_images(manifest, samples, device, augment_seed),
        Modality::Text => Err(DlError::Unsupported(
            "text models are not supported yet".to_string(),
        )),
    }
}

fn encode_tabular(
    manifest: &ModelManifest,
    samples: &[Sample],
    device: &Device,
) -> Result<BatchInput> {
    let preprocessing =
        manifest.preprocessing.tabular.as_ref().ok_or_else(|| {
            DlError::Config("the manifest has no tabular preprocessing".to_string())
        })?;
    let n_features = preprocessing.feature_names.len();
    let mut values = Vec::with_capacity(samples.len() * n_features);
    for sample in samples {
        if let SampleInput::Tabular(row) = &sample.input {
            values.extend(preprocessing.standardize(row)?);
        }
    }
    let data = TensorData::new(values, [samples.len(), n_features]);
    Ok(BatchInput::Tabular(Tensor::from_data(data, device)))
}

fn encode_images(
    manifest: &ModelManifest,
    samples: &[Sample],
    device: &Device,
    augment_seed: Option<u64>,
) -> Result<BatchInput> {
    let preprocessing =
        manifest.preprocessing.image.as_ref().ok_or_else(|| {
            DlError::Config("the manifest has no image preprocessing".to_string())
        })?;
    let pixels = image::encode_pixels(samples, preprocessing, augment_seed)?;
    let size = preprocessing.size as usize;
    let data = TensorData::new(pixels, [samples.len(), 3, size, size]);
    Ok(BatchInput::Image(Tensor::from_data(data, device)))
}

fn column_stats(values: impl Iterator<Item = f32>) -> (f32, f32) {
    let present: Vec<f64> = values
        .filter(|value| !value.is_nan())
        .map(f64::from)
        .collect();
    if present.is_empty() {
        return (0.0, 0.0);
    }
    let n = present.len() as f64;
    let mean = present.iter().sum::<f64>() / n;
    let variance = present
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / n;
    (mean as f32, variance.sqrt() as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dataset() -> LabeledDataset {
        let rows = [vec![1.0, f32::NAN], vec![3.0, 4.0], vec![5.0, 6.0]];
        LabeledDataset {
            samples: rows
                .iter()
                .enumerate()
                .map(|(i, row)| Sample::new(i.to_string(), SampleInput::Tabular(row.clone())))
                .collect(),
            targets: ["b", "a", "b"]
                .iter()
                .map(|label| Target::Class(label.to_string()))
                .collect(),
            feature_names: vec!["x".to_string(), "y".to_string()],
            skipped_rows: 0,
        }
    }

    #[test]
    fn test_tabular_preprocessing_ignores_missing() {
        let stats = dataset().tabular_preprocessing().unwrap();
        assert_eq!(stats.means, vec![3.0, 5.0]);
        assert!((stats.stds[0] - (8.0f32 / 3.0).sqrt()).abs() < 1e-6);
        assert!((stats.stds[1] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_class_labels_sorted_unique() {
        assert_eq!(dataset().class_labels(), vec!["a", "b"]);
    }

    #[test]
    fn test_subset_keeps_order() {
        let subset = dataset().subset(&[2, 0]);
        assert_eq!(subset.samples[0].id, "2");
        assert_eq!(subset.targets[1], Target::Class("b".to_string()));
    }
}
