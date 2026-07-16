# nahpu_archive

A utility crate for ZIP archives, tar.gz archives, and single-file gzip
compression used by NAHPU exports.

## Example Usage

### Creating an Archive

```rust
use nahpu_archive::zip::ZipArchive;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let files = vec!["path/to/metadata.json".into(), "path/to/records.csv".into()];
    ZipArchive::new(Path::new("path/to"), None, Path::new("data_export.zip"), &files)
        .write()?;
    Ok(())
}
```

### Extracting an Archive

```rust
use nahpu_archive::zip::ZipExtractor;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    ZipExtractor::new(Path::new("data_export.zip"), Path::new("extracted_data"))
        .extract()?;
    Ok(())
}
```

### Creating a TAR.GZ Package

```rust
use nahpu_archive::tar_gzip::TarGzipArchive;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let files = vec![
        "package/datapackage.json".into(),
        "package/records.csv".into(),
    ];
    TarGzipArchive::new(
        Path::new("package"),
        Path::new("data-package.tar.gz"),
        &files,
    )
    .write()?;
    Ok(())
}
```

`gzip` is intentionally limited to one input stream. Use `tar_gzip` when a
package contains multiple files.
