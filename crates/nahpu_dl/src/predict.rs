use serde::{Deserialize, Serialize};

use crate::data::Sample;
use crate::manifest::Task;
use crate::models::{DlModel, OutputKind};
use crate::progress::{NoProgress, ProgressReporter};
use crate::{DlError, Result};

/// Default number of samples per inference batch.
pub const DEFAULT_BATCH_SIZE: usize = 64;

/// Default number of ranked classes kept per prediction.
pub const DEFAULT_TOP_K: usize = 5;

/// The model output for one sample.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Prediction {
    /// Sample identifier.
    pub id: String,
    /// What the model predicted.
    pub output: PredictionOutput,
}

/// A classification or regression result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PredictionOutput {
    /// Most likely class and the ranked alternatives.
    Class {
        /// Most likely label.
        label: String,
        /// Its probability.
        score: f32,
        /// Highest-scoring labels, best first, including the top one.
        top_k: Vec<ClassScore>,
    },
    /// Predicted value.
    Value {
        /// The value.
        value: f32,
    },
}

/// A label with its probability.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassScore {
    /// Class label.
    pub label: String,
    /// Softmax probability.
    pub score: f32,
}

/// Runs a classification or regression model on samples.
pub struct Predictor<'a> {
    model: &'a DlModel,
    batch_size: usize,
    top_k: usize,
    progress: &'a dyn ProgressReporter,
}

impl<'a> Predictor<'a> {
    /// Creates a predictor with the default batch size and top-k.
    pub fn new(model: &'a DlModel) -> Self {
        Self {
            model,
            batch_size: DEFAULT_BATCH_SIZE,
            top_k: DEFAULT_TOP_K,
            progress: &NoProgress,
        }
    }

    /// Sets the number of samples per batch.
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    /// Sets how many ranked classes each prediction keeps.
    pub fn with_top_k(mut self, top_k: usize) -> Self {
        self.top_k = top_k;
        self
    }

    /// Reports one step per batch to `progress`.
    pub fn with_progress(mut self, progress: &'a dyn ProgressReporter) -> Self {
        self.progress = progress;
        self
    }

    /// Predicts every sample.
    ///
    /// # Errors
    ///
    /// Returns an error when a sample does not fit the model, or when the model is an
    /// embedding-only model.
    pub fn predict(&self, samples: &[Sample]) -> Result<Vec<Prediction>> {
        let rows = self.predict_rows(samples)?;
        samples
            .iter()
            .zip(rows)
            .map(|(sample, row)| {
                Ok(Prediction {
                    id: sample.id.clone(),
                    output: self.to_output(&row)?,
                })
            })
            .collect()
    }

    /// Raw head outputs: class probabilities for classifiers, one value for regressors.
    pub(crate) fn predict_rows(&self, samples: &[Sample]) -> Result<Vec<Vec<f32>>> {
        let rows =
            self.model
                .run_rows(samples, self.batch_size, OutputKind::Outputs, self.progress)?;
        let manifest = self.model.manifest();
        match (manifest.task, manifest.preprocessing.target) {
            (Task::Classification, _) => Ok(rows.iter().map(|row| softmax(row)).collect()),
            (Task::Regression, Some(scaling)) => Ok(rows
                .into_iter()
                .map(|row| row.into_iter().map(|v| scaling.unscale(v)).collect())
                .collect()),
            (Task::Regression | Task::Embedding, _) => Ok(rows),
        }
    }

    fn to_output(&self, row: &[f32]) -> Result<PredictionOutput> {
        let manifest = self.model.manifest();
        match manifest.task {
            Task::Classification => {
                let top_k = rank_classes(row, &manifest.labels, self.top_k.max(1));
                let best = top_k.first().cloned().ok_or_else(|| {
                    DlError::Data("the model returned no class scores".to_string())
                })?;
                Ok(PredictionOutput::Class {
                    label: best.label,
                    score: best.score,
                    top_k,
                })
            }
            Task::Regression => Ok(PredictionOutput::Value {
                value: row.first().copied().unwrap_or(f32::NAN),
            }),
            Task::Embedding => Err(DlError::Unsupported(
                "embedding models do not predict; use Embedder".to_string(),
            )),
        }
    }
}

/// Numerically stable softmax.
pub(crate) fn softmax(logits: &[f32]) -> Vec<f32> {
    let max = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let exps: Vec<f32> = logits.iter().map(|x| (x - max).exp()).collect();
    let sum: f32 = exps.iter().sum();
    exps.iter().map(|x| x / sum).collect()
}

/// Indices of `scores` from highest to lowest.
pub(crate) fn ranked_indices(scores: &[f32]) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..scores.len()).collect();
    indices.sort_by(|&a, &b| scores[b].total_cmp(&scores[a]));
    indices
}

fn rank_classes(probabilities: &[f32], labels: &[String], k: usize) -> Vec<ClassScore> {
    ranked_indices(probabilities)
        .into_iter()
        .take(k)
        .map(|i| ClassScore {
            label: labels.get(i).cloned().unwrap_or_else(|| i.to_string()),
            score: probabilities[i],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_softmax_sums_to_one() {
        let probabilities = softmax(&[1.0, 2.0, 3.0]);
        assert!((probabilities.iter().sum::<f32>() - 1.0).abs() < 1e-6);
        assert!(probabilities[2] > probabilities[1]);
    }

    #[test]
    fn test_rank_classes() {
        let labels = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let ranked = rank_classes(&[0.2, 0.5, 0.3], &labels, 2);
        assert_eq!(ranked.len(), 2);
        assert_eq!(ranked[0].label, "b");
        assert_eq!(ranked[1].label, "c");
    }
}
