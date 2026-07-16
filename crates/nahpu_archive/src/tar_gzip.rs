//! Creation and extraction of multi-file tar archives compressed with gzip.

use std::{
    fs::{self, File},
    io,
    path::{Component, Path, PathBuf},
};

use flate2::{Compression, read::GzDecoder, write::GzEncoder};

/// Creates a tar.gz archive from files below a common parent directory.
pub struct TarGzipArchive<'a> {
    parent_dir: &'a Path,
    files: &'a [PathBuf],
    output_path: &'a Path,
}

impl<'a> TarGzipArchive<'a> {
    pub fn new(parent_dir: &'a Path, output_path: &'a Path, files: &'a [PathBuf]) -> Self {
        Self {
            parent_dir,
            files,
            output_path,
        }
    }

    pub fn write(&self) -> io::Result<()> {
        let output = File::create(self.output_path)?;
        let encoder = GzEncoder::new(output, Compression::default());
        let mut archive = tar::Builder::new(encoder);
        let mut files = self.files.to_vec();
        files.sort();

        for file in files {
            let relative = safe_relative_path(self.parent_dir, &file)?;
            archive.append_path_with_name(&file, relative)?;
        }

        let encoder = archive.into_inner()?;
        encoder.finish()?;
        Ok(())
    }
}

/// Safely extracts a tar.gz archive below the requested destination.
pub struct TarGzipExtractor<'a> {
    archive_path: &'a Path,
    output_dir: &'a Path,
}

impl<'a> TarGzipExtractor<'a> {
    pub fn new(archive_path: &'a Path, output_dir: &'a Path) -> Self {
        Self {
            archive_path,
            output_dir,
        }
    }

    pub fn extract(&self) -> io::Result<()> {
        fs::create_dir_all(self.output_dir)?;
        let input = File::open(self.archive_path)?;
        let decoder = GzDecoder::new(input);
        let mut archive = tar::Archive::new(decoder);

        for entry in archive.entries()? {
            let mut entry = entry?;
            let relative = entry.path()?.into_owned();
            validate_relative_path(&relative)?;
            let target = self.output_dir.join(&relative);
            if !target.starts_with(self.output_dir) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "tar entry escapes destination",
                ));
            }
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            entry.unpack(target)?;
        }
        Ok(())
    }
}

fn safe_relative_path<'a>(parent: &Path, file: &'a Path) -> io::Result<&'a Path> {
    let relative = file.strip_prefix(parent).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "archive input is outside parent directory: {}",
                file.display()
            ),
        )
    })?;
    validate_relative_path(relative)?;
    Ok(relative)
}

fn validate_relative_path(path: &Path) -> io::Result<()> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unsafe archive path: {}", path.display()),
        ));
    }
    Ok(())
}
