//! Redb config database manager.
//!
//! Provides the storage layer and CRUD operations for managing user settings,
//! project configurations, and document presets.

use crate::models::{ConfigExportPreset, ConfigPresetEntry};
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use std::sync::OnceLock;

const USER_CONFIGS: TableDefinition<&str, &[u8]> = TableDefinition::new("user_configs");
const DOCUMENT_PRESETS: TableDefinition<&str, &[u8]> = TableDefinition::new("document_presets");

static INSTANCE: OnceLock<ConfigDb> = OnceLock::new();

/// Database manager for NAHPU configurations and presets.
///
/// Wraps a `redb` database instance and handles read/write transactions
/// to store serialized preferences and presets.
pub struct ConfigDb {
    database: Database,
}

impl ConfigDb {
    /// Initializes the database instance at the given file path.
    ///
    /// Creates tables if they do not exist and sets the global singleton instance.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nahpu_configs::ConfigDb;
    /// ConfigDb::init("path/to/db.redb").unwrap();
    /// ```
    pub fn init(path: &str) -> Result<(), String> {
        if INSTANCE.get().is_some() {
            return Ok(());
        }

        let database = Database::create(path).map_err(|e| e.to_string())?;
        let write_txn = database.begin_write().map_err(|e| e.to_string())?;
        {
            let _table1 = write_txn
                .open_table(USER_CONFIGS)
                .map_err(|e| e.to_string())?;
            let _table2 = write_txn
                .open_table(DOCUMENT_PRESETS)
                .map_err(|e| e.to_string())?;
        }
        write_txn.commit().map_err(|e| e.to_string())?;

        let manager = ConfigDb { database };
        if INSTANCE.set(manager).is_err() {
            // Already set by another thread concurrently.
            println!("ConfigDb already initialized");
        }

        Ok(())
    }

