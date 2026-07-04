# nahpu_configs

A utility crate for managing user configuration and document presets for NAHPU (Natural History Project Utility).

It provides data models, a `redb` storage layer, and utility functions for exporting and importing preferences in JSON and JSON Lines formats.

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
