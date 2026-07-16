# nahpu_dp

`nahpu_dp` creates reproducible **NAHPU Data Packages**. Each package is a
Frictionless Data Package containing:

- `datapackage.json`;
- `nahpu.toml`;
- a lossless NAHPU SQLite database snapshot;
- CSV representations of every NAHPU table;
- a CSV mapping each SQLite enum index to its code and display name;
- CSV snapshots of the site, event, and specimen controlled vocabularies;
- versioned user configurations;
- available project media and user files.

Packages can be written as standard ZIP archives or tar archives compressed
with gzip.

```rust
use nahpu_dp::{ArchiveFormat, PackageRequest};

let request = PackageRequest {
    archive_format: ArchiveFormat::TarGzip,
    name: "field-project".to_string(),
    app_name: "NAHPU".to_string(),
    app_version: "1.0.0".to_string(),
    app_build: "36".to_string(),
    database_schema_version: 7,
    user_config_schema_version: 1,
    dependencies: Default::default(),
    database_path: "nahpu.sqlite3".to_string(),
    database_schema_path: None,
    user_configs: serde_json::json!({"schema_version": 1}),
    tables: Vec::new(),
    enum_mappings: Vec::new(),
    controlled_vocabularies: Vec::new(),
    files: Vec::new(),
};

let _ = request;
```
