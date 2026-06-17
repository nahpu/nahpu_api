use serde::Serialize;
use serde_json::Value;
use std::fs::File;
use std::path::Path;

/// Builder for generating a Darwin Core Data Package (Frictionless Data Package).
pub struct DataPackageBuilder {
    name: String,
    resources: Vec<Value>,
}

impl DataPackageBuilder {
    /// Creates a new `DataPackageBuilder` with the specified package name.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            resources: Vec::new(),
        }
    }

    /// Adds a resource (table) to the Data Package.
    /// Serializes the records to a CSV file in the `output_dir` using Darwin Core headers,
    /// and records the schema for `datapackage.json`.
    pub fn add_resource<T: Serialize>(
        &mut self,
        table_name: &str,
        records: &[T],
        output_dir: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if records.is_empty() {
            return Ok(());
        }

        let csv_filename = format!("{}.csv", table_name);
        let csv_path = output_dir.join(&csv_filename);
        let mut writer = csv::Writer::from_path(&csv_path)?;

        // Serialize the first record to extract the field keys
        let first_record_val = serde_json::to_value(&records[0])?;
        let keys: Vec<String> = if let Value::Object(map) = first_record_val {
            map.keys().cloned().collect()
        } else {
            return Err("Records must serialize to JSON objects".into());
        };

        let mut dwc_headers = Vec::new();
        let mut schema_fields = Vec::new();

        // Write CSV headers mapped to Darwin Core terms
        for key in &keys {
            let dwc_term = crate::dwc::DwcMapper::get_dwc_term(table_name, key).unwrap_or(key);
            dwc_headers.push(dwc_term.to_string());

            schema_fields.push(serde_json::json!({
                "name": dwc_term,
                "type": "string"
            }));
        }
        writer.write_record(&dwc_headers)?;

        // Write rows
        for record in records {
            let val = serde_json::to_value(record)?;
            if let Value::Object(map) = val {
                let mut row = Vec::new();
                for key in &keys {
                    let field_val = map.get(key).unwrap_or(&Value::Null);
                    let str_val = match field_val {
                        Value::String(s) => s.to_string(),
                        Value::Null => "".to_string(),
                        // Simple serialization for numbers, booleans, etc.
                        _ => field_val.to_string(),
                    };
                    row.push(str_val);
                }
                writer.write_record(&row)?;
            }
        }
        writer.flush()?;

        // Register the resource in the Data Package metadata
        self.resources.push(serde_json::json!({
            "name": table_name,
            "path": csv_filename,
            "profile": "tabular-data-resource",
            "schema": {
                "fields": schema_fields
            }
        }));

        Ok(())
    }

    /// Finalizes the Data Package by writing `datapackage.json` into the `output_dir`.
    pub fn build(&self, output_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let dp = serde_json::json!({
            "name": self.name,
            "profile": "data-package",
            "resources": self.resources
        });

        let dp_path = output_dir.join("datapackage.json");
        let file = File::create(dp_path)?;
        serde_json::to_writer_pretty(file, &dp)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct DummySite {
        site_id: String,
        country: String,
    }

    #[test]
    fn test_data_package_generation() {
        let dir = tempdir().unwrap();
        let output_dir = dir.path();

        let sites = vec![
            DummySite {
                site_id: "S1".to_string(),
                country: "USA".to_string(),
            },
            DummySite {
                site_id: "S2".to_string(),
                country: "Canada".to_string(),
            },
        ];

        let mut builder = DataPackageBuilder::new("nahpu_test_dp");
        
        // Add resource
        builder.add_resource("site", &sites, output_dir).unwrap();
        
        // Build datapackage.json
        builder.build(output_dir).unwrap();

        // Verify CSV creation
        let csv_path = output_dir.join("site.csv");
        assert!(csv_path.exists());

        let csv_content = std::fs::read_to_string(&csv_path).unwrap();
        // Check that DWC terms were used as headers (order from serde_json might vary)
        assert!(csv_content.contains("dwc:locationID"));
        assert!(csv_content.contains("dwc:country"));
        assert!(csv_content.contains("S1"));
        assert!(csv_content.contains("USA"));

        // Verify JSON creation
        let dp_path = output_dir.join("datapackage.json");
        assert!(dp_path.exists());

        let dp_content = std::fs::read_to_string(&dp_path).unwrap();
        let dp_json: Value = serde_json::from_str(&dp_content).unwrap();

        assert_eq!(dp_json["name"], "nahpu_test_dp");
        assert_eq!(dp_json["resources"][0]["name"], "site");
        assert_eq!(dp_json["resources"][0]["path"], "site.csv");
        
        let fields = dp_json["resources"][0]["schema"]["fields"].as_array().unwrap();
        let field_names: Vec<&str> = fields.iter()
            .map(|f| f["name"].as_str().unwrap())
            .collect();
        
        assert!(field_names.contains(&"dwc:locationID"));
        assert!(field_names.contains(&"dwc:country"));
    }
}
