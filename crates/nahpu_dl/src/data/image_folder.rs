use std::fs;
use std::path::{Path, PathBuf};

use crate::data::image::is_image_path;
use crate::data::{LabeledDataset, Sample, SampleInput, Target};
use crate::{DlError, Result};

/// Reads images from a directory.
///
/// For labeled data, each sub-directory of `root` is a class and its images (searched
/// recursively) are the samples:
///
/// ```text
/// root/
/// ├── Mus/        img001.jpg, img002.jpg, ...
/// └── Rattus/     img101.jpg, ...
/// ```
///
/// Sample ids are paths relative to `root`, with `/` separators on every platform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageFolder {
    /// Root directory, or a single image file for unlabeled reads.
    pub root: PathBuf,
}

impl ImageFolder {
    /// Creates a reader for `root`.
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    /// Reads labeled images, one class per sub-directory.
    ///
    /// # Errors
    ///
    /// Returns an error when `root` cannot be read or contains no class directory with
    /// images.
    pub fn read_labeled(&self) -> Result<LabeledDataset> {
        let mut dataset = LabeledDataset::default();
        for class_dir in sorted_entries(&self.root)?
            .into_iter()
            .filter(|p| p.is_dir())
        {
            let label = file_name(&class_dir);
            for path in find_images(&class_dir)? {
                dataset.samples.push(self.sample(path));
                dataset.targets.push(Target::Class(label.clone()));
            }
        }
        if dataset.is_empty() {
            return Err(DlError::Data(format!(
                "no images found in class sub-directories of {}",
                self.root.display()
            )));
        }
        Ok(dataset)
    }

    /// Reads unlabeled images: every image under `root`, or `root` itself if it is a file.
    ///
    /// # Errors
    ///
    /// Returns an error when `root` cannot be read or holds no images.
    pub fn read_samples(&self) -> Result<Vec<Sample>> {
        if self.root.is_file() {
            let id = file_name(&self.root);
            return Ok(vec![Sample::new(
                id,
                SampleInput::ImagePath(self.root.clone()),
            )]);
        }
        let samples: Vec<Sample> = find_images(&self.root)?
            .into_iter()
            .map(|path| self.sample(path))
            .collect();
        if samples.is_empty() {
            return Err(DlError::Data(format!(
                "no images found in {}",
                self.root.display()
            )));
        }
        Ok(samples)
    }

    fn sample(&self, path: PathBuf) -> Sample {
        let relative = path.strip_prefix(&self.root).unwrap_or(&path);
        let id = relative
            .components()
            .map(|part| part.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        Sample::new(id, SampleInput::ImagePath(path))
    }
}

/// Image files under `dir`, recursively, in sorted order.
pub(crate) fn find_images(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut images = Vec::new();
    for path in sorted_entries(dir)? {
        if path.is_dir() {
            images.extend(find_images(&path)?);
        } else if is_image_path(&path) {
            images.push(path);
        }
    }
    Ok(images)
}

/// Directory entries sorted by name, so datasets are read in the same order everywhere.
/// Hidden entries are skipped.
fn sorted_entries(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut entries = fs::read_dir(dir)
        .map_err(|e| DlError::io(dir, e))?
        .map(|entry| entry.map(|e| e.path()).map_err(|e| DlError::io(dir, e)))
        .collect::<Result<Vec<PathBuf>>>()?;
    entries.retain(|path| !file_name(path).starts_with('.'));
    entries.sort();
    Ok(entries)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch_image(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        image::RgbImage::new(4, 4).save(path).unwrap();
    }

    #[test]
    fn test_read_labeled_uses_class_directories() {
        let dir = tempfile::tempdir().unwrap();
        touch_image(&dir.path().join("Rattus/b.png"));
        touch_image(&dir.path().join("Mus/nested/a.png"));
        touch_image(&dir.path().join("Mus/c.jpg"));
        fs::write(dir.path().join("Mus/notes.txt"), "x").unwrap();
        fs::write(dir.path().join("README.md"), "x").unwrap();
        let dataset = ImageFolder::new(dir.path()).read_labeled().unwrap();
        let ids: Vec<&str> = dataset.samples.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, vec!["Mus/c.jpg", "Mus/nested/a.png", "Rattus/b.png"]);
        assert_eq!(dataset.class_labels(), vec!["Mus", "Rattus"]);
    }

    #[test]
    fn test_read_samples_from_file_and_directory() {
        let dir = tempfile::tempdir().unwrap();
        touch_image(&dir.path().join("x/a.png"));
        touch_image(&dir.path().join("b.webp"));
        let samples = ImageFolder::new(dir.path()).read_samples().unwrap();
        assert_eq!(samples.len(), 2);
        let single = ImageFolder::new(dir.path().join("b.webp"))
            .read_samples()
            .unwrap();
        assert_eq!(single[0].id, "b.webp");
    }

    #[test]
    fn test_empty_folder_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(ImageFolder::new(dir.path()).read_labeled().is_err());
        assert!(ImageFolder::new(dir.path()).read_samples().is_err());
    }
}
