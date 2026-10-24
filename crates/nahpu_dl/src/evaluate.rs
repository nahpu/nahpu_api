use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::data::{LabeledDataset, Target};
use crate::manifest::Task;
use crate::models::DlModel;
use crate::predict::{DEFAULT_BATCH_SIZE, DEFAULT_TOP_K, Predictor, ranked_indices};
use crate::progress::{NoProgress, ProgressReporter};
use crate::{DlError, Result};

/// Evaluation results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "task", rename_all = "snake_case")]
pub enum EvalReport {
    /// Metrics of a classifier.
    Classification(ClassificationMetrics),
    /// Metrics of a regressor.
    Regression(RegressionMetrics),
}

impl EvalReport {
    /// Headline metrics, suitable for [`crate::ModelManifest::metrics`].
    pub fn summary(&self) -> BTreeMap<String, f64> {
        let pairs: Vec<(&str, f64)> = match self {
            Self::Classification(m) => vec![
                ("accuracy", m.accuracy),
                ("top_k_accuracy", m.top_k_accuracy),
                ("macro_f1", m.macro_f1),
                ("weighted_f1", m.weighted_f1),
            ],
            Self::Regression(m) => vec![("mae", m.mae), ("rmse", m.rmse), ("r2", m.r2)],
        };
        pairs
            .into_iter()
            .map(|(name, value)| (name.to_string(), value))
            .collect()
    }
}

/// Classification metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassificationMetrics {
    /// Number of evaluated samples.
    pub n_samples: usize,
    /// Fraction of samples whose top prediction is correct.
    pub accuracy: f64,
    /// The `k` of [`ClassificationMetrics::top_k_accuracy`].
    pub top_k: usize,
    /// Fraction of samples whose label is among the top `k` predictions.
    pub top_k_accuracy: f64,
    /// Unweighted mean of per-class precision.
    pub macro_precision: f64,
    /// Unweighted mean of per-class recall.
    pub macro_recall: f64,
    /// Unweighted mean of per-class F1.
    pub macro_f1: f64,
    /// Mean of per-class F1 weighted by support.
    pub weighted_f1: f64,
    /// Metrics per class, in label order.
    pub per_class: Vec<ClassMetrics>,
    /// Counts with true labels as rows and predicted labels as columns, in label order.
    pub confusion_matrix: Vec<Vec<usize>>,
}

/// Metrics for one class.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassMetrics {
    /// Class label.
    pub label: String,
    /// True positives over predicted positives.
    pub precision: f64,
    /// True positives over actual positives.
    pub recall: f64,
    /// Harmonic mean of precision and recall.
    pub f1: f64,
    /// Number of samples with this label.
    pub support: usize,
}

/// Regression metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegressionMetrics {
    /// Number of evaluated samples.
    pub n_samples: usize,
    /// Mean absolute error.
    pub mae: f64,
    /// Root mean squared error.
    pub rmse: f64,
    /// Coefficient of determination.
    pub r2: f64,
}

/// Measures a model against a labeled dataset.
pub struct Evaluator<'a> {
    model: &'a DlModel,
    batch_size: usize,
    top_k: usize,
    progress: &'a dyn ProgressReporter,
}

impl<'a> Evaluator<'a> {
    /// Creates an evaluator with the default batch size and top-k.
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

    /// Sets the `k` for top-k accuracy.
    pub fn with_top_k(mut self, top_k: usize) -> Self {
        self.top_k = top_k;
        self
    }

    /// Reports one step per batch to `progress`.
    pub fn with_progress(mut self, progress: &'a dyn ProgressReporter) -> Self {
        self.progress = progress;
        self
    }

    /// Evaluates the model.
    ///
    /// # Errors
    ///
    /// Returns an error when the dataset is empty, contains labels the model does not know,
    /// or does not fit the model.
    pub fn evaluate(&self, dataset: &LabeledDataset) -> Result<EvalReport> {
        if dataset.is_empty() {
            return Err(DlError::Data("the evaluation dataset is empty".to_string()));
        }
        let rows = Predictor::new(self.model)
            .with_batch_size(self.batch_size)
            .with_progress(self.progress)
            .predict_rows(&dataset.samples)?;
        match self.model.manifest().task {
            Task::Classification => self.classification(dataset, &rows),
            Task::Regression => regression(dataset, &rows),
            Task::Embedding => Err(DlError::Unsupported(
                "embedding models have no prediction head to evaluate".to_string(),
            )),
        }
    }

    fn classification(&self, dataset: &LabeledDataset, rows: &[Vec<f32>]) -> Result<EvalReport> {
        let labels = &self.model.manifest().labels;
        let truth = class_indices(&dataset.targets, labels)?;
        let ranked: Vec<Vec<usize>> = rows.iter().map(|row| ranked_indices(row)).collect();
        let predicted: Vec<usize> = ranked.iter().map(|order| order[0]).collect();
        let top_k = self.top_k.clamp(1, labels.len());
        let top_k_hits = truth
            .iter()
            .zip(&ranked)
            .filter(|(t, order)| order[..top_k].contains(t))
            .count();
        let mut metrics = classification_metrics(&truth, &predicted, labels);
        metrics.top_k = top_k;
        metrics.top_k_accuracy = top_k_hits as f64 / truth.len() as f64;
        Ok(EvalReport::Classification(metrics))
    }
}

fn class_indices(targets: &[Target], labels: &[String]) -> Result<Vec<usize>> {
    targets
        .iter()
        .map(|target| match target {
            Target::Class(label) => labels.iter().position(|l| l == label).ok_or_else(|| {
                DlError::Data(format!("label `{label}` is not one of the model's labels"))
            }),
            Target::Value(_) => Err(DlError::Data(
                "a classifier needs class labels, found a numeric target".to_string(),
            )),
        })
        .collect()
}

