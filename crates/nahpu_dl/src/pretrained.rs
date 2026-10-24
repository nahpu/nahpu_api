//! Import of ImageNet-pretrained weights from torchvision and timm checkpoints.

use std::path::{Path, PathBuf};

use burn::module::Module;
use burn::store::{
    KeyRemapper, ModuleSnapshot, PyTorchToBurnAdapter, PytorchStore, SafetensorsStore,
};
use burn::tensor::Device;

use crate::manifest::{
    Architecture, ImagePreprocessing, Modality, ModelManifest, Preprocessing, Provenance, Task,
    WeightsInfo,
};
use crate::models::{Backbone, DlModel, Network};
use crate::store::check_apply_result;
use crate::{DlError, Result};

/// Number of ImageNet-1k classes.
pub const IMAGENET_CLASSES: usize = 1000;

/// Builds a model package from a torchvision or timm checkpoint.
///
/// Supported checkpoints are `.safetensors` files and PyTorch `.pth`, `.pt`, or `.bin` state
/// dicts for ResNet (torchvision or timm names) and MobileNetV2 (torchvision or timm
/// `mobilenetv2_100` names).
#[derive(Debug, Clone, PartialEq)]
pub struct PretrainedImport {
    /// Network architecture of the checkpoint.
    pub architecture: Architecture,
    /// Checkpoint file.
    pub weights: PathBuf,
    /// Name written to the manifest.
    pub name: String,
    /// Keep the ImageNet classifier, with these labels in output order. Without it the
    /// package is an embedding model that can be fine-tuned with a new head.
    pub classifier_labels: Option<Vec<String>>,
    /// Image preprocessing the checkpoint was trained with.
    pub preprocessing: ImagePreprocessing,
    /// Where the checkpoint came from, for the manifest.
    pub source: Option<String>,
}

impl PretrainedImport {
    /// Imports a checkpoint as an embedding model with ImageNet preprocessing.
    pub fn new<P: AsRef<Path>>(architecture: Architecture, weights: P) -> Self {
        Self {
            architecture,
            weights: weights.as_ref().to_path_buf(),
            name: "pretrained".to_string(),
            classifier_labels: None,
            preprocessing: ImagePreprocessing::default(),
            source: None,
        }
    }

    /// Sets the model name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Keeps the ImageNet classifier. Labels default to class indices `0`..`999`.
    pub fn with_classifier(mut self, labels: Option<Vec<String>>) -> Self {
        let labels =
            labels.unwrap_or_else(|| (0..IMAGENET_CLASSES).map(|i| i.to_string()).collect());
        self.classifier_labels = Some(labels);
        self
    }

    /// Sets the image preprocessing.
    pub fn with_preprocessing(mut self, preprocessing: ImagePreprocessing) -> Self {
        self.preprocessing = preprocessing;
        self
    }

    /// Records where the checkpoint came from.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Reads the checkpoint into a model on `device`.
    ///
    /// # Errors
    ///
    /// Returns an error when the architecture is not an image backbone, the file format is
    /// unknown, or the checkpoint lacks tensors the network needs.
    pub fn import(&self, device: &Device) -> Result<DlModel> {
        if self.architecture.modality() != Modality::Image {
            return Err(DlError::Config(
                "pretrained import supports image architectures only".to_string(),
            ));
        }
        let manifest = self.build_manifest();
        let mut network = Network::init(&manifest, device)?;
        self.load_backbone(&mut network.backbone)?;
        if let Some(head) = &mut network.head {
            load_into(head, &self.weights, &self.head_remapper()?)?;
        }
        Ok(DlModel::from_parts(manifest, network, device))
    }

    fn build_manifest(&self) -> ModelManifest {
        let (task, labels) = match &self.classifier_labels {
            Some(labels) => (Task::Classification, labels.clone()),
            None => (Task::Embedding, Vec::new()),
        };
        ModelManifest {
            name: self.name.clone(),
            nahpu_dl_version: crate::VERSION.to_string(),
            modality: Modality::Image,
            task,
            embedding_dim: self.architecture.embedding_dim(),
            architecture: self.architecture.clone(),
            labels,
            preprocessing: Preprocessing {
                image: Some(self.preprocessing),
                ..Default::default()
            },
            weights: WeightsInfo::default(),
            metrics: Default::default(),
            provenance: Provenance {
                base_model: Some(
                    self.source
                        .clone()
                        .unwrap_or_else(|| self.weights.display().to_string()),
                ),
                ..Default::default()
            },
        }
    }

    fn load_backbone(&self, backbone: &mut Backbone) -> Result<()> {
        match backbone {
            Backbone::ResNet(resnet) => load_into(resnet, &self.weights, &resnet_remapper()?),
            Backbone::MobileNetV2(mobilenet) => {
                load_into(mobilenet, &self.weights, &mobilenet_remapper()?)
            }
            Backbone::Mlp(_) => Err(DlError::Config(
                "pretrained import supports image architectures only".to_string(),
            )),
        }
    }

