pub mod export;
pub mod import;

use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

pub trait NahpuExport {
    /// Convert the collection to a JSON array.
    fn to_json_array(&self) -> Result<Vec<Value>, String>;
}

pub trait NahpuImport: Sized {
    /// Create a collection from a JSON array.
    fn from_json_array(data: &[Value]) -> Result<Vec<Self>, String>;
}

impl<T: Serialize> NahpuExport for [T] {
    fn to_json_array(&self) -> Result<Vec<Value>, String> {
        let json_value = serde_json::to_value(self).map_err(|e| e.to_string())?;
        match json_value {
            Value::Array(arr) => Ok(arr),
            _ => Err("Expected array format".to_string()),
        }
    }
}

impl<T: Serialize> NahpuExport for Vec<T> {
    fn to_json_array(&self) -> Result<Vec<Value>, String> {
        self.as_slice().to_json_array()
    }
}

impl<T: DeserializeOwned> NahpuImport for T {
    fn from_json_array(data: &[Value]) -> Result<Vec<Self>, String> {
        let json_value = Value::Array(data.to_vec());
        serde_json::from_value(json_value).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::nahpu_sqlite::Site;
    use std::fs;
    use std::path::PathBuf;

    fn get_dummy_sites() -> Vec<Site> {
        vec![
            Site {
                id: 1,
                site_id: Some("S1".to_string()),
                project_uuid: Some("uuid-1".to_string()),
                lead_staff_id: None,
                site_type: Some("Forest".to_string()),
                country: Some("USA".to_string()),
                state_province: Some("California".to_string()),
                county: None,
                municipality: None,
                media_id: None,
                locality: Some("Yosemite".to_string()),
                remark: None,
                habitat_type: None,
                habitat_condition: None,
                habitat_description: None,
            },
            Site {
                id: 2,
                site_id: Some("S2".to_string()),
                project_uuid: Some("uuid-2".to_string()),
                lead_staff_id: None,
                site_type: Some("Desert".to_string()),
                country: Some("USA".to_string()),
                state_province: Some("Nevada".to_string()),
                county: None,
                municipality: None,
                media_id: None,
                locality: Some("Mojave".to_string()),
                remark: None,
                habitat_type: None,
                habitat_condition: None,
                habitat_description: None,
            },
        ]
    }

    #[test]
    fn test_csv_export_import() {
        let sites = get_dummy_sites();
        let data = sites.to_json_array().unwrap();
        // Just extract a couple headers for test
        let cols: Vec<String> = data[0].as_object().unwrap().keys().cloned().collect();

        let path = PathBuf::from("test_export.csv");
        let exporter = super::export::RecordExporter::new(&data, &cols, true);
        exporter.export_csv(&path).unwrap();

        let importer = super::import::RecordImporter::new(&path);
        let imported_data = importer.import_csv().unwrap();
        let imported_sites: Vec<Site> = Site::from_json_array(&imported_data).unwrap();

        assert_eq!(imported_sites.len(), 2);
        assert_eq!(imported_sites[0].site_id.as_deref(), Some("S1"));
        assert_eq!(imported_sites[1].site_id.as_deref(), Some("S2"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_csv_export_import_raw() {
        let sites = get_dummy_sites();
        let data = sites.to_json_array().unwrap();
        let cols: Vec<String> = data[0].as_object().unwrap().keys().cloned().collect();

        let path = PathBuf::from("test_export_raw.csv");
        let exporter = super::export::RecordExporter::new(&data, &cols, true);
        exporter.export_csv(&path).unwrap();

        let importer = super::import::RecordImporter::new(&path);
        let raw_data = importer.import_delimited_raw(b',').unwrap();

        assert_eq!(raw_data.len(), 3); // 1 header + 2 rows
        assert_eq!(raw_data[0].len(), cols.len());
        assert_eq!(raw_data[0], cols);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_excel_export_import() {
        let sites = get_dummy_sites();
        let data = sites.to_json_array().unwrap();
        let cols: Vec<String> = data[0].as_object().unwrap().keys().cloned().collect();

        let path = PathBuf::from("test_export.xlsx");
        let exporter = super::export::RecordExporter::new(&data, &cols, true);
        exporter.export_excel(&path).unwrap();

        let importer = super::import::RecordImporter::new(&path);
        let imported_data = importer.import_excel("Sheet1").unwrap();
        let imported_sites: Vec<Site> = Site::from_json_array(&imported_data).unwrap();

        assert_eq!(imported_sites.len(), 2);
        assert_eq!(imported_sites[0].site_id.as_deref(), Some("S1"));
        assert_eq!(imported_sites[1].site_id.as_deref(), Some("S2"));

        let sheet_names = importer.get_excel_sheet_names().unwrap();
        assert_eq!(sheet_names, vec!["Sheet1"]);

        let raw_data = importer.import_excel_raw("Sheet1").unwrap();
        assert_eq!(raw_data.len(), 3); // 1 header + 2 rows
        assert_eq!(raw_data[0].len(), cols.len());

        let _ = fs::remove_file(path);
    }
}
