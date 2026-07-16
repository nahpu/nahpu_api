# nahpu_configs

A utility crate for managing user configuration and document presets for NAHPU (Natural History Project Utility).

It provides data models, a `redb` storage layer, and utility functions for exporting and importing preferences in JSON and JSON Lines formats.

Configuration exports include a `schema_version`. Version `1` covers user
configuration values, record export presets, template presets, and document
layouts. Missing versions are treated as legacy input during deserialization.

## Role in NAHPU

NAHPU uses `nahpu_configs` for user configuration that affects reproducible
outputs but does not belong in the project SQLite database. The crate stores
these values in `redb` tables for user configs, record export presets, template
presets, and document layouts.

This is distinct from:

- **SQLite project data**, which remains in the Flutter app's Drift database and
  contains the canonical specimen, site, collecting event, personnel, taxonomy,
  media metadata, and narrative records.
- **SharedPreferences app settings**, which stay in Flutter for local UI or app
  state that is not required for reproducibility.

Because `nahpu_configs` stores reproducibility-related user choices, its data can
be exported/imported with project archives so another NAHPU installation can use
the same option lists, formatting choices, presets, and document layouts.

## Example Usage

### Initializing the Config Database

```rust
use nahpu_configs::ConfigDb;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    ConfigDb::init("path/to/configs.db")?;
    let db = ConfigDb::get_instance()?;
    
    // Set a user config string
    db.set_user_config_string("themeMode", "dark")?;
    
    // Retrieve the value
    if let Some(theme) = db.get_user_config_string("themeMode")? {
        println!("Theme: {}", theme);
    }
    
    Ok(())
}
```

### Exporting and Importing Configs

```rust
use nahpu_configs::ConfigDb;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    ConfigDb::init("path/to/configs.db")?;
    let db = ConfigDb::get_instance()?;
    
    // Export configs
    let export = db.export_configs()?;
    
    // Serialize to JSON
    let json_str = serde_json::to_string_pretty(&export)?;
    fs::write("configs.json", json_str)?;
    
    // Serialize to JSON Lines (json.nl)
    let json_lines_str = nahpu_configs::json_lines::export_to_json_lines(&export);
    fs::write("configs.json.nl", json_lines_str)?;
    
    Ok(())
}
```