    /// Maps the checkpoint classifier onto [`crate::models::Head`], whose layer is `linear`.
    fn head_remapper(&self) -> Result<KeyRemapper> {
        let pattern = match self.architecture {
            Architecture::ResNet { .. } => r"^fc\.",
            // torchvision wraps the layer in Sequential(Dropout, Linear); timm does not.
            Architecture::MobileNetV2 => r"^classifier\.(1\.)?",
            Architecture::Mlp { .. } => return Ok(KeyRemapper::new()),
        };
        remapper([(pattern.to_string(), "linear.".to_string())])
    }
}

/// Loads checkpoint tensors into `module`, whose parameter paths are the checkpoint names
/// after remapping. Extra checkpoint tensors (the classifier, batch counters) are ignored.
fn load_into<M: Module>(module: &mut M, path: &Path, remap: &KeyRemapper) -> Result<()> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase);
    let result = match extension.as_deref() {
        Some("safetensors") => {
            let mut store = SafetensorsStore::from_file(path)
                .with_from_adapter(PyTorchToBurnAdapter)
                .skip_enum_variants(true)
                .remap(remap.clone());
            module
                .load_from(&mut store)
                .map_err(|e| DlError::store(path, e))?
        }
        Some("pth" | "pt" | "bin") => {
            let mut store = PytorchStore::from_file(path).remap(remap.clone());
            module
                .load_from(&mut store)
                .map_err(|e| DlError::store(path, e))?
        }
        _ => {
            return Err(DlError::store(
                path,
                "expected a .safetensors, .pth, .pt, or .bin checkpoint",
            ));
        }
    };
    check_apply_result(&result, path)
}

/// torchvision and timm share ResNet names except for the shortcut, a `Sequential`.
fn resnet_remapper() -> Result<KeyRemapper> {
    remapper([
        (
            r"\.downsample\.0\.".to_string(),
            ".downsample.conv.".to_string(),
        ),
        (
            r"\.downsample\.1\.".to_string(),
            ".downsample.bn.".to_string(),
        ),
    ])
}

/// Maps torchvision MobileNetV2 names (`features.N...`) onto the timm names the network
/// uses. timm checkpoints pass through unchanged.
fn mobilenet_remapper() -> Result<KeyRemapper> {
    let mut rules = vec![
        (r"^features\.0\.0\.".to_string(), "conv_stem.".to_string()),
        (r"^features\.0\.1\.".to_string(), "bn1.".to_string()),
        (
            r"^features\.1\.conv\.0\.0\.".to_string(),
            "blocks.0.0.conv_dw.".to_string(),
        ),
        (
            r"^features\.1\.conv\.0\.1\.".to_string(),
            "blocks.0.0.bn1.".to_string(),
        ),
        (
            r"^features\.1\.conv\.1\.".to_string(),
            "blocks.0.0.conv_pw.".to_string(),
        ),
        (
            r"^features\.1\.conv\.2\.".to_string(),
            "blocks.0.0.bn2.".to_string(),
        ),
        (r"^features\.18\.0\.".to_string(), "conv_head.".to_string()),
        (r"^features\.18\.1\.".to_string(), "bn2.".to_string()),
    ];
    // Blocks per stage after the first, from the MobileNetV2 paper.
    let stage_blocks = [2, 3, 4, 3, 3, 1];
    let mut feature = 2;
    for (stage, &n_blocks) in stage_blocks.iter().enumerate() {
        for block in 0..n_blocks {
            let target = format!("blocks.{}.{block}.", stage + 1);
            let parts = [
                ("conv\\.0\\.0", "conv_pw"),
                ("conv\\.0\\.1", "bn1"),
                ("conv\\.1\\.0", "conv_dw"),
                ("conv\\.1\\.1", "bn2"),
                ("conv\\.2", "conv_pwl"),
                ("conv\\.3", "bn3"),
            ];
            for (source, name) in parts {
                rules.push((
                    format!(r"^features\.{feature}\.{source}\."),
                    format!("{target}{name}."),
                ));
            }
            feature += 1;
        }
    }
    remapper(rules)
}

fn remapper(rules: impl IntoIterator<Item = (String, String)>) -> Result<KeyRemapper> {
    KeyRemapper::from_pattern_iter(rules)
        .map_err(|e| DlError::Config(format!("invalid key remapping pattern: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mobilenet_remapper_covers_all_torchvision_blocks() {
        // 1 stem + 17 blocks + 1 head; block 17 is the last inverted residual.
        let remapper = mobilenet_remapper().unwrap();
        let rules = remapper.to_regex_pairs();
        assert_eq!(rules.len(), 8 + 16 * 6);
        let (pattern, target) = rules
            .iter()
            .find(|(pattern, _)| pattern.is_match("features.17.conv.2.weight"))
            .unwrap();
        assert_eq!(
            pattern.replace("features.17.conv.2.weight", target.as_str()),
            "blocks.6.0.conv_pwl.weight"
        );
    }
}
