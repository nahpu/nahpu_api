# nahpu_archive

A utility crate for archiving and extracting Nahpu project data using the `zip` format.

## Example Usage

### Creating an Archive

```rust
use nahpu_archive::archive::ZipArchive;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut archive = ZipArchive::new("data_export.zip")?;
    
    // Add files to the archive
    archive.write("metadata.json", "path/to/metadata.json")?;
    archive.write("records.csv", "path/to/records.csv")?;
    
    Ok(())
}
```

### Extracting an Archive

```rust
use nahpu_archive::archive::ZipExtractor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let extractor = ZipExtractor::new("data_export.zip")?;
    
    // Extract to a destination directory
    extractor.extract("extracted_data")?;
    
    Ok(())
}
```
