//! Configuration data models.
//!
//! This module defines the data models used for storing user preferences
//! and document export configurations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a combined export field configuration.
///
/// It holds a single ID representing a group of fields that are combined
/// into a single column/field during export operations.
///
/// # Examples
///
/// ```
/// use nahpu_configs::models::ConfigCombinedField;
///
/// let field = ConfigCombinedField {
///     field_id: "name_and_id".to_string(),
///     fields: vec!["first_name".to_string(), "id".to_string()],
/// };
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConfigCombinedField {
    /// Unique identifier for the combined field.
    pub field_id: String,
    /// List of field names that are combined.
    pub fields: Vec<String>,
}

/// Represents an export preset containing field maps and combined fields.
///
/// A preset specifies how fields are mapped and which fields are grouped together
/// when exporting research data.
///
/// # Examples
///
/// ```
/// use nahpu_configs::models::ConfigExportPreset;
/// use std::collections::HashMap;
///
/// let mut fields = HashMap::new();
/// fields.insert("id".to_string(), "Identifier".to_string());
///
/// let preset = ConfigExportPreset {
///     fields,
///     combined_fields: Vec::new(),
/// };
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConfigExportPreset {
    /// Map of standard field keys to their export names.
    pub fields: HashMap<String, String>,
    /// List of fields that are combined during export.
    pub combined_fields: Vec<ConfigCombinedField>,
}

/// Represents a single preset entry stored under a specific name.
///
/// Bundles a preset configuration with its user-provided name.
///
/// # Examples
///
/// ```
/// use nahpu_configs::models::{ConfigExportPreset, ConfigPresetEntry};
/// use std::collections::HashMap;
///
/// let preset = ConfigExportPreset {
///     fields: HashMap::new(),
///     combined_fields: Vec::new(),
/// };
///
/// let entry = ConfigPresetEntry {
///     name: "Default Preset".to_string(),
///     preset,
/// };
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConfigPresetEntry {
    /// Name of the preset.
    pub name: String,
    /// Preset details.
    pub preset: ConfigExportPreset,
}
