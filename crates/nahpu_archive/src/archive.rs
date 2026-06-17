//! An archive writer module.
//!
//! Compresses and decompresses files to and from zip archives.
use std::{
    io::BufReader,
    path::{Path, PathBuf},
};

use zip::write::FileOptions;

pub struct ZipArchive<'a> {
    /// The parent directory of the files to be archived.
    /// This is used to create the directory structure in the archive.
    pub parent_dir: &'a Path,
    /// Alternative parent directory to be used in the archive.
    /// This is used to create the directory structure
    /// if the parent directory is not present.
    pub alt_parent_dir: Option<&'a Path>,
    /// The files to be archived.
    pub files: &'a [PathBuf],
    /// The path to the output file.
    pub output_path: &'a Path,
}

/// A zip archive writer.
/// Writes the files to a zip archive.
/// The files are written to the archive with the directory structure
/// relative to the parent directory.
/// If the parent directory is not present, the alternative parent directory
/// is used to create the directory structure.
///
/// # Examples
/// ```no_run
/// use std::path::Path;
/// use nahpu_archive::archive::ZipArchive;
///
/// let parent_dir = Path::new("/source");
/// let output_path = Path::new("/out/archive.zip");
/// let files = vec![];
/// let archive = ZipArchive::new(parent_dir, None, output_path, &files);
/// ```
impl<'a> ZipArchive<'a> {
    /// Creates a new `ZipArchive` instance.
    ///
    /// # Examples
    /// ```no_run
    /// use std::path::Path;
    /// use nahpu_archive::archive::ZipArchive;
    ///
    /// let parent = Path::new("/src");
    /// let out = Path::new("archive.zip");
    /// let files = vec![];
    /// let archiver = ZipArchive::new(parent, None, out, &files);
    /// ```
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

    /// Writes the configured files into the zip archive.
    ///
    /// # Examples
    /// ```no_run
    /// use std::path::Path;
    /// use nahpu_archive::archive::ZipArchive;
    ///
    /// let archiver = ZipArchive::new(Path::new("/src"), None, Path::new("out.zip"), &[]);
    /// archiver.write().unwrap();
    /// ```
    pub fn write(&self) -> Result<(), std::io::Error> {
        let mut zip = zip::ZipWriter::new(std::fs::File::create(self.output_path).unwrap());
        let options: FileOptions<()> =
            FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        for file in self.files {
            let file_path = self.parse_file_path(file);
            let inner = std::fs::File::open(file)?;
            let mut buff = BufReader::new(inner);
            zip.start_file(file_path, options)?;

            std::io::copy(&mut buff, &mut zip)?;
        }

        zip.finish()?;
        Ok(())
    }

    fn parse_file_path(&self, file: &Path) -> String {
        let file_name = Path::new(file.file_name().unwrap_or_else(|| {
            panic!("Failed parsing file name: {:?}", file);
        }));
        let prefix = file.strip_prefix(self.parent_dir);
        let file_path = match prefix {
            Ok(p) => p,
            // If the file is not in the parent directory,
            // use the alternative parent directory.
            // Otherwise, use the file name.
            Err(_) => file
                .strip_prefix(self.alt_parent_dir.unwrap_or(file_name))
                .unwrap_or(file_name),
        };
        file_path
            .to_str()
            .expect("Failed parsing file path")
            .to_string()
    }
}

pub struct ZipExtractor<'a> {
    /// The path to the zip archive.
    pub archive_path: &'a Path,
    /// The directory to extract the files to.
    pub output_dir: &'a Path,
}

/// A zip archive extractor.
/// Extracts the files from a zip archive.
/// The files are extracted to the output directory.
/// The directory structure is preserved.
///
/// # Examples
/// ```no_run
/// use std::path::Path;
/// use nahpu_archive::archive::ZipExtractor;
///
/// let archive_path = Path::new("archive.zip");
/// let output_dir = Path::new("/extracted");
/// let extractor = ZipExtractor::new(archive_path, output_dir);
/// ```
impl<'a> ZipExtractor<'a> {
    /// Creates a new `ZipExtractor` instance.
    ///
    /// # Examples
    /// ```no_run
    /// use std::path::Path;
    /// use nahpu_archive::archive::ZipExtractor;
    ///
    /// let extractor = ZipExtractor::new(Path::new("archive.zip"), Path::new("/extracted"));
    /// ```
    pub fn new(archive_path: &'a Path, output_dir: &'a Path) -> Self {
        Self {
            archive_path,
            output_dir,
        }
    }

    /// Extracts the contents of the zip archive to the target directory.
    ///
    /// # Examples
    /// ```no_run
    /// use std::path::Path;
    /// use nahpu_archive::archive::ZipExtractor;
    ///
    /// let extractor = ZipExtractor::new(Path::new("archive.zip"), Path::new("/extracted"));
    /// extractor.extract().unwrap();
    /// ```
    pub fn extract(&self) -> Result<(), std::io::Error> {
        let file = std::fs::File::open(self.archive_path)?;
        let mut zip = zip::ZipArchive::new(file)?;

        zip.extract(self.output_dir)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_file_path_with_parent_dir() {
        let parent_dir = Path::new("/base/dir");
        let output_path = Path::new("/out/archive.zip");
        let files = vec![];
        let zip_archive = ZipArchive::new(parent_dir, None, output_path, &files);

        let file = Path::new("/base/dir/sub/file.txt");
        let parsed = zip_archive.parse_file_path(file);
        
        // Strip prefix on windows could result in backslashes, but `to_str()` just uses what's there. 
        // For Zip paths, they should be consistent, but since `parse_file_path` just uses `to_str`, 
        // it might return OS specific paths. Let's test standard stripping.
        assert_eq!(parsed, Path::new("sub/file.txt").to_str().unwrap());
    }

    #[test]
    fn test_parse_file_path_with_alt_parent_dir() {
        let parent_dir = Path::new("/base/dir");
        let alt_parent_dir = Path::new("/alt/dir");
        let output_path = Path::new("/out/archive.zip");
        let files = vec![];
        let zip_archive = ZipArchive::new(parent_dir, Some(alt_parent_dir), output_path, &files);

        let file = Path::new("/alt/dir/sub/file.txt");
        let parsed = zip_archive.parse_file_path(file);
        
        assert_eq!(parsed, Path::new("sub/file.txt").to_str().unwrap());
    }

    #[test]
    fn test_parse_file_path_fallback_to_filename() {
        let parent_dir = Path::new("/base/dir");
        let output_path = Path::new("/out/archive.zip");
        let files = vec![];
        let zip_archive = ZipArchive::new(parent_dir, None, output_path, &files);

        let file = Path::new("/completely/different/sub/file.txt");
        let parsed = zip_archive.parse_file_path(file);
        
        assert_eq!(parsed, "file.txt");
    }
}
