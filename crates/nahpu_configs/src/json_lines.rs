use crate::models::{
    ConfigPresetEntry, DocumentLayoutPreset, TemplatePresetEntry, UserConfigsExport,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum JsonLineEntry {
    Config {
        key: String,
        value: serde_json::Value,
    },
    RecordExportPreset(ConfigPresetEntry),
    TemplatePreset(TemplatePresetEntry),
    DocumentLayout(DocumentLayoutPreset),
}

/// Serializes the configs to a JSON Lines string.
pub fn export_to_json_lines(export: &UserConfigsExport) -> String {
    let mut lines = Vec::new();

    // Sort keys to have deterministic output
    let mut config_keys: Vec<&String> = export.configs.keys().collect();
    config_keys.sort();

    for key in config_keys {
        let value = export.configs[key].clone();
        let entry = JsonLineEntry::Config {
            key: key.clone(),
            value,
        };
        if let Ok(line) = serde_json::to_string(&entry) {
            lines.push(line);
        }
    }

    for preset in &export.record_export_presets {
        let entry = JsonLineEntry::RecordExportPreset(preset.clone());
        if let Ok(line) = serde_json::to_string(&entry) {
            lines.push(line);
        }
    }

    for template in &export.template_presets {
        let entry = JsonLineEntry::TemplatePreset(template.clone());
        if let Ok(line) = serde_json::to_string(&entry) {
            lines.push(line);
        }
    }

    for layout in &export.document_layouts {
        let entry = JsonLineEntry::DocumentLayout(layout.clone());
        if let Ok(line) = serde_json::to_string(&entry) {
            lines.push(line);
        }
    }

    lines.join("\n") + "\n"
}

/// Parses a JSON Lines string back into `UserConfigsExport`.
pub fn parse_json_lines_to_export(content: &str) -> Result<UserConfigsExport, String> {
    let mut configs = HashMap::new();
    let mut record_export_presets = Vec::new();
    let mut template_presets = Vec::new();
    let mut document_layouts = Vec::new();

    for (line_idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let entry: JsonLineEntry = serde_json::from_str(trimmed)
            .map_err(|e| format!("Failed to parse line {}: {}", line_idx + 1, e))?;

        match entry {
            JsonLineEntry::Config { key, value } => {
                configs.insert(key, value);
            }
            JsonLineEntry::RecordExportPreset(preset) => {
                record_export_presets.push(preset);
            }
            JsonLineEntry::TemplatePreset(template) => {
                template_presets.push(template);
            }
            JsonLineEntry::DocumentLayout(layout) => {
                document_layouts.push(layout);
            }
        }
    }

    Ok(UserConfigsExport {
        schema_version: crate::USER_CONFIG_SCHEMA_VERSION,
        configs,
        record_export_presets,
        template_presets,
        document_layouts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ConfigCombinedField, ConfigExportPreset, DocumentLayoutBlock};
    use serde_json::json;

    #[test]
    fn test_json_lines_serialization_deserialization() {
        let mut configs = HashMap::new();
        configs.insert("siteTypeFmt".to_string(), json!("anyCase"));
        configs.insert("collectorFieldKey".to_string(), json!(false));
        configs.insert("tissueIDNumberKey".to_string(), json!(42));
        configs.insert("siteTypes".to_string(), json!(["Forest", "Stream"]));

        let preset = ConfigExportPreset {
            fields: {
                let mut m = HashMap::new();
                m.insert("id".to_string(), "Identifier".to_string());
                m
            },
            combined_fields: vec![ConfigCombinedField {
                field_id: "comb".to_string(),
                fields: vec!["f1".to_string(), "f2".to_string()],
            }],
        };

        let record_export_presets = vec![ConfigPresetEntry {
            name: "test_preset".to_string(),
            preset,
        }];

        let template_presets = vec![TemplatePresetEntry {
            name: "test_template".to_string(),
            record_type: "specimen".to_string(),
            description: "".to_string(),
            value: json!({
                "name": "test_template",
                "page1": {},
                "page2": {}
            }),
        }];

        let document_layouts = vec![DocumentLayoutPreset {
            name: "test_layout".to_string(),
            layout_type: "WholePage".to_string(),
            page_size_key: "Letter".to_string(),
            page_orientation: "portrait".to_string(),
            custom_page_width_mm: None,
            custom_page_height_mm: None,
            page_pad_top_mm: 8.0,
            page_pad_left_mm: 8.0,
            page_pad_right_mm: 8.0,
            page_pad_bottom_mm: 8.0,
            blocks: vec![DocumentLayoutBlock {
                template_name: "test_template".to_string(),
                template_count: 1,
                rows: 8,
                cols: 4,
                template_pad_top_mm: 1.0,
                template_pad_left_mm: 1.0,
                template_pad_right_mm: 1.0,
                template_pad_bottom_mm: 1.0,
                page_break_after: false,
            }],
            fill_page: false,
            multi_block_mode: "Continuous".to_string(),
        }];

        let export = UserConfigsExport {
            schema_version: crate::USER_CONFIG_SCHEMA_VERSION,
            configs,
            record_export_presets,
            template_presets,
            document_layouts,
        };

        let json_lines_str = export_to_json_lines(&export);
        let imported = parse_json_lines_to_export(&json_lines_str).unwrap();

        assert_eq!(imported.configs.get("siteTypeFmt").unwrap(), "anyCase");
        assert_eq!(
            imported.configs.get("collectorFieldKey").unwrap(),
            &json!(false)
        );
        assert_eq!(
            imported.configs.get("tissueIDNumberKey").unwrap(),
            &json!(42)
        );
        assert_eq!(
            imported.configs.get("siteTypes").unwrap(),
            &json!(["Forest", "Stream"])
        );
        assert_eq!(imported.record_export_presets.len(), 1);
        assert_eq!(imported.record_export_presets[0].name, "test_preset");
        assert_eq!(
            imported.record_export_presets[0]
                .preset
                .fields
                .get("id")
                .unwrap(),
            "Identifier"
        );
        assert_eq!(imported.template_presets.len(), 1);
        assert_eq!(imported.template_presets[0].name, "test_template");
        assert_eq!(
            imported.template_presets[0].value.get("name").unwrap(),
            "test_template"
        );
        assert_eq!(imported.document_layouts.len(), 1);
        assert_eq!(imported.document_layouts[0].name, "test_layout");
        assert_eq!(imported.document_layouts[0].layout_type, "WholePage");
        assert_eq!(imported.document_layouts[0].blocks.len(), 1);
        assert_eq!(
            imported.document_layouts[0].blocks[0].template_name,
            "test_template"
        );
    }
}
