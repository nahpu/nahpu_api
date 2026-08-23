//! Creation and extraction of multi-file tar archives compressed with gzip.

use std::{
    fs::{self, File},
    io::{self, BufReader},
    path::{Component, Path, PathBuf},
};

use flate2::{Compression, read::GzDecoder, write::GzEncoder};

use crate::progress::{
    ProgressFn, ProgressReader, ProgressTracker, discard_progress, total_file_size,
};

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
        self.write_with_progress(&mut discard_progress)
    }

    /// Creates the archive, reporting each entry to `on_progress` while it is written.
    ///
    /// Progress is measured in uncompressed input bytes, which the caller can predict
    /// from the file sizes it already knows.
    pub fn write_with_progress(&self, on_progress: ProgressFn<'_>) -> io::Result<()> {
        let mut files = self.files.to_vec();
        files.sort();
        let mut tracker =
            ProgressTracker::new(on_progress, files.len() as u64, total_file_size(&files));

        let output = File::create(self.output_path)?;
        let encoder = GzEncoder::new(output, Compression::default());
        let mut archive = tar::Builder::new(encoder);

        for file in files {
            let relative = safe_relative_path(self.parent_dir, &file)?;
            tracker.start_entry(&relative.to_string_lossy());
            let mut header = tar::Header::new_gnu();
            header.set_metadata(&fs::metadata(&file)?);
            let input = File::open(&file)?;
            archive.append_data(
                &mut header,
                relative,
                ProgressReader::new(BufReader::new(input), &mut tracker),
            )?;
            tracker.finish_entry();
        }

        let encoder = archive.into_inner()?;
        encoder.finish()?;
        tracker.finish();
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
        self.extract_with_progress(&mut discard_progress)
    }

    /// Extracts the archive, reporting each entry to `on_progress` while it is unpacked.
    ///
    /// A tar stream carries no index, so the entry and byte totals stay unknown until the
    /// last entry. Callers report the running counts rather than a percentage.
    pub fn extract_with_progress(&self, on_progress: ProgressFn<'_>) -> io::Result<()> {
        fs::create_dir_all(self.output_dir)?;
        let input = File::open(self.archive_path)?;
        let decoder = GzDecoder::new(input);
        let mut archive = tar::Archive::new(decoder);
        let mut tracker = ProgressTracker::new(on_progress, 0, 0);

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
            tracker.start_entry(&relative.to_string_lossy());
            let size = entry.size();
            entry.unpack(target)?;
            tracker.add_bytes(size);
            tracker.finish_entry();
        }
        tracker.finish();
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
