//! Redb config database manager.
//!
//! Provides the storage layer and CRUD operations for managing user settings,
//! project configurations, and document presets.

use crate::models::{
    ConfigExportPreset, ConfigPresetEntry, DocumentLayoutPreset, DocumentLayoutStatus,
    TemplatePresetDeletionResult, TemplatePresetEntry, TemplatePresetUsage,
};
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use std::sync::OnceLock;

const USER_CONFIGS: TableDefinition<&str, &[u8]> = TableDefinition::new("user_configs");
const RECORD_EXPORT_PRESETS: TableDefinition<&str, &[u8]> =
    TableDefinition::new("record_export_presets");
const TEMPLATE_PRESETS: TableDefinition<&str, &[u8]> = TableDefinition::new("template_presets");
const DOCUMENT_LAYOUTS: TableDefinition<&str, &[u8]> = TableDefinition::new("document_layouts");

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
                .open_table(RECORD_EXPORT_PRESETS)
                .map_err(|e| e.to_string())?;
            let _table3 = write_txn
                .open_table(TEMPLATE_PRESETS)
                .map_err(|e| e.to_string())?;
            let _table4 = write_txn
                .open_table(DOCUMENT_LAYOUTS)
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

    /// Saves a record export preset configuration.
    pub fn set_record_export_preset(
        &self,
        name: &str,
        preset: &ConfigExportPreset,
    ) -> Result<(), String> {
        let bytes = serde_json::to_vec(preset).map_err(|e| e.to_string())?;
        let write_txn = self.database.begin_write().map_err(|e| e.to_string())?;
        {
            let mut table = write_txn
                .open_table(RECORD_EXPORT_PRESETS)
                .map_err(|e| e.to_string())?;
            table
                .insert(name, bytes.as_slice())
                .map_err(|e| e.to_string())?;
        }
        write_txn.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Retrieves a saved record export preset by name.
    pub fn get_record_export_preset(
        &self,
        name: &str,
    ) -> Result<Option<ConfigExportPreset>, String> {
        let read_txn = self.database.begin_read().map_err(|e| e.to_string())?;
        let table = read_txn
            .open_table(RECORD_EXPORT_PRESETS)
            .map_err(|e| e.to_string())?;
        match table.get(name).map_err(|e| e.to_string())? {
            Some(guard) => {
                let preset = serde_json::from_slice(guard.value()).map_err(|e| e.to_string())?;
                Ok(Some(preset))
            }
            None => Ok(None),
        }
    }

    /// Deletes a record export preset.
    pub fn delete_record_export_preset(&self, name: &str) -> Result<(), String> {
        let write_txn = self.database.begin_write().map_err(|e| e.to_string())?;
        {
            let mut table = write_txn
                .open_table(RECORD_EXPORT_PRESETS)
                .map_err(|e| e.to_string())?;
            table.remove(name).map_err(|e| e.to_string())?;
        }
        write_txn.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Retrieves all record export presets from the database.
    pub fn get_all_record_export_presets(&self) -> Result<Vec<ConfigPresetEntry>, String> {
        let read_txn = self.database.begin_read().map_err(|e| e.to_string())?;
        let table = read_txn
            .open_table(RECORD_EXPORT_PRESETS)
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

    /// Saves a template preset.
    pub fn set_template_preset(
        &self,
        name: &str,
        preset: &serde_json::Value,
    ) -> Result<(), String> {
        let bytes = serde_json::to_vec(preset).map_err(|e| e.to_string())?;
        let write_txn = self.database.begin_write().map_err(|e| e.to_string())?;
        {
            let mut table = write_txn
                .open_table(TEMPLATE_PRESETS)
                .map_err(|e| e.to_string())?;
            table
                .insert(name, bytes.as_slice())
                .map_err(|e| e.to_string())?;
        }
        write_txn.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Retrieves a saved template preset by name.
    pub fn get_template_preset(&self, name: &str) -> Result<Option<serde_json::Value>, String> {
        let read_txn = self.database.begin_read().map_err(|e| e.to_string())?;
        let table = read_txn
            .open_table(TEMPLATE_PRESETS)
            .map_err(|e| e.to_string())?;
        match table.get(name).map_err(|e| e.to_string())? {
            Some(guard) => {
                let preset = serde_json::from_slice(guard.value()).map_err(|e| e.to_string())?;
                Ok(Some(preset))
            }
            None => Ok(None),
        }
    }

    /// Deletes a template preset.
    pub fn delete_template_preset(&self, name: &str) -> Result<(), String> {
        let write_txn = self.database.begin_write().map_err(|e| e.to_string())?;
        {
            let mut table = write_txn
                .open_table(TEMPLATE_PRESETS)
                .map_err(|e| e.to_string())?;
            table.remove(name).map_err(|e| e.to_string())?;
        }
        write_txn.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Lists each print layout and block that references a template preset.
    ///
    /// Layouts that cannot be read by the current schema cause this method to
    /// fail rather than risk reporting an incomplete dependency list.
    pub fn get_template_preset_usages(
        &self,
        name: &str,
    ) -> Result<Vec<TemplatePresetUsage>, String> {
        let read_txn = self.database.begin_read().map_err(|e| e.to_string())?;
        let table = read_txn
            .open_table(DOCUMENT_LAYOUTS)
            .map_err(|e| e.to_string())?;
        let mut usages = Vec::new();

        for entry in table.iter().map_err(|e| e.to_string())? {
            let (key, value) = entry.map_err(|e| e.to_string())?;
            let layout: DocumentLayoutPreset =
                serde_json::from_slice(value.value()).map_err(|e| e.to_string())?;
            let block_indices = layout
                .blocks
                .iter()
                .enumerate()
                .filter_map(|(index, block)| (block.template_name == name).then_some(index as i32))
                .collect::<Vec<_>>();

            if !block_indices.is_empty() {
                usages.push(TemplatePresetUsage {
                    layout_name: key.value().to_string(),
                    block_indices,
                });
            }
        }

        Ok(usages)
    }

    /// Replaces every reference to a template preset and deletes it atomically.
    ///
    /// A referenced template requires a replacement with the same record type.
    /// The operation is performed in one redb write transaction, so a failure
    /// leaves both layouts and templates unchanged.
    pub fn delete_template_preset_with_replacement(
        &self,
        name: &str,
        replacement_name: Option<&str>,
    ) -> Result<TemplatePresetDeletionResult, String> {
        if replacement_name == Some(name) {
            return Err("A template cannot replace itself".to_string());
        }

        let write_txn = self.database.begin_write().map_err(|e| e.to_string())?;
        let (target_record_type, replacement_record_type) = {
            let table = write_txn
                .open_table(TEMPLATE_PRESETS)
                .map_err(|e| e.to_string())?;
            let target = table
                .get(name)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Template preset '{name}' not found"))?;
            let target_value: serde_json::Value =
                serde_json::from_slice(target.value()).map_err(|e| e.to_string())?;
            let target_record_type = template_record_type(&target_value);

            let replacement_record_type = if let Some(replacement_name) = replacement_name {
                let replacement = table
                    .get(replacement_name)
                    .map_err(|e| e.to_string())?
                    .ok_or_else(|| {
                        format!("Replacement template preset '{replacement_name}' not found")
                    })?;
                let replacement_value: serde_json::Value =
                    serde_json::from_slice(replacement.value()).map_err(|e| e.to_string())?;
                Some(template_record_type(&replacement_value))
            } else {
                None
            };

            (target_record_type, replacement_record_type)
        };

        if replacement_record_type
            .as_ref()
            .is_some_and(|record_type| record_type != &target_record_type)
        {
            return Err("The replacement template must use the same record type".to_string());
        }

        let mut updated_layout_count = 0;
        let mut updated_block_count = 0;
        {
            let mut table = write_txn
                .open_table(DOCUMENT_LAYOUTS)
                .map_err(|e| e.to_string())?;
            let layouts = table
                .iter()
                .map_err(|e| e.to_string())?
                .map(|entry| {
                    let (key, value) = entry.map_err(|e| e.to_string())?;
                    let layout: DocumentLayoutPreset =
                        serde_json::from_slice(value.value()).map_err(|e| e.to_string())?;
                    Ok::<_, String>((key.value().to_string(), layout))
                })
                .collect::<Result<Vec<_>, _>>()?;

            for (layout_name, mut layout) in layouts {
                let mut replacements = 0;
                for block in &mut layout.blocks {
                    if block.template_name == name {
                        let replacement_name = replacement_name.ok_or_else(|| {
                            format!(
                                "Template preset '{name}' is used by print layout '{}'",
                                layout_name
                            )
                        })?;
                        block.template_name = replacement_name.to_string();
                        replacements += 1;
                    }
                }

                if replacements > 0 {
                    let bytes = serde_json::to_vec(&layout).map_err(|e| e.to_string())?;
                    table
                        .insert(layout_name.as_str(), bytes.as_slice())
                        .map_err(|e| e.to_string())?;
                    updated_layout_count += 1;
                    updated_block_count += replacements;
                }
            }
        }

        {
            let mut table = write_txn
                .open_table(TEMPLATE_PRESETS)
                .map_err(|e| e.to_string())?;
            table.remove(name).map_err(|e| e.to_string())?;
        }
        write_txn.commit().map_err(|e| e.to_string())?;

        Ok(TemplatePresetDeletionResult {
            updated_layout_count,
            updated_block_count,
        })
    }

    /// Retrieves all template presets from the database.
    pub fn get_all_template_presets(&self) -> Result<Vec<TemplatePresetEntry>, String> {
        let read_txn = self.database.begin_read().map_err(|e| e.to_string())?;
        let table = read_txn
            .open_table(TEMPLATE_PRESETS)
            .map_err(|e| e.to_string())?;
        let mut presets = Vec::new();
        for entry in table.iter().map_err(|e| e.to_string())? {
            let (key, value) = entry.map_err(|e| e.to_string())?;
            let preset: serde_json::Value =
                serde_json::from_slice(value.value()).map_err(|e| e.to_string())?;

            let record_type = preset
                .get("recordType")
                .and_then(|v| v.as_str())
                .unwrap_or("specimen")
                .to_string();

            let description = preset
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            presets.push(TemplatePresetEntry {
                name: key.value().to_string(),
                record_type,
                description,
                value: preset,
            });
        }
        Ok(presets)
    }

    /// Saves a document layout preset.
    pub fn set_document_layout(
        &self,
        name: &str,
        layout: &DocumentLayoutPreset,
    ) -> Result<(), String> {
        let bytes = serde_json::to_vec(layout).map_err(|e| e.to_string())?;
        let write_txn = self.database.begin_write().map_err(|e| e.to_string())?;
        {
            let mut table = write_txn
                .open_table(DOCUMENT_LAYOUTS)
                .map_err(|e| e.to_string())?;
            table
                .insert(name, bytes.as_slice())
                .map_err(|e| e.to_string())?;
        }
        write_txn.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Retrieves a saved document layout preset by name.
    pub fn get_document_layout(&self, name: &str) -> Result<Option<DocumentLayoutPreset>, String> {
        let read_txn = self.database.begin_read().map_err(|e| e.to_string())?;
        let table = read_txn
            .open_table(DOCUMENT_LAYOUTS)
            .map_err(|e| e.to_string())?;
        match table.get(name).map_err(|e| e.to_string())? {
            Some(guard) => {
                let layout = serde_json::from_slice(guard.value()).map_err(|e| e.to_string())?;
                Ok(Some(layout))
            }
            None => Ok(None),
        }
    }

    /// Deletes a document layout preset.
    pub fn delete_document_layout(&self, name: &str) -> Result<(), String> {
        let write_txn = self.database.begin_write().map_err(|e| e.to_string())?;
        {
            let mut table = write_txn
                .open_table(DOCUMENT_LAYOUTS)
                .map_err(|e| e.to_string())?;
            table.remove(name).map_err(|e| e.to_string())?;
        }
        write_txn.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Retrieves all document layout presets from the database.
    pub fn get_all_document_layouts(&self) -> Result<Vec<DocumentLayoutPreset>, String> {
        let read_txn = self.database.begin_read().map_err(|e| e.to_string())?;
        let table = read_txn
            .open_table(DOCUMENT_LAYOUTS)
            .map_err(|e| e.to_string())?;
        let mut layouts = Vec::new();
        for entry in table.iter().map_err(|e| e.to_string())? {
            let (_key, value) = entry.map_err(|e| e.to_string())?;
            let layout = serde_json::from_slice(value.value()).map_err(|e| e.to_string())?;
            layouts.push(layout);
        }
        Ok(layouts)
    }

    /// Retrieves compatibility status for every stored document layout preset.
    pub fn get_document_layout_statuses(&self) -> Result<Vec<DocumentLayoutStatus>, String> {
        let read_txn = self.database.begin_read().map_err(|e| e.to_string())?;
        let table = read_txn
            .open_table(DOCUMENT_LAYOUTS)
            .map_err(|e| e.to_string())?;
        let mut statuses = Vec::new();
        for entry in table.iter().map_err(|e| e.to_string())? {
            let (key, value) = entry.map_err(|e| e.to_string())?;
            let name = key.value().to_string();
            let result = serde_json::from_slice::<DocumentLayoutPreset>(value.value());
            statuses.push(DocumentLayoutStatus {
                name,
                is_compatible: result.is_ok(),
                error: result.err().map(|e| e.to_string()),
            });
        }
        Ok(statuses)
    }

    /// Exports all user configs, record export presets, and template presets from the database.
    pub fn export_configs(&self) -> Result<crate::models::UserConfigsExport, String> {
        let configs = self.get_all_user_configs()?;
        let record_export_presets = self.get_all_record_export_presets()?;
        let template_presets = self.get_all_template_presets()?;
        let document_layouts = self.get_all_document_layouts()?;
        Ok(crate::models::UserConfigsExport {
            schema_version: crate::USER_CONFIG_SCHEMA_VERSION,
            configs,
            record_export_presets,
            template_presets,
            document_layouts,
        })
    }

    /// Imports and replaces all user configs, record export presets, and template presets.
    pub fn import_configs(&self, export: crate::models::UserConfigsExport) -> Result<(), String> {
        // Clear existing record export presets first
        let write_txn = self.database.begin_write().map_err(|e| e.to_string())?;
        {
            let mut table = write_txn
                .open_table(RECORD_EXPORT_PRESETS)
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

        // Clear existing template presets
        let write_txn = self.database.begin_write().map_err(|e| e.to_string())?;
        {
            let mut table = write_txn
                .open_table(TEMPLATE_PRESETS)
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

        // Clear existing document layouts
        let write_txn = self.database.begin_write().map_err(|e| e.to_string())?;
        {
            let mut table = write_txn
                .open_table(DOCUMENT_LAYOUTS)
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

        // Import new record export presets
        for entry in export.record_export_presets {
            self.set_record_export_preset(&entry.name, &entry.preset)?;
        }

        // Import new template presets
        for entry in export.template_presets {
            self.set_template_preset(&entry.name, &entry.value)?;
        }

        // Import new document layouts
        for entry in export.document_layouts {
            self.set_document_layout(&entry.name, &entry)?;
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

fn template_record_type(template: &serde_json::Value) -> String {
    template
        .get("recordType")
        .and_then(|value| value.as_str())
        .unwrap_or("specimen")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{DocumentLayoutBlock, DocumentLayoutPreset};
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_document_layout_crud() {
        let _guard = TEST_LOCK.lock().unwrap();
        let mut db_path = std::env::temp_dir();
        db_path.push(format!(
            "test_config_{}.redb",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let db_path_str = db_path.to_str().unwrap();

        ConfigDb::init(db_path_str).unwrap();
        let db = ConfigDb::get_instance().unwrap();

        let block = DocumentLayoutBlock {
            template_name: "Standard".to_string(),
            template_count: 1,
            rows: 8,
            cols: 4,
            template_pad_top_mm: 1.0,
            template_pad_left_mm: 1.0,
            template_pad_right_mm: 1.0,
            template_pad_bottom_mm: 1.0,
            page_break_after: false,
        };

        let layout = DocumentLayoutPreset {
            name: "Test Layout".to_string(),
            layout_type: "WholePage".to_string(),
            page_size_key: "Letter".to_string(),
            page_orientation: "portrait".to_string(),
            custom_page_width_mm: None,
            custom_page_height_mm: None,
            page_pad_top_mm: 8.0,
            page_pad_left_mm: 8.0,
            page_pad_right_mm: 8.0,
            page_pad_bottom_mm: 8.0,
            blocks: vec![block],
            fill_page: false,
            multi_block_mode: "Continuous".to_string(),
        };

        // Insert
        db.set_document_layout("Test Layout", &layout).unwrap();

        // Retrieve
        let retrieved = db.get_document_layout("Test Layout").unwrap().unwrap();
        assert_eq!(retrieved.name, "Test Layout");
        assert_eq!(retrieved.layout_type, "WholePage");
        assert_eq!(retrieved.blocks.len(), 1);
        assert_eq!(retrieved.blocks[0].template_name, "Standard");

        // Retrieve all
        let all = db.get_all_document_layouts().unwrap();
        assert!(all.iter().any(|l| l.name == "Test Layout"));

        // Delete
        db.delete_document_layout("Test Layout").unwrap();
        assert!(db.get_document_layout("Test Layout").unwrap().is_none());

        // Clean up
        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn test_template_deletion_replaces_all_layout_references() {
        let _guard = TEST_LOCK.lock().unwrap();
        if ConfigDb::get_instance().is_err() {
            let mut db_path = std::env::temp_dir();
            db_path.push(format!(
                "test_template_delete_{}.redb",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            ConfigDb::init(db_path.to_str().unwrap()).unwrap();
        }
        let db = ConfigDb::get_instance().unwrap();
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let target_name = format!("target_template_{suffix}");
        let replacement_name = format!("replacement_template_{suffix}");
        let layout_name = format!("template_usage_layout_{suffix}");
        let target = serde_json::json!({ "recordType": "specimen" });
        let replacement = serde_json::json!({ "recordType": "specimen" });

        db.set_template_preset(&target_name, &target).unwrap();
        db.set_template_preset(&replacement_name, &replacement)
            .unwrap();
        let layout = DocumentLayoutPreset {
            name: layout_name.clone(),
            layout_type: "WholePage".to_string(),
            page_size_key: "Letter".to_string(),
            page_orientation: "portrait".to_string(),
            custom_page_width_mm: None,
            custom_page_height_mm: None,
            page_pad_top_mm: 8.0,
            page_pad_left_mm: 8.0,
            page_pad_right_mm: 8.0,
            page_pad_bottom_mm: 8.0,
            blocks: vec![
                DocumentLayoutBlock {
                    template_name: target_name.clone(),
                    template_count: 1,
                    rows: 1,
                    cols: 1,
                    template_pad_top_mm: 0.0,
                    template_pad_left_mm: 0.0,
                    template_pad_right_mm: 0.0,
                    template_pad_bottom_mm: 0.0,
                    page_break_after: false,
                },
                DocumentLayoutBlock {
                    template_name: target_name.clone(),
                    template_count: 1,
                    rows: 1,
                    cols: 1,
                    template_pad_top_mm: 0.0,
                    template_pad_left_mm: 0.0,
                    template_pad_right_mm: 0.0,
                    template_pad_bottom_mm: 0.0,
                    page_break_after: false,
                },
            ],
            fill_page: false,
            multi_block_mode: "Continuous".to_string(),
        };
        db.set_document_layout(&layout_name, &layout).unwrap();

        let usages = db.get_template_preset_usages(&target_name).unwrap();
        assert_eq!(usages.len(), 1);
        assert_eq!(usages[0].block_indices, vec![0, 1]);

        let result = db
            .delete_template_preset_with_replacement(&target_name, Some(&replacement_name))
            .unwrap();
        assert_eq!(result.updated_layout_count, 1);
        assert_eq!(result.updated_block_count, 2);
        assert!(db.get_template_preset(&target_name).unwrap().is_none());

        let saved_layout = db.get_document_layout(&layout_name).unwrap().unwrap();
        assert!(
            saved_layout
                .blocks
                .iter()
                .all(|block| block.template_name == replacement_name)
        );

        db.delete_document_layout(&layout_name).unwrap();
        db.delete_template_preset(&replacement_name).unwrap();
    }

    #[test]
    fn test_document_layout_statuses_include_invalid_layouts() {
        let _guard = TEST_LOCK.lock().unwrap();
        let mut db_path = std::env::temp_dir();
        db_path.push(format!(
            "test_config_status_{}.redb",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let db_path_str = db_path.to_str().unwrap();

        ConfigDb::init(db_path_str).unwrap();
        let db = ConfigDb::get_instance().unwrap();

        let write_txn = db.database.begin_write().unwrap();
        {
            let mut table = write_txn.open_table(DOCUMENT_LAYOUTS).unwrap();
            table
                .insert(
                    "Old Layout",
                    br#"{"name":"Old Layout","blocks":[{"label_count":1}]}"#.as_slice(),
                )
                .unwrap();
        }
        write_txn.commit().unwrap();

        let statuses = db.get_document_layout_statuses().unwrap();
        let status = statuses.iter().find(|s| s.name == "Old Layout").unwrap();
        assert!(!status.is_compatible);
        assert!(status.error.is_some());

        db.delete_document_layout("Old Layout").unwrap();
        assert!(db.get_document_layout_statuses().unwrap().is_empty());

        let _ = std::fs::remove_file(db_path);
    }
}
