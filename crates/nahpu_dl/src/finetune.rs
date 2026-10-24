use std::path::PathBuf;

use burn::module::Module;
use burn::nn::loss::{CrossEntropyLossConfig, MseLoss, Reduction};
use burn::optim::{AdamWConfig, GradientsParams};
use burn::tensor::{Device, Int, Tensor, TensorData};
use serde::{Deserialize, Serialize};

use crate::data::split::SplitMix64;
use crate::data::{self, DataSplit, LabeledDataset, Target};
use crate::evaluate::{EvalReport, Evaluator};
use crate::manifest::{
    Architecture, ModelManifest, Preprocessing, Provenance, TargetScaling, Task, WeightsInfo,
};
use crate::models::{DlModel, Head, Network};
use crate::predict::Predictor;
use crate::progress::{NoProgress, ProgressReporter};
use crate::{DlError, Result};

/// How much of the network is trained.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FineTuneStrategy {
    /// Train every layer.
    #[default]
    Full,
    /// Freeze the backbone and train only the head. Needs a base model.
    HeadOnly,
    /// Train only the head for the first half of the epochs, then every layer. Needs a base
    /// model.
    GradualUnfreeze,
}

/// Settings for [`FineTuner`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FineTuneConfig {
    /// Name written to the model manifest.
    pub name: String,
    /// What the model predicts.
    pub task: Task,
    /// Architecture for a model trained from scratch. Ignored when `base_model` is set.
    pub architecture: Option<Architecture>,
    /// Model package to start from.
    pub base_model: Option<PathBuf>,
    /// Which layers are trained.
    pub strategy: FineTuneStrategy,
    /// Maximum number of passes over the training data.
    pub epochs: usize,
    /// Samples per optimizer step.
    pub batch_size: usize,
    /// AdamW learning rate.
    pub learning_rate: f64,
    /// AdamW decoupled weight decay.
    pub weight_decay: f32,
    /// Fraction of the data held out for early stopping.
    pub valid_fraction: f64,
    /// Fraction of the data held out for the final test report.
    pub test_fraction: f64,
    /// Seed for splitting, shuffling, and weight initialization.
    pub seed: u64,
    /// Stop after this many epochs without a lower validation loss. `None` trains every epoch.
    pub patience: Option<usize>,
}

impl Default for FineTuneConfig {
    fn default() -> Self {
        Self {
            name: "model".to_string(),
            task: Task::Classification,
            architecture: None,
            base_model: None,
            strategy: FineTuneStrategy::Full,
            epochs: 50,
            batch_size: 32,
            learning_rate: 1e-3,
            weight_decay: 1e-4,
            valid_fraction: 0.15,
            test_fraction: 0.15,
            seed: 42,
            patience: Some(10),
        }
    }
}

/// Training statistics for one epoch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EpochStats {
    /// Epoch number, starting at 1.
    pub epoch: usize,
    /// Mean training loss.
    pub train_loss: f64,
    /// Validation loss, when there is a validation split.
    pub valid_loss: Option<f64>,
}

/// The result of fine-tuning.
#[derive(Debug, Clone)]
pub struct FineTuneOutcome {
    /// The model from the epoch with the lowest validation (or training) loss.
    pub model: DlModel,
    /// Statistics of every epoch that ran.
    pub history: Vec<EpochStats>,
    /// The epoch the returned model comes from.
    pub best_epoch: usize,
    /// How the data was split.
    pub split: DataSplit,
    /// Metrics on the test split, when there is one.
    pub test_report: Option<EvalReport>,
}

/// Fine-tunes a base model, or trains a new one, on a labeled dataset.
pub struct FineTuner<'a> {
    config: FineTuneConfig,
    progress: &'a dyn ProgressReporter,
}

impl<'a> FineTuner<'a> {
    /// Creates a fine-tuner.
    pub fn new(config: FineTuneConfig) -> Self {
        Self {
            config,
            progress: &NoProgress,
        }
    }

    /// Reports one step per epoch, plus a message with the epoch statistics.
    pub fn with_progress(mut self, progress: &'a dyn ProgressReporter) -> Self {
        self.progress = progress;
        self
    }

