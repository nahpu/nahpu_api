//! Configuration data models.
//!
//! This module defines the data models used for storing user preferences
//! and document export configurations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Independently transferable groups of NAHPU user configuration data.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum UserConfigSection {
    UserConfigs,
    RecordExportPresets,
    TemplatePresets,
    DocumentLayouts,
    TemplateTablePreview,
    CustomFields,
}

impl UserConfigSection {
    pub const ALL: [Self; 6] = [
        Self::UserConfigs,
        Self::RecordExportPresets,
        Self::TemplatePresets,
        Self::DocumentLayouts,
        Self::TemplateTablePreview,
        Self::CustomFields,
    ];
}

/// Reusable custom-field configuration. Values and destination scope are
/// deliberately excluded because SQLite remains their canonical store.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CustomFieldTemplate {
    pub template_uuid: String,
    pub label: String,
    pub field_type: String,
    pub placement: String,
    #[serde(default)]
    pub catalog_format: Option<String>,
    #[serde(default)]
    pub options_json: Option<String>,
    #[serde(default)]
    pub dwc_target: Option<String>,
    #[serde(default)]
    pub dwc_field: Option<String>,
    #[serde(default)]
    pub dwc_mode: Option<String>,
    #[serde(default)]
    pub allow_dwc_conflict: bool,
}

fn default_user_config_sections() -> Vec<UserConfigSection> {
    vec![
        UserConfigSection::UserConfigs,
        UserConfigSection::RecordExportPresets,
        UserConfigSection::TemplatePresets,
        UserConfigSection::DocumentLayouts,
    ]
}

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

/// Represents a single template preset entry stored under a specific name.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TemplatePresetEntry {
    /// Name of the template preset.
    pub name: String,
    /// Record type of the template preset.
    #[serde(rename = "recordType", default = "default_record_type")]
    pub record_type: String,
    /// Description of the template preset.
    #[serde(default)]
    pub description: String,
    /// Template configuration JSON blob.
    pub value: serde_json::Value,
}

/// Identifies layout blocks that use a template preset.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TemplatePresetUsage {
    /// Name of the print layout that contains the references.
    pub layout_name: String,
    /// Zero-based indexes of the blocks that reference the template.
    pub block_indices: Vec<i32>,
}

/// Summarizes an atomic template replacement and deletion.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TemplatePresetDeletionResult {
    /// Number of layouts whose blocks were updated.
    pub updated_layout_count: i32,
    /// Number of template block references that were updated.
    pub updated_block_count: i32,
}

fn default_record_type() -> String {
    "specimen".to_string()
}

/// Direction used to order records within a document layout block.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DocumentSortDirection {
    #[default]
    Ascending,
    Descending,
}

/// Represents a layout block within a document.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DocumentLayoutBlock {
    #[serde(alias = "templateName")]
    pub template_name: String,
    #[serde(alias = "templateCount")]
    pub template_count: i32,
    pub rows: i32,
    pub cols: i32,
    #[serde(alias = "templatePadTopMm")]
    pub template_pad_top_mm: f64,
    #[serde(alias = "templatePadLeftMm")]
    pub template_pad_left_mm: f64,
    #[serde(alias = "templatePadRightMm")]
    pub template_pad_right_mm: f64,
    #[serde(alias = "templatePadBottomMm")]
    pub template_pad_bottom_mm: f64,
    #[serde(alias = "pageBreakAfter")]
    pub page_break_after: bool,
    #[serde(alias = "sortField", default)]
    pub sort_field: Option<String>,
    #[serde(alias = "sortDirection", default)]
    pub sort_direction: DocumentSortDirection,
}

fn default_multi_block_mode() -> String {
    "Continuous".to_string()
}

/// Represents the overall configuration for document layouts.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DocumentLayoutPreset {
    pub name: String,
    #[serde(alias = "layoutType")]
    pub layout_type: String, // "WholePage" or "Continuous"
    #[serde(alias = "pageSizeKey")]
    pub page_size_key: String,
    #[serde(alias = "pageOrientation")]
    pub page_orientation: String,
    #[serde(alias = "customPageWidthMm")]
    pub custom_page_width_mm: Option<f64>,
    #[serde(alias = "customPageHeightMm")]
    pub custom_page_height_mm: Option<f64>,
    #[serde(alias = "pagePadTopMm")]
    pub page_pad_top_mm: f64,
    #[serde(alias = "pagePadLeftMm")]
    pub page_pad_left_mm: f64,
    #[serde(alias = "pagePadRightMm")]
    pub page_pad_right_mm: f64,
    #[serde(alias = "pagePadBottomMm")]
    pub page_pad_bottom_mm: f64,
    pub blocks: Vec<DocumentLayoutBlock>,
    #[serde(alias = "fillPage", default)]
    pub fill_page: bool,
    #[serde(alias = "multiBlockMode", default = "default_multi_block_mode")]
    pub multi_block_mode: String,
}

/// Represents whether a stored document layout can be read by the current schema.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DocumentLayoutStatus {
    /// Name of the stored document layout.
    pub name: String,
    /// Whether the stored layout can be deserialized by the current schema.
    pub is_compatible: bool,
    /// Deserialization error for incompatible layouts.
    pub error: Option<String>,
}

/// Represents a complete bundle of user configurations and presets for export.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserConfigsExport {
    /// Version of the serialized user configuration contract.
    #[serde(default)]
    pub schema_version: u32,
    /// Sections intentionally included in this transfer.
    ///
    /// Older JSON exports omit this field and are interpreted as complete
    /// backups containing every section.
    #[serde(default = "default_user_config_sections")]
    pub included_sections: Vec<UserConfigSection>,
    /// Map of configuration keys to their values.
    pub configs: HashMap<String, serde_json::Value>,
    /// List of record export presets.
    pub record_export_presets: Vec<ConfigPresetEntry>,
    /// List of template presets.
    pub template_presets: Vec<TemplatePresetEntry>,
    /// List of document layouts.
    #[serde(default)]
    pub document_layouts: Vec<DocumentLayoutPreset>,
    /// Ordered columns shown in specimen template-table previews.
    #[serde(default)]
    pub template_table_preview_columns: Vec<String>,
    /// Custom-field templates serialized in the transfer envelope only.
    #[serde(default)]
    pub custom_field_templates: Vec<CustomFieldTemplate>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn legacy_exports_default_to_sections_available_in_that_schema() {
        let export: UserConfigsExport = serde_json::from_value(json!({
            "schema_version": 1,
            "configs": {},
            "record_export_presets": [],
            "template_presets": [],
            "document_layouts": [],
        }))
        .unwrap();

        assert_eq!(
            export.included_sections,
            vec![
                UserConfigSection::UserConfigs,
                UserConfigSection::RecordExportPresets,
                UserConfigSection::TemplatePresets,
                UserConfigSection::DocumentLayouts,
            ]
        );
        assert!(export.template_table_preview_columns.is_empty());
    }

    #[test]
    fn legacy_layout_blocks_default_to_original_ascending_order() {
        let block: DocumentLayoutBlock = serde_json::from_value(json!({
            "templateName": "Specimen",
            "templateCount": 1,
            "rows": 1,
            "cols": 1,
            "templatePadTopMm": 0.0,
            "templatePadLeftMm": 0.0,
            "templatePadRightMm": 0.0,
            "templatePadBottomMm": 0.0,
            "pageBreakAfter": false
        }))
        .unwrap();

        assert_eq!(block.sort_field, None);
        assert_eq!(block.sort_direction, DocumentSortDirection::Ascending);
    }
}