    /// Retrieves the static instance of `ConfigDb`.
    pub fn get_instance() -> Result<&'static ConfigDb, String> {
        INSTANCE
            .get()
            .ok_or_else(|| "ConfigDb not initialized".to_string())
    }

    /// Sets a list configuration value under the given key.
    pub fn set_user_config_list(&self, key: &str, value: &[String]) -> Result<(), String> {
        let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
        self.set_user_config_bytes(key, &bytes)
    }

    /// Retrieves a list configuration value by key.
    pub fn get_user_config_list(&self, key: &str) -> Result<Option<Vec<String>>, String> {
        match self.get_user_config_bytes(key)? {
            Some(bytes) => {
                let list = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                Ok(Some(list))
            }
            None => Ok(None),
        }
    }

    /// Sets a string configuration value under the given key.
    pub fn set_user_config_string(&self, key: &str, value: &str) -> Result<(), String> {
        let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
        self.set_user_config_bytes(key, &bytes)
    }

    /// Retrieves a string configuration value by key.
    pub fn get_user_config_string(&self, key: &str) -> Result<Option<String>, String> {
        match self.get_user_config_bytes(key)? {
            Some(bytes) => {
                let s = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                Ok(Some(s))
            }
            None => Ok(None),
        }
    }

    /// Deletes a configuration entry.
    pub fn delete_user_config(&self, key: &str) -> Result<(), String> {
        let write_txn = self.database.begin_write().map_err(|e| e.to_string())?;
        {
            let mut table = write_txn
                .open_table(USER_CONFIGS)
                .map_err(|e| e.to_string())?;
            table.remove(key).map_err(|e| e.to_string())?;
        }
        write_txn.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Saves a document export preset configuration.
    pub fn set_document_preset(
        &self,
        name: &str,
        preset: &ConfigExportPreset,
    ) -> Result<(), String> {
        let bytes = serde_json::to_vec(preset).map_err(|e| e.to_string())?;
        let write_txn = self.database.begin_write().map_err(|e| e.to_string())?;
        {
            let mut table = write_txn
                .open_table(DOCUMENT_PRESETS)
                .map_err(|e| e.to_string())?;
            table
                .insert(name, bytes.as_slice())
                .map_err(|e| e.to_string())?;
        }
        write_txn.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Retrieves a saved document export preset by name.
    pub fn get_document_preset(&self, name: &str) -> Result<Option<ConfigExportPreset>, String> {
        let read_txn = self.database.begin_read().map_err(|e| e.to_string())?;
        let table = read_txn
            .open_table(DOCUMENT_PRESETS)
            .map_err(|e| e.to_string())?;
        match table.get(name).map_err(|e| e.to_string())? {
            Some(guard) => {
                let preset = serde_json::from_slice(guard.value()).map_err(|e| e.to_string())?;
                Ok(Some(preset))
            }
            None => Ok(None),
        }
    }

    /// Deletes a document export preset.
    pub fn delete_document_preset(&self, name: &str) -> Result<(), String> {
        let write_txn = self.database.begin_write().map_err(|e| e.to_string())?;
        {
            let mut table = write_txn
                .open_table(DOCUMENT_PRESETS)
                .map_err(|e| e.to_string())?;
            table.remove(name).map_err(|e| e.to_string())?;
        }
        write_txn.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Retrieves all document presets from the database.
    pub fn get_all_document_presets(&self) -> Result<Vec<ConfigPresetEntry>, String> {
        let read_txn = self.database.begin_read().map_err(|e| e.to_string())?;
        let table = read_txn
            .open_table(DOCUMENT_PRESETS)
            .map_err(|e| e.to_string())?;
        let mut presets = Vec::new();
        for entry in table.iter().map_err(|e| e.to_string())? {
            let (key, value) = entry.map_err(|e| e.to_string())?;
            let preset = serde_json::from_slice(value.value()).map_err(|e| e.to_string())?;
            presets.push(ConfigPresetEntry {
                name: key.value().to_string(),
                preset,
            });
        }
        Ok(presets)
    }

    /// Exports all user configs and document presets from the database.
    pub fn export_configs(&self) -> Result<crate::models::UserConfigsExport, String> {
        let configs = self.get_all_user_configs()?;
        let document_presets = self.get_all_document_presets()?;
        Ok(crate::models::UserConfigsExport {
            configs,
            document_presets,
        })
    }

    /// Imports and replaces all user configs and document presets.
    pub fn import_configs(&self, export: crate::models::UserConfigsExport) -> Result<(), String> {
        // Clear existing presets first
        let write_txn = self.database.begin_write().map_err(|e| e.to_string())?;
        {
            let mut table = write_txn
                .open_table(DOCUMENT_PRESETS)
                .map_err(|e| e.to_string())?;
            let keys: Vec<String> = table
                .iter()
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok().map(|(k, _)| k.value().to_string()))
                .collect();
            for k in keys {
                table.remove(k.as_str()).map_err(|e| e.to_string())?;
            }
        }
        write_txn.commit().map_err(|e| e.to_string())?;

        // Import new configs
        for (key, val) in export.configs {
            let bytes = serde_json::to_vec(&val).map_err(|e| e.to_string())?;
            self.set_user_config_bytes(&key, &bytes)?;
        }

        // Import new presets
        for entry in export.document_presets {
            self.set_document_preset(&entry.name, &entry.preset)?;
        }

        Ok(())
    }

    /// Helper to get all user configs from the database.
    pub fn get_all_user_configs(
        &self,
    ) -> Result<std::collections::HashMap<String, serde_json::Value>, String> {
        let read_txn = self.database.begin_read().map_err(|e| e.to_string())?;
        let table = read_txn
            .open_table(USER_CONFIGS)
            .map_err(|e| e.to_string())?;
        let mut configs = std::collections::HashMap::new();
        for entry in table.iter().map_err(|e| e.to_string())? {
            let (key, value) = entry.map_err(|e| e.to_string())?;
            let val = serde_json::from_slice(value.value()).map_err(|e| e.to_string())?;
            configs.insert(key.value().to_string(), val);
        }
        Ok(configs)
    }

    fn set_user_config_bytes(&self, key: &str, value: &[u8]) -> Result<(), String> {
        let write_txn = self.database.begin_write().map_err(|e| e.to_string())?;
        {
            let mut table = write_txn
                .open_table(USER_CONFIGS)
                .map_err(|e| e.to_string())?;
            table.insert(key, value).map_err(|e| e.to_string())?;
        }
        write_txn.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn get_user_config_bytes(&self, key: &str) -> Result<Option<Vec<u8>>, String> {
        let read_txn = self.database.begin_read().map_err(|e| e.to_string())?;
        let table = read_txn
            .open_table(USER_CONFIGS)
            .map_err(|e| e.to_string())?;
        let value = table.get(key).map_err(|e| e.to_string())?;
        Ok(value.map(|guard| guard.value().to_vec()))
    }
}