    /// Splits the data, trains, and evaluates on the test split.
    ///
    /// `device` is a plain (non-autodiff) device; gradients are tracked on its autodiff
    /// counterpart, and the returned model lives on `device`.
    ///
    /// # Errors
    ///
    /// Returns an error when the configuration is invalid, the data does not fit the model,
    /// or the base model cannot be loaded.
    pub fn fine_tune(&self, dataset: &LabeledDataset, device: &Device) -> Result<FineTuneOutcome> {
        self.check_config(dataset)?;
        let split = DataSplit::new(
            &dataset.targets,
            self.config.valid_fraction,
            self.config.test_fraction,
            self.config.seed,
        )?;
        let train_set = dataset.subset(&split.train);
        let valid_set = dataset.subset(&split.valid);
        device.seed(self.config.seed);
        let (manifest, network) = self.prepare_model(dataset, &train_set, device)?;
        let mut session = TrainingSession::new(&self.config, manifest, device);
        let (best, history, best_epoch) =
            session.run(network, &train_set, &valid_set, self.progress)?;
        let mut model = DlModel::from_parts(session.manifest, best, device);
        let test_report = self.test(&model, &dataset.subset(&split.test))?;
        if let Some(report) = &test_report {
            model.manifest_mut().metrics = report.summary();
        }
        Ok(FineTuneOutcome {
            model,
            history,
            best_epoch,
            split,
            test_report,
        })
    }

    fn check_config(&self, dataset: &LabeledDataset) -> Result<()> {
        let config = &self.config;
        if config.epochs == 0 || config.batch_size == 0 {
            return Err(DlError::Config(
                "epochs and batch size must be at least 1".to_string(),
            ));
        }
        if config.task == Task::Embedding {
            return Err(DlError::Config(
                "fine-tuning needs a classification or regression task".to_string(),
            ));
        }
        if config.base_model.is_none() && config.strategy != FineTuneStrategy::Full {
            return Err(DlError::Config(format!(
                "the {:?} strategy needs a base model",
                config.strategy
            )));
        }
        if dataset.is_empty() {
            return Err(DlError::Data("the training dataset is empty".to_string()));
        }
        if config.task == Task::Classification && dataset.class_labels().len() < 2 {
            return Err(DlError::Data(
                "classification needs at least two classes".to_string(),
            ));
        }
        Ok(())
    }

    /// Builds the manifest and the starting network on the autodiff device.
    fn prepare_model(
        &self,
        dataset: &LabeledDataset,
        train_set: &LabeledDataset,
        device: &Device,
    ) -> Result<(ModelManifest, Network)> {
        let autodiff = device.clone().autodiff();
        let labels = match self.config.task {
            Task::Classification => dataset.class_labels(),
            Task::Regression | Task::Embedding => Vec::new(),
        };
        let target = match self.config.task {
            Task::Regression => Some(target_scaling(&train_set.targets)),
            Task::Classification | Task::Embedding => None,
        };
        match &self.config.base_model {
            Some(path) => self.build_from_base(path, labels, target, train_set, &autodiff),
            None => self.build_new_model(labels, target, train_set, &autodiff),
        }
    }

    fn build_new_model(
        &self,
        labels: Vec<String>,
        target: Option<TargetScaling>,
        train_set: &LabeledDataset,
        autodiff: &Device,
    ) -> Result<(ModelManifest, Network)> {
        let architecture =
            self.config.architecture.clone().ok_or_else(|| {
                DlError::Config("set an architecture or a base model".to_string())
            })?;
        let manifest = ModelManifest {
            name: self.config.name.clone(),
            nahpu_dl_version: crate::VERSION.to_string(),
            modality: architecture.modality(),
            task: self.config.task,
            embedding_dim: architecture.embedding_dim(),
            architecture,
            labels,
            preprocessing: Preprocessing {
                tabular: Some(train_set.tabular_preprocessing()?),
                target,
            },
            weights: WeightsInfo::default(),
            metrics: Default::default(),
            provenance: self.provenance(None),
        };
        let network = Network::init(&manifest, autodiff)?;
        Ok((manifest, network))
    }

    fn build_from_base(
        &self,
        path: &std::path::Path,
        labels: Vec<String>,
        target: Option<TargetScaling>,
        train_set: &LabeledDataset,
        autodiff: &Device,
    ) -> Result<(ModelManifest, Network)> {
        let base = DlModel::load(path, &autodiff.clone().inner())?;
        let base_manifest = base.manifest().clone();
        check_features(&base_manifest, train_set)?;
        let same_head = base_manifest.task == self.config.task && base_manifest.labels == labels;
        // Loaded weights are in inference mode; `train` re-enables gradient tracking.
        let mut network = base.network().clone().fork(autodiff).train();
        let mut manifest = ModelManifest {
            name: self.config.name.clone(),
            nahpu_dl_version: crate::VERSION.to_string(),
            task: self.config.task,
            labels,
            weights: WeightsInfo::default(),
            metrics: Default::default(),
            provenance: self.provenance(Some(path.display().to_string())),
            ..base_manifest
        };
        // The backbone was trained with the base model's feature scaling, so it is kept.
        manifest.preprocessing.target = target;
        if !same_head {
            let output_dim = manifest.output_dim().unwrap_or(1);
            network.head = Some(Head::new(manifest.embedding_dim, output_dim, autodiff));
        }
        Ok((manifest, network))
    }

