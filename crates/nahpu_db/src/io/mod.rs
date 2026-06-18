pub mod export;
pub mod import;

use polars::prelude::{DataFrame, SerReader, SerWriter};
use serde::{Serialize, de::DeserializeOwned};

pub trait NahpuExport {
    /// Convert the collection to a Polars DataFrame.
    fn to_dataframe(&self) -> Result<DataFrame, String>;
}

pub trait NahpuImport: Sized {
    /// Create a collection from a Polars DataFrame.
    fn from_dataframe(df: &mut DataFrame) -> Result<Vec<Self>, String>;
}

impl<T> NahpuExport for [T]
where
    T: Serialize,
{
    fn to_dataframe(&self) -> Result<DataFrame, String> {
        let json_bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        let cursor = std::io::Cursor::new(json_bytes);
        polars::prelude::JsonReader::new(cursor)
            .finish()
            .map_err(|e| e.to_string())
    }
}

impl<T> NahpuExport for Vec<T>
where
    T: Serialize,
{
    fn to_dataframe(&self) -> Result<DataFrame, String> {
        self.as_slice().to_dataframe()
    }
}

impl<T> NahpuImport for T
where
    T: DeserializeOwned,
{
    fn from_dataframe(df: &mut DataFrame) -> Result<Vec<Self>, String> {
        let mut buf = Vec::new();
        polars::prelude::JsonWriter::new(&mut buf)
            .with_json_format(polars::prelude::JsonFormat::Json)
            .finish(df)
            .map_err(|e| e.to_string())?;
        serde_json::from_slice(&buf).map_err(|e| e.to_string())
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
        let mut df = sites.to_dataframe().unwrap();

        let path = PathBuf::from("test_export.csv");
        super::export::export_csv(&mut df, &path).unwrap();

        let mut imported_df = super::import::import_csv(&path).unwrap();
        let imported_sites: Vec<Site> = Site::from_dataframe(&mut imported_df).unwrap();

        assert_eq!(imported_sites.len(), 2);
        assert_eq!(imported_sites[0].site_id.as_deref(), Some("S1"));
        assert_eq!(imported_sites[1].site_id.as_deref(), Some("S2"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_excel_export_import() {
        let sites = get_dummy_sites();
        let mut df = sites.to_dataframe().unwrap();

        let path = PathBuf::from("test_export.xlsx");
        super::export::export_excel(&mut df, &path).unwrap();

        let mut imported_df = super::import::import_excel(&path, "Sheet1").unwrap();
        let imported_sites: Vec<Site> = Site::from_dataframe(&mut imported_df).unwrap();

        assert_eq!(imported_sites.len(), 2);
        assert_eq!(imported_sites[0].site_id.as_deref(), Some("S1"));
        assert_eq!(imported_sites[1].site_id.as_deref(), Some("S2"));

        let _ = fs::remove_file(path);
    }
}
