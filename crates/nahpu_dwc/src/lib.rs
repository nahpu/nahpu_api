//! # NAHPU Darwin Core (DwC) Converter
//!
//! `nahpu_dwc` provides data structures and conversion utilities to map NAHPU
//! data types to Darwin Core (DwC) standards.
//!
//! It auto-generates Rust structs from the NAHPU SQLite drift schema and
//! implements logic to serialize NAHPU records into Darwin Core compliant formats.

pub mod dwc;
pub mod export;
pub mod types;

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[derive(Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct DummyProject {
        uuid: String,
        name: String,
        unmapped_field: Option<String>,
        null_field: Option<String>,
    }

    #[test]
    fn test_json_conversion() {
        let project = DummyProject {
            uuid: "1234-5678".to_string(),
            name: "My Project".to_string(),
            unmapped_field: Some("Data".to_string()),
            null_field: None,
        };

        let result = export::json::convert_to_dwc_json("project", &project).unwrap();

        assert_eq!(result["dcterms:identifier"], json!("1234-5678"));
        assert_eq!(result["dwc:datasetName"], json!("My Project"));
        assert_eq!(result["unmappedField"], json!("Data")); // Falls back to original camelCase name
        assert!(result.get("nullField").is_none()); // Nulls are excluded
    }
}