    fn provenance(&self, base_model: Option<String>) -> Provenance {
        Provenance {
            base_model,
            seed: Some(self.config.seed),
            ..Default::default()
        }
    }

    fn test(&self, model: &DlModel, test_set: &LabeledDataset) -> Result<Option<EvalReport>> {
        if test_set.is_empty() {
            return Ok(None);
        }
        let report = Evaluator::new(model)
            .with_batch_size(self.config.batch_size)
            .evaluate(test_set)?;
        Ok(Some(report))
    }
}

/// State of one training run.
struct TrainingSession<'a> {
    config: &'a FineTuneConfig,
    manifest: ModelManifest,
    device: Device,
    autodiff: Device,
}

impl<'a> TrainingSession<'a> {
    fn new(config: &'a FineTuneConfig, manifest: ModelManifest, device: &Device) -> Self {
        Self {
            config,
            manifest,
            device: device.clone(),
            autodiff: device.clone().autodiff(),
        }
    }

    /// Trains and returns the best network (on the plain device), the history, and the
    /// best epoch.
    fn run(
        &mut self,
        mut network: Network,
        train_set: &LabeledDataset,
        valid_set: &LabeledDataset,
        progress: &dyn ProgressReporter,
    ) -> Result<(Network, Vec<EpochStats>, usize)> {
        let mut optimizer = AdamWConfig::new()
            .with_weight_decay(self.config.weight_decay)
            .init();
        let mut rng = SplitMix64::new(self.config.seed);
        let mut order: Vec<usize> = (0..train_set.len()).collect();
        let mut history = Vec::with_capacity(self.config.epochs);
        let mut best: Option<(f64, usize, Network)> = None;
        progress.start("fine-tuning", self.config.epochs);
        for epoch in 1..=self.config.epochs {
            network = self.apply_strategy(network, epoch);
            rng.shuffle(&mut order);
            let mut loss_sum = 0.0;
            for batch in order.chunks(self.config.batch_size) {
                let batch_set = train_set.subset(batch);
                let loss = self.batch_loss(&network, &batch_set)?;
                loss_sum += scalar(&loss)? * batch.len() as f64;
                let grads = GradientsParams::from_grads(loss.backward(), &network);
                network = optimizer.step(self.config.learning_rate, network, grads);
            }
            let train_loss = loss_sum / train_set.len() as f64;
            let current = network.valid();
            let valid_loss = self.validation_loss(&current, valid_set)?;
            let stats = EpochStats {
                epoch,
                train_loss,
                valid_loss,
            };
            progress.message(&format_stats(&stats));
            progress.advance(1);
            history.push(stats);
            let score = valid_loss.unwrap_or(train_loss);
            if best
                .as_ref()
                .is_none_or(|(best_score, _, _)| score < *best_score)
            {
                best = Some((score, epoch, current));
            }
            if self.should_stop(epoch, best.as_ref().map(|(_, e, _)| *e)) {
                break;
            }
        }
        progress.finish();
        let (_, best_epoch, best_network) =
            best.ok_or_else(|| DlError::Config("no epoch was run".to_string()))?;
        Ok((best_network, history, best_epoch))
    }

    fn apply_strategy(&self, network: Network, epoch: usize) -> Network {
        let freeze_backbone = match self.config.strategy {
            FineTuneStrategy::Full => false,
            FineTuneStrategy::HeadOnly => true,
            FineTuneStrategy::GradualUnfreeze => epoch <= self.config.epochs.div_ceil(2),
        };
        let backbone = if freeze_backbone {
            network.backbone.freeze()
        } else {
            network.backbone.unfreeze()
        };
        Network {
            backbone,
            ..network
        }
    }

    fn should_stop(&self, epoch: usize, best_epoch: Option<usize>) -> bool {
        match (self.config.patience, best_epoch) {
            (Some(patience), Some(best_epoch)) => epoch - best_epoch >= patience,
            _ => false,
        }
    }

    fn batch_loss(&self, network: &Network, batch: &LabeledDataset) -> Result<Tensor<1>> {
        let input = data::encode_batch(&self.manifest, &batch.samples, &self.autodiff)?;
        let output = network
            .forward(input)
            .ok_or_else(|| DlError::Config("the network has no head to train".to_string()))?;
        match self.manifest.task {
            Task::Classification => {
                let targets = self.class_targets(&batch.targets)?;
                let loss = CrossEntropyLossConfig::new().init(&self.autodiff);
                Ok(loss.forward(output, targets))
            }
            Task::Regression => {
                let targets = self.value_targets(&batch.targets)?;
                Ok(MseLoss::new().forward(output, targets, Reduction::Mean))
            }
            Task::Embedding => Err(DlError::Config(
                "embedding models have no loss to train".to_string(),
            )),
        }
    }

