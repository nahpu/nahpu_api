//! ZIP archive creation and safe extraction.

use std::{
    fs::{self, File},
    io::{self, BufReader},
    path::{Path, PathBuf},
};

use zip::write::FileOptions;

/// Creates a ZIP archive from an explicit list of files.
pub struct ZipArchive<'a> {
    pub parent_dir: &'a Path,
    pub alt_parent_dir: Option<&'a Path>,
    pub files: &'a [PathBuf],
    pub output_path: &'a Path,
}

impl<'a> ZipArchive<'a> {
    pub fn new(
        parent_dir: &'a Path,
        alt_parent_dir: Option<&'a Path>,
        output_path: &'a Path,
        files: &'a [PathBuf],
    ) -> Self {
        Self {
            parent_dir,
            alt_parent_dir,
            files,
            output_path,
        }
    }

    pub fn write(&self) -> io::Result<()> {
        let output = File::create(self.output_path)?;
        let mut zip = zip::ZipWriter::new(output);
        let options: FileOptions<()> =
            FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        let mut files = self.files.to_vec();
        files.sort();
        for file in files {
            let archive_path = self.parse_file_path(&file)?;
            let input = File::open(&file)?;
            zip.start_file(archive_path, options)?;
            io::copy(&mut BufReader::new(input), &mut zip)?;
        }
        zip.finish()?;
        Ok(())
    }

    fn parse_file_path(&self, file: &Path) -> io::Result<String> {
        let file_name = file.file_name().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "archive input has no file name",
            )
        })?;
        let relative = file.strip_prefix(self.parent_dir).or_else(|_| {
            self.alt_parent_dir
                .and_then(|parent| file.strip_prefix(parent).ok())
                .ok_or(())
        });
        let relative = relative.unwrap_or(Path::new(file_name));
        let value = relative.to_str().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "archive path is not UTF-8")
        })?;
        Ok(value.replace('\\', "/"))
    }
}

/// Safely extracts a ZIP archive. Entries that escape the destination are rejected.
pub struct ZipExtractor<'a> {
    pub archive_path: &'a Path,
    pub output_dir: &'a Path,
}

impl<'a> ZipExtractor<'a> {
    pub fn new(archive_path: &'a Path, output_dir: &'a Path) -> Self {
        Self {
            archive_path,
            output_dir,
        }
    }

    pub fn extract(&self) -> io::Result<()> {
        let mut archive = zip::ZipArchive::new(File::open(self.archive_path)?)?;
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index)?;
            let relative = entry.enclosed_name().ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "ZIP entry escapes destination")
            })?;
            let target = self.output_dir.join(relative);
            if entry.is_dir() {
                fs::create_dir_all(&target)?;
            } else {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut output = File::create(target)?;
                io::copy(&mut entry, &mut output)?;
            }
        }
        Ok(())
    }
}
