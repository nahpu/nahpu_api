//! Pretrained models downloadable from the Hugging Face Hub (`hub` feature).

use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use burn::tensor::Device;

use crate::manifest::{Architecture, ImagePreprocessing, WeightFormat};
use crate::models::DlModel;
use crate::pretrained::PretrainedImport;
use crate::progress::ProgressReporter;
use crate::{DlError, Result};

const HUB_URL: &str = "https://huggingface.co";

/// Download progress is reported in steps of this many bytes.
const PROGRESS_STEP_BYTES: u64 = 1 << 20;

/// File name of the downloaded checkpoint, removed once the package is written.
const CHECKPOINT_FILE_NAME: &str = "checkpoint.safetensors";

/// An ImageNet-pretrained checkpoint on the Hugging Face Hub.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PretrainedModel {
    /// Short name used on the command line, such as `resnet50`.
    pub id: &'static str,
    /// Hub repository.
    pub repo: &'static str,
    /// Checkpoint file in the repository.
    pub file: &'static str,
    /// Center crop fraction the checkpoint was evaluated with.
    pub crop_fraction: f32,
    /// One-line description.
    pub description: &'static str,
}

/// Checkpoints `nahpu_dl` can download. All are timm models trained on ImageNet-1k.
pub const PRETRAINED_MODELS: [PretrainedModel; 4] = [
    PretrainedModel {
        id: "resnet18",
        repo: "timm/resnet18.a1_in1k",
        file: "model.safetensors",
        crop_fraction: 0.95,
        description: "ResNet-18, 11.7M parameters, 512-dimensional embeddings",
    },
    PretrainedModel {
        id: "resnet34",
        repo: "timm/resnet34.a1_in1k",
        file: "model.safetensors",
        crop_fraction: 0.95,
        description: "ResNet-34, 21.8M parameters, 512-dimensional embeddings",
    },
    PretrainedModel {
        id: "resnet50",
        repo: "timm/resnet50.a1_in1k",
        file: "model.safetensors",
        crop_fraction: 0.95,
        description: "ResNet-50, 25.6M parameters, 2048-dimensional embeddings",
    },
    PretrainedModel {
        id: "mobilenet_v2",
        repo: "timm/mobilenetv2_100.ra_in1k",
        file: "model.safetensors",
        crop_fraction: 0.875,
        description: "MobileNetV2, 3.5M parameters, 1280-dimensional embeddings; fast on phones",
    },
];

impl PretrainedModel {
    /// Looks up a checkpoint by id.
    pub fn find(id: &str) -> Option<Self> {
        PRETRAINED_MODELS
            .iter()
            .copied()
            .find(|model| model.id == id)
    }

    /// The network architecture of the checkpoint.
    pub fn architecture(&self) -> Architecture {
        match self.id {
            "resnet18" => Architecture::ResNet { depth: 18 },
            "resnet34" => Architecture::ResNet { depth: 34 },
            "resnet50" => Architecture::ResNet { depth: 50 },
            _ => Architecture::MobileNetV2,
        }
    }

    /// Download URL of the checkpoint.
    pub fn url(&self) -> String {
        format!("{HUB_URL}/{}/resolve/main/{}", self.repo, self.file)
    }

    /// Downloads the checkpoint and writes a model package to `dir`.
    ///
    /// The package is an embedding model; with `keep_classifier` it is an ImageNet classifier
    /// whose labels are class indices.
    ///
    /// # Errors
    ///
    /// Returns an error when the download fails or the package cannot be written.
    pub fn pull<P: AsRef<Path>>(
        &self,
        dir: P,
        keep_classifier: bool,
        device: &Device,
        progress: &dyn ProgressReporter,
    ) -> Result<DlModel> {
        let dir = dir.as_ref();
        fs::create_dir_all(dir).map_err(|e| DlError::io(dir, e))?;
        let checkpoint = dir.join(CHECKPOINT_FILE_NAME);
        download(&self.url(), &checkpoint, progress)?;
        let mut import = PretrainedImport::new(self.architecture(), &checkpoint)
            .with_name(self.id)
            .with_source(self.repo)
            .with_preprocessing(ImagePreprocessing {
                crop_fraction: self.crop_fraction,
                ..Default::default()
            });
        if keep_classifier {
            import = import.with_classifier(None);
        }
        let mut model = import.import(device)?;
        model.save(dir, WeightFormat::Burnpack)?;
        fs::remove_file(&checkpoint).map_err(|e| DlError::io(&checkpoint, e))?;
        Ok(model)
    }
}

/// Streams `url` to `path` through a temporary file, so an interrupted download never
/// leaves a truncated checkpoint behind.
///
/// # Errors
///
/// Returns an error when the request fails or the file cannot be written.
pub fn download(url: &str, path: &Path, progress: &dyn ProgressReporter) -> Result<PathBuf> {
    let response = ureq::get(url)
        .call()
        .map_err(|e| DlError::Data(format!("download of {url} failed: {e}")))?;
    let total = response
        .headers()
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());
    let partial = path.with_extension("part");
    let mut writer = BufWriter::new(File::create(&partial).map_err(|e| DlError::io(&partial, e))?);
    let mut reader = response.into_body().into_reader();
    progress.start(
        "download (MiB)",
        total.map_or(0, |t| t.div_ceil(PROGRESS_STEP_BYTES)) as usize,
    );
    let mut buffer = vec![0u8; 64 * 1024];
    let mut written = 0u64;
    loop {
        let n = reader
            .read(&mut buffer)
            .map_err(|e| DlError::io(&partial, e))?;
        if n == 0 {
            break;
        }
        writer
            .write_all(&buffer[..n])
            .map_err(|e| DlError::io(&partial, e))?;
        let steps = (written + n as u64) / PROGRESS_STEP_BYTES - written / PROGRESS_STEP_BYTES;
        progress.advance(steps as usize);
        written += n as u64;
    }
    writer.flush().map_err(|e| DlError::io(&partial, e))?;
    progress.finish();
    if total.is_some_and(|total| total != written) {
        return Err(DlError::Data(format!(
            "download of {url} stopped after {written} bytes"
        )));
    }
    fs::rename(&partial, path).map_err(|e| DlError::io(path, e))?;
    Ok(path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_lookup() {
        let model = PretrainedModel::find("resnet50").unwrap();
        assert_eq!(model.architecture(), Architecture::ResNet { depth: 50 });
        assert_eq!(
            model.url(),
            "https://huggingface.co/timm/resnet50.a1_in1k/resolve/main/model.safetensors"
        );
        assert_eq!(
            PretrainedModel::find("mobilenet_v2")
                .unwrap()
                .architecture(),
            Architecture::MobileNetV2
        );
        assert!(PretrainedModel::find("vgg16").is_none());
    }
}