    fn class_targets(&self, targets: &[Target]) -> Result<Tensor<1, Int>> {
        let indices = targets
            .iter()
            .map(|target| match target {
                Target::Class(label) => self
                    .manifest
                    .labels
                    .iter()
                    .position(|l| l == label)
                    .map(|i| i as i64)
                    .ok_or_else(|| DlError::Data(format!("unknown label `{label}`"))),
                Target::Value(_) => Err(DlError::Data(
                    "classification needs class labels".to_string(),
                )),
            })
            .collect::<Result<Vec<i64>>>()?;
        let n = indices.len();
        Ok(Tensor::from_data(
            TensorData::new(indices, [n]),
            &self.autodiff,
        ))
    }

    fn value_targets(&self, targets: &[Target]) -> Result<Tensor<2>> {
        let scaling = self.manifest.preprocessing.target.unwrap_or_default();
        let values = targets
            .iter()
            .map(|target| match target {
                Target::Value(value) => Ok(scaling.scale(*value)),
                Target::Class(_) => Err(DlError::Data(
                    "regression needs numeric targets".to_string(),
                )),
            })
            .collect::<Result<Vec<f32>>>()?;
        let n = values.len();
        Ok(Tensor::from_data(
            TensorData::new(values, [n, 1]),
            &self.autodiff,
        ))
    }

    /// Mean validation loss computed from model outputs, or `None` without a validation set.
    fn validation_loss(
        &self,
        network: &Network,
        valid_set: &LabeledDataset,
    ) -> Result<Option<f64>> {
        if valid_set.is_empty() {
            return Ok(None);
        }
        let model = DlModel::from_parts(self.manifest.clone(), network.clone(), &self.device);
        let rows = Predictor::new(&model)
            .with_batch_size(self.config.batch_size)
            .predict_rows(&valid_set.samples)?;
        let losses = valid_set
            .targets
            .iter()
            .zip(&rows)
            .map(|(target, row)| self.sample_loss(target, row))
            .collect::<Result<Vec<f64>>>()?;
        Ok(Some(losses.iter().sum::<f64>() / losses.len() as f64))
    }

    fn sample_loss(&self, target: &Target, row: &[f32]) -> Result<f64> {
        match target {
            Target::Class(label) => {
                let index = self
                    .manifest
                    .labels
                    .iter()
                    .position(|l| l == label)
                    .ok_or_else(|| DlError::Data(format!("unknown label `{label}`")))?;
                // Rows are already probabilities; clamp to keep the log finite.
                Ok(-f64::from(row[index]).max(1e-12).ln())
            }
            Target::Value(value) => {
                // Predictions come back in original units; compare on the training scale.
                let scaling = self.manifest.preprocessing.target.unwrap_or_default();
                let error = scaling.scale(row[0]) - scaling.scale(*value);
                Ok(f64::from(error * error))
            }
        }
    }
}

fn scalar(loss: &Tensor<1>) -> Result<f64> {
    loss.clone()
        .into_data()
        .try_into_vec_as::<f64>()
        .map_err(|e| DlError::Data(format!("cannot read the loss: {e:?}")))?
        .first()
        .copied()
        .ok_or_else(|| DlError::Data("the loss is empty".to_string()))
}

fn format_stats(stats: &EpochStats) -> String {
    match stats.valid_loss {
        Some(valid) => format!(
            "epoch {}: train loss {:.4}, valid loss {:.4}",
            stats.epoch, stats.train_loss, valid
        ),
        None => format!("epoch {}: train loss {:.4}", stats.epoch, stats.train_loss),
    }
}

fn target_scaling(targets: &[Target]) -> TargetScaling {
    let values: Vec<f64> = targets
        .iter()
        .filter_map(|target| match target {
            Target::Value(value) => Some(f64::from(*value)),
            Target::Class(_) => None,
        })
        .collect();
    let n = values.len().max(1) as f64;
    let mean = values.iter().sum::<f64>() / n;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    TargetScaling {
        mean: mean as f32,
        std: variance.sqrt() as f32,
    }
}

fn check_features(manifest: &ModelManifest, train_set: &LabeledDataset) -> Result<()> {
    let expected = manifest
        .preprocessing
        .tabular
        .as_ref()
        .map(|tabular| tabular.feature_names.as_slice())
        .unwrap_or_default();
    if expected != train_set.feature_names.as_slice() {
        return Err(DlError::Data(format!(
            "the base model expects features [{}] but the data has [{}]",
            expected.join(", "),
            train_set.feature_names.join(", ")
        )));
    }
    Ok(())
}
