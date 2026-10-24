use serde::{Deserialize, Serialize};

use crate::Result;
use crate::data::Sample;
use crate::models::{DlModel, OutputKind};
use crate::predict::DEFAULT_BATCH_SIZE;
use crate::progress::{NoProgress, ProgressReporter};

/// The embedding of one sample.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Embedding {
    /// Sample identifier.
    pub id: String,
    /// Embedding vector.
    pub vector: Vec<f32>,
}

/// Extracts backbone embeddings. Works with any model, with or without a head.
pub struct Embedder<'a> {
    model: &'a DlModel,
    batch_size: usize,
    normalize: bool,
    progress: &'a dyn ProgressReporter,
}

impl<'a> Embedder<'a> {
    /// Creates an embedder that returns L2-normalized vectors.
    pub fn new(model: &'a DlModel) -> Self {
        Self {
            model,
            batch_size: DEFAULT_BATCH_SIZE,
            normalize: true,
            progress: &NoProgress,
        }
    }

    /// Sets the number of samples per batch.
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    /// Whether to scale vectors to unit length. Unit vectors make cosine similarity a dot
    /// product, which is what similarity search usually wants.
    pub fn with_normalize(mut self, normalize: bool) -> Self {
        self.normalize = normalize;
        self
    }

    /// Reports one step per batch to `progress`.
    pub fn with_progress(mut self, progress: &'a dyn ProgressReporter) -> Self {
        self.progress = progress;
        self
    }

    /// Embeds every sample.
    ///
    /// # Errors
    ///
    /// Returns an error when a sample does not fit the model.
    pub fn embed(&self, samples: &[Sample]) -> Result<Vec<Embedding>> {
        let rows = self.model.run_rows(
            samples,
            self.batch_size,
            OutputKind::Embeddings,
            self.progress,
        )?;
        Ok(samples
            .iter()
            .zip(rows)
            .map(|(sample, vector)| Embedding {
                id: sample.id.clone(),
                vector: if self.normalize {
                    l2_normalize(vector)
                } else {
                    vector
                },
            })
            .collect())
    }
}

fn l2_normalize(vector: Vec<f32>) -> Vec<f32> {
    let norm = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    // A zero vector has no direction; leave it as is rather than produce NaNs.
    if norm <= f32::EPSILON {
        return vector;
    }
    vector.into_iter().map(|x| x / norm).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_l2_normalize() {
        assert_eq!(l2_normalize(vec![3.0, 4.0]), vec![0.6, 0.8]);
        assert_eq!(l2_normalize(vec![0.0, 0.0]), vec![0.0, 0.0]);
    }
}
