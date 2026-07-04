use crate::models::{ConfigExportPreset, ConfigPresetEntry, UserConfigsExport};
use kdl::{KdlDocument, KdlNode, KdlValue, KdlEntry};
use std::collections::HashMap;

/// Helper to convert a JSON value to a KDL value.
fn json_to_kdl_value(val: &serde_json::Value) -> Option<KdlValue> {
    match val {
        serde_json::Value::String(s) => Some(KdlValue::String(s.clone())),
        serde_json::Value::Bool(b) => Some(KdlValue::Bool(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(KdlValue::Integer(i as i128))
            } else if let Some(f) = n.as_f64() {
                Some(KdlValue::Float(f))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Helper to convert a KDL value back to a JSON value.
fn kdl_value_to_json(value: &KdlValue) -> serde_json::Value {
    match value {
        KdlValue::String(s) => serde_json::Value::String(s.clone()),
        KdlValue::Bool(b) => serde_json::Value::Bool(*b),
        KdlValue::Integer(i) => serde_json::Value::Number((*i as i64).into()),
        KdlValue::Float(f) => {
            if let Some(n) = serde_json::Number::from_f64(*f) {
                serde_json::Value::Number(n)
            } else {
                serde_json::Value::Null
            }
        }
        KdlValue::Null => serde_json::Value::Null,
    }
}

/// Serializes the configs to a KDL string using the `kdl` crate.
pub fn export_to_kdl(export: &UserConfigsExport) -> String {
    let mut doc = KdlDocument::new();

    // Sort keys to have deterministic output
    let mut config_keys: Vec<&String> = export.configs.keys().collect();
    config_keys.sort();

    for key in config_keys {
        let val = &export.configs[key.as_str()];
        let mut node = KdlNode::new(key.as_str());

        match val {
            serde_json::Value::String(s) => {
                node.entries_mut()
                    .push(KdlEntry::new(KdlValue::String(s.clone())));
                doc.nodes_mut().push(node);
            }
            serde_json::Value::Bool(b) => {
                node.entries_mut().push(KdlEntry::new(KdlValue::Bool(*b)));
                doc.nodes_mut().push(node);
            }
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    node.entries_mut().push(KdlEntry::new(KdlValue::Integer(i as i128)));
                    doc.nodes_mut().push(node);
                } else if let Some(f) = n.as_f64() {
                    node.entries_mut().push(KdlEntry::new(KdlValue::Float(f)));
                    doc.nodes_mut().push(node);
                }
            }
            serde_json::Value::Array(arr) => {
                for item in arr {
                    if let Some(kval) = json_to_kdl_value(item) {
                        node.entries_mut().push(KdlEntry::new(kval));
                    }
                }
                doc.nodes_mut().push(node);
            }
            _ => {}
        }
    }

    // For document presets, serialize to JSON string and store in KDL
    if let Ok(presets_str) = serde_json::to_string(&export.document_presets) {
        let mut node = KdlNode::new("document_presets");
        node.entries_mut()
            .push(KdlEntry::new(KdlValue::String(presets_str)));
        doc.nodes_mut().push(node);
    }

    doc.to_string()
}

/// Parses a KDL string back into `UserConfigsExport` using the `kdl` crate.
pub fn parse_kdl_to_export(content: &str) -> Result<UserConfigsExport, String> {
    let doc: KdlDocument = content.parse().map_err(|e: kdl::KdlError| e.to_string())?;

    let mut configs = HashMap::new();
    let mut document_presets = Vec::new();

    for node in doc.nodes() {
        let name = node.name().value().to_string();
        let entries = node.entries();

        if name == "document_presets" || name == "exportPresets" {
            if let Some(entry) = entries.first() {
                if let KdlValue::String(json_str) = entry.value() {
                    if let Ok(presets) = serde_json::from_str::<Vec<ConfigPresetEntry>>(json_str) {
                        document_presets = presets;
                    } else if let Ok(presets_map) =
                        serde_json::from_str::<HashMap<String, ConfigExportPreset>>(json_str)
                    {
                        // Support old format where it was a map
                        for (name, preset) in presets_map {
                            document_presets.push(ConfigPresetEntry { name, preset });
                        }
                    }
                }
            }
        } else {
            if entries.is_empty() {
                continue;
            }

            let val = if entries.len() == 1 {
                kdl_value_to_json(entries[0].value())
            } else {
                let arr: Vec<serde_json::Value> = entries
                    .iter()
                    .map(|e| kdl_value_to_json(e.value()))
                    .collect();
                serde_json::Value::Array(arr)
            };
            configs.insert(name, val);
        }
    }

    Ok(UserConfigsExport {
        configs,
        document_presets,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ConfigCombinedField, ConfigExportPreset, ConfigPresetEntry, UserConfigsExport};
    use serde_json::json;

    #[test]
    fn test_kdl_serialization_deserialization() {
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

        let document_presets = vec![ConfigPresetEntry {
            name: "test_preset".to_string(),
            preset,
        }];

        let export = UserConfigsExport {
            configs,
            document_presets,
        };

        let kdl_str = export_to_kdl(&export);
        let imported = parse_kdl_to_export(&kdl_str).unwrap();

        assert_eq!(imported.configs.get("siteTypeFmt").unwrap(), "anyCase");
        assert_eq!(imported.configs.get("collectorFieldKey").unwrap(), &json!(false));
        assert_eq!(imported.configs.get("tissueIDNumberKey").unwrap(), &json!(42));
        assert_eq!(
            imported.configs.get("siteTypes").unwrap(),
            &json!(["Forest", "Stream"])
        );
        assert_eq!(imported.document_presets.len(), 1);
        assert_eq!(imported.document_presets[0].name, "test_preset");
        assert_eq!(
            imported.document_presets[0].preset.fields.get("id").unwrap(),
            "Identifier"
        );
    }
}