/// Computes classification metrics from true and predicted class indices.
pub(crate) fn classification_metrics(
    truth: &[usize],
    predicted: &[usize],
    labels: &[String],
) -> ClassificationMetrics {
    let n_classes = labels.len();
    let mut confusion_matrix = vec![vec![0usize; n_classes]; n_classes];
    for (&t, &p) in truth.iter().zip(predicted) {
        confusion_matrix[t][p] += 1;
    }
    let per_class: Vec<ClassMetrics> = labels
        .iter()
        .enumerate()
        .map(|(class, label)| class_metrics(&confusion_matrix, class, label))
        .collect();
    let n = truth.len();
    let correct = (0..n_classes)
        .map(|c| confusion_matrix[c][c])
        .sum::<usize>();
    let mean =
        |f: fn(&ClassMetrics) -> f64| per_class.iter().map(f).sum::<f64>() / n_classes as f64;
    let weighted_f1 = per_class
        .iter()
        .map(|m| m.f1 * m.support as f64)
        .sum::<f64>()
        / n as f64;
    ClassificationMetrics {
        n_samples: n,
        accuracy: correct as f64 / n as f64,
        top_k: 1,
        top_k_accuracy: correct as f64 / n as f64,
        macro_precision: mean(|m| m.precision),
        macro_recall: mean(|m| m.recall),
        macro_f1: mean(|m| m.f1),
        weighted_f1,
        per_class,
        confusion_matrix,
    }
}

fn class_metrics(confusion_matrix: &[Vec<usize>], class: usize, label: &str) -> ClassMetrics {
    let true_positives = confusion_matrix[class][class] as f64;
    let predicted: usize = confusion_matrix.iter().map(|row| row[class]).sum();
    let support: usize = confusion_matrix[class].iter().sum();
    let precision = ratio(true_positives, predicted as f64);
    let recall = ratio(true_positives, support as f64);
    ClassMetrics {
        label: label.to_string(),
        precision,
        recall,
        f1: ratio(2.0 * precision * recall, precision + recall),
        support,
    }
}

/// Division that treats 0/0 as 0, the usual convention for classes never predicted.
fn ratio(numerator: f64, denominator: f64) -> f64 {
    if denominator == 0.0 {
        0.0
    } else {
        numerator / denominator
    }
}

fn regression(dataset: &LabeledDataset, rows: &[Vec<f32>]) -> Result<EvalReport> {
    let truth = dataset
        .targets
        .iter()
        .map(|target| match target {
            Target::Value(value) => Ok(f64::from(*value)),
            Target::Class(_) => Err(DlError::Data(
                "a regressor needs numeric targets, found a class label".to_string(),
            )),
        })
        .collect::<Result<Vec<f64>>>()?;
    let predicted: Vec<f64> = rows.iter().map(|row| f64::from(row[0])).collect();
    Ok(EvalReport::Regression(regression_metrics(
        &truth, &predicted,
    )))
}

/// Computes regression metrics from true and predicted values.
pub(crate) fn regression_metrics(truth: &[f64], predicted: &[f64]) -> RegressionMetrics {
    let n = truth.len() as f64;
    let errors: Vec<f64> = truth.iter().zip(predicted).map(|(t, p)| p - t).collect();
    let mae = errors.iter().map(|e| e.abs()).sum::<f64>() / n;
    let sse = errors.iter().map(|e| e * e).sum::<f64>();
    let mean = truth.iter().sum::<f64>() / n;
    let sst = truth.iter().map(|t| (t - mean).powi(2)).sum::<f64>();
    RegressionMetrics {
        n_samples: truth.len(),
        mae,
        rmse: (sse / n).sqrt(),
        r2: if sst == 0.0 { 0.0 } else { 1.0 - sse / sst },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn test_classification_metrics_by_hand() {
        let labels = vec!["a".to_string(), "b".to_string()];
        // truth: a a a b b; predicted: a a b b a
        let metrics = classification_metrics(&[0, 0, 0, 1, 1], &[0, 0, 1, 1, 0], &labels);
        assert_eq!(metrics.confusion_matrix, vec![vec![2, 1], vec![1, 1]]);
        assert!(close(metrics.accuracy, 0.6));
        let a = &metrics.per_class[0];
        assert!(close(a.precision, 2.0 / 3.0));
        assert!(close(a.recall, 2.0 / 3.0));
        let b = &metrics.per_class[1];
        assert!(close(b.precision, 0.5));
        assert!(close(b.recall, 0.5));
        assert!(close(metrics.macro_f1, (2.0 / 3.0 + 0.5) / 2.0));
        assert!(close(
            metrics.weighted_f1,
            (2.0 / 3.0 * 3.0 + 0.5 * 2.0) / 5.0
        ));
    }

    #[test]
    fn test_unpredicted_class_has_zero_precision() {
        let labels = vec!["a".to_string(), "b".to_string()];
        let metrics = classification_metrics(&[0, 1], &[0, 0], &labels);
        assert!(close(metrics.per_class[1].precision, 0.0));
        assert!(close(metrics.per_class[1].f1, 0.0));
    }

    #[test]
    fn test_regression_metrics_by_hand() {
        let metrics = regression_metrics(&[1.0, 2.0, 3.0], &[1.0, 2.0, 5.0]);
        assert!(close(metrics.mae, 2.0 / 3.0));
        assert!(close(metrics.rmse, (4.0f64 / 3.0).sqrt()));
        assert!(close(metrics.r2, 1.0 - 4.0 / 2.0));
    }

    #[test]
    fn test_class_indices_rejects_unknown_label() {
        let labels = vec!["a".to_string()];
        let targets = vec![Target::Class("z".to_string())];
        assert!(class_indices(&targets, &labels).is_err());
    }
}
