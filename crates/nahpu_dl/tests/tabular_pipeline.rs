//! End-to-end check of the tabular workflow: train, evaluate, save, reload, predict, embed.
#![cfg(all(feature = "train", feature = "flex"))]

use std::path::Path;

use nahpu_dl::data::TabularCsv;
use nahpu_dl::{
    Architecture, DeviceKind, DlModel, Embedder, EvalReport, Evaluator, FineTuneConfig,
    FineTuneStrategy, FineTuner, PredictionOutput, Predictor, Task, WeightFormat,
};

/// Two well-separated clusters, with a few missing values.
fn write_classification_csv(path: &Path) {
    let mut text = String::from("catalog,length,mass,species\n");
    for i in 0..60 {
        let jitter = (i % 7) as f32 * 0.1;
        let (length, mass, species) = if i % 2 == 0 {
            (10.0 + jitter, 20.0 - jitter, "Rattus")
        } else {
            (30.0 - jitter, 60.0 + jitter, "Mus")
        };
        let mass = if i % 13 == 0 {
            "NA".to_string()
        } else {
            mass.to_string()
        };
        text.push_str(&format!("C{i},{length},{mass},{species}\n"));
    }
    std::fs::write(path, text).unwrap();
}

fn write_regression_csv(path: &Path) {
    let mut text = String::from("length,mass\n");
    for i in 0..80 {
        let length = i as f32 * 0.5;
        text.push_str(&format!("{length},{}\n", 3.0 * length + 100.0));
    }
    std::fs::write(path, text).unwrap();
}

fn mlp() -> Architecture {
    Architecture::Mlp {
        input_dim: 2,
        hidden: vec![16, 8],
        dropout: 0.0,
    }
}

#[test]
fn test_classification_train_save_reload_predict_embed() {
    let dir = tempfile::tempdir().unwrap();
    let csv_path = dir.path().join("skulls.csv");
    write_classification_csv(&csv_path);
    let dataset = TabularCsv::new(&csv_path)
        .with_id_column("catalog")
        .with_target_column("species")
        .read_labeled(Task::Classification)
        .unwrap();
    let device = DeviceKind::Cpu.to_device().unwrap();

    let config = FineTuneConfig {
        name: "skull_species".to_string(),
        architecture: Some(mlp()),
        epochs: 40,
        batch_size: 8,
        learning_rate: 1e-2,
        ..Default::default()
    };
    let outcome = FineTuner::new(config).fine_tune(&dataset, &device).unwrap();
    let Some(EvalReport::Classification(metrics)) = &outcome.test_report else {
        panic!("expected a classification test report");
    };
    assert!(metrics.accuracy >= 0.9, "accuracy {}", metrics.accuracy);
    assert_eq!(outcome.model.manifest().labels, vec!["Mus", "Rattus"]);

    let mut model = outcome.model;
    let samples = &dataset.samples[..6];
    let expected = Predictor::new(&model).predict(samples).unwrap();

    for format in [WeightFormat::Burnpack, WeightFormat::Safetensors] {
        let package = dir.path().join(format.extension());
        let weights = model.save(&package, format).unwrap();
        assert!(weights.is_file());
        let reloaded = DlModel::load(&package, &device).unwrap();
        let predictions = Predictor::new(&reloaded).predict(samples).unwrap();
        assert_eq!(
            predictions, expected,
            "{format:?} reload changed predictions"
        );
    }

    let PredictionOutput::Class { top_k, .. } = &expected[0].output else {
        panic!("expected a class prediction");
    };
    assert_eq!(top_k.len(), 2);

    let embeddings = Embedder::new(&model).embed(samples).unwrap();
    assert_eq!(embeddings.len(), 6);
    assert_eq!(embeddings[0].id, "C0");
    assert_eq!(embeddings[0].vector.len(), 8);
}

#[test]
fn test_regression_learns_linear_relation() {
    let dir = tempfile::tempdir().unwrap();
    let csv_path = dir.path().join("mass.csv");
    write_regression_csv(&csv_path);
    let dataset = TabularCsv::new(&csv_path)
        .with_target_column("mass")
        .read_labeled(Task::Regression)
        .unwrap();
    let device = DeviceKind::Cpu.to_device().unwrap();
    let config = FineTuneConfig {
        task: Task::Regression,
        architecture: Some(Architecture::Mlp {
            input_dim: 1,
            hidden: vec![16],
            dropout: 0.0,
        }),
        epochs: 80,
        batch_size: 8,
        learning_rate: 1e-2,
        ..Default::default()
    };
    let outcome = FineTuner::new(config).fine_tune(&dataset, &device).unwrap();
    let report = Evaluator::new(&outcome.model).evaluate(&dataset).unwrap();
    let EvalReport::Regression(metrics) = report else {
        panic!("expected a regression report");
    };
    assert!(metrics.r2 > 0.95, "r2 {}", metrics.r2);
}

#[test]
fn test_head_only_fine_tune_from_base_model() {
    let dir = tempfile::tempdir().unwrap();
    let csv_path = dir.path().join("skulls.csv");
    write_classification_csv(&csv_path);
    let dataset = TabularCsv::new(&csv_path)
        .with_target_column("species")
        .with_id_column("catalog")
        .read_labeled(Task::Classification)
        .unwrap();
    let device = DeviceKind::Cpu.to_device().unwrap();
    let base_config = FineTuneConfig {
        architecture: Some(mlp()),
        epochs: 5,
        ..Default::default()
    };
    let mut base = FineTuner::new(base_config)
        .fine_tune(&dataset, &device)
        .unwrap()
        .model;
    let base_dir = dir.path().join("base");
    base.save(&base_dir, WeightFormat::Burnpack).unwrap();

    let config = FineTuneConfig {
        base_model: Some(base_dir.clone()),
        strategy: FineTuneStrategy::HeadOnly,
        epochs: 5,
        ..Default::default()
    };
    let outcome = FineTuner::new(config).fine_tune(&dataset, &device).unwrap();
    let base_backbone = Embedder::new(&base).embed(&dataset.samples[..3]).unwrap();
    let tuned_backbone = Embedder::new(&outcome.model)
        .embed(&dataset.samples[..3])
        .unwrap();
    assert_eq!(
        base_backbone, tuned_backbone,
        "head-only training changed the backbone"
    );
    assert_eq!(
        outcome.model.manifest().provenance.base_model.as_deref(),
        Some(base_dir.display().to_string().as_str())
    );
}

#[test]
fn test_head_only_without_base_model_is_rejected() {
    let config = FineTuneConfig {
        architecture: Some(mlp()),
        strategy: FineTuneStrategy::HeadOnly,
        ..Default::default()
    };
    let dataset = nahpu_dl::LabeledDataset::default();
    let device = DeviceKind::Cpu.to_device().unwrap();
    assert!(FineTuner::new(config).fine_tune(&dataset, &device).is_err());
}
