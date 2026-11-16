//! End-to-end checks of the image workflow on small synthetic images.
#![cfg(all(feature = "train", feature = "flex"))]

use std::path::Path;

use nahpu_dl::data::ImageFolder;
use nahpu_dl::{
    Architecture, DeviceKind, DlModel, Embedder, EvalReport, FineTuneConfig, FineTuneStrategy,
    FineTuner, ImagePreprocessing, Modality, ModelManifest, Predictor, Preprocessing, Task,
    WeightFormat, WeightsInfo,
};

const IMAGE_SIZE: u32 = 32;

/// Reddish and bluish images with a little texture, `n` per class.
fn write_image_folder(root: &Path, n: u32) {
    for (class, base) in [("red", [200u8, 40, 40]), ("blue", [40u8, 40, 200])] {
        let dir = root.join(class);
        std::fs::create_dir_all(&dir).unwrap();
        for i in 0..n {
            let image = image::RgbImage::from_fn(48, 40, |x, y| {
                let noise = ((x * 7 + y * 13 + i * 31) % 40) as u8;
                image::Rgb([base[0] + noise, base[1] + noise, base[2] + noise])
            });
            image.save(dir.join(format!("{class}_{i}.png"))).unwrap();
        }
    }
}

/// An untrained ResNet-18 embedding package, standing in for a pretrained base model.
fn write_base_model(dir: &Path) {
    let architecture = Architecture::ResNet { depth: 18 };
    let manifest = ModelManifest {
        name: "base".to_string(),
        nahpu_dl_version: nahpu_dl::VERSION.to_string(),
        modality: Modality::Image,
        task: Task::Embedding,
        embedding_dim: architecture.embedding_dim(),
        architecture,
        labels: Vec::new(),
        preprocessing: Preprocessing {
            image: Some(ImagePreprocessing::default().with_size(IMAGE_SIZE)),
            ..Default::default()
        },
        weights: WeightsInfo::default(),
        metrics: Default::default(),
        provenance: Default::default(),
    };
    let device = DeviceKind::Cpu.to_device().unwrap();
    let mut model = DlModel::new(manifest, &device).unwrap();
    model.save(dir, WeightFormat::Burnpack).unwrap();
}

#[test]
fn test_head_only_fine_tune_on_image_folder() {
    let dir = tempfile::tempdir().unwrap();
    let images = dir.path().join("images");
    write_image_folder(&images, 12);
    let base = dir.path().join("base");
    write_base_model(&base);
    let dataset = ImageFolder::new(&images).read_labeled().unwrap();
    let device = DeviceKind::Cpu.to_device().unwrap();

    let config = FineTuneConfig {
        name: "colors".to_string(),
        base_model: Some(base),
        strategy: FineTuneStrategy::HeadOnly,
        epochs: 15,
        batch_size: 8,
        learning_rate: 1e-2,
        valid_fraction: 0.2,
        test_fraction: 0.2,
        ..Default::default()
    };
    let outcome = FineTuner::new(config).fine_tune(&dataset, &device).unwrap();
    let Some(EvalReport::Classification(metrics)) = &outcome.test_report else {
        panic!("expected a classification report");
    };
    assert!(metrics.accuracy >= 0.8, "accuracy {}", metrics.accuracy);

    let mut model = outcome.model;
    assert_eq!(model.manifest().labels, vec!["blue", "red"]);
    let samples = &dataset.samples[..4];
    let expected = Predictor::new(&model).predict(samples).unwrap();
    for format in [WeightFormat::Burnpack, WeightFormat::Safetensors] {
        let package = dir.path().join(format.extension());
        model.save(&package, format).unwrap();
        let reloaded = DlModel::load(&package, &device).unwrap();
        assert_eq!(
            Predictor::new(&reloaded).predict(samples).unwrap(),
            expected
        );
    }
    let embeddings = Embedder::new(&model).embed(samples).unwrap();
    assert_eq!(embeddings[0].vector.len(), 512);
    assert_eq!(embeddings[0].id, "blue/blue_0.png");
}

#[test]
fn test_full_training_from_scratch_runs() {
    let dir = tempfile::tempdir().unwrap();
    write_image_folder(dir.path(), 4);
    let dataset = ImageFolder::new(dir.path()).read_labeled().unwrap();
    let device = DeviceKind::Cpu.to_device().unwrap();
    let config = FineTuneConfig {
        architecture: Some(Architecture::MobileNetV2),
        image_size: Some(IMAGE_SIZE),
        epochs: 1,
        batch_size: 4,
        valid_fraction: 0.0,
        test_fraction: 0.0,
        ..Default::default()
    };
    let outcome = FineTuner::new(config).fine_tune(&dataset, &device).unwrap();
    assert_eq!(outcome.history.len(), 1);
    assert!(outcome.history[0].train_loss.is_finite());
    assert_eq!(
        outcome.model.manifest().preprocessing.image.unwrap().size,
        IMAGE_SIZE
    );
}
