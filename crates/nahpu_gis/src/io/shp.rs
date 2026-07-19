//! Read and write shapefile.

use crate::types::CoordinateData;
use nahpu_archive::archive::ZipArchive;
use shapefile::{PointZ, Writer};
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) struct ShapefileExporter<'a> {
    data: &'a [CoordinateData],
}

impl<'a> ShapefileExporter<'a> {
    pub(crate) fn new(data: &'a [CoordinateData]) -> Self {
        Self { data }
    }

    pub(crate) fn export(&self, output_zip_path: &Path) -> Result<(), String> {
        let temp_dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let shp_path = temp_dir.path().join("coordinates.shp");

        let name_field = dbase::FieldName::try_from("nameId").map_err(|error| error.to_string())?;
        let notes_field = dbase::FieldName::try_from("notes").map_err(|error| error.to_string())?;
        let table_builder = dbase::TableWriterBuilder::new()
            .add_character_field(name_field, 254)
            .add_character_field(notes_field, 254);
        let mut writer = Writer::from_path(&shp_path, table_builder).map_err(|e| e.to_string())?;

        for coord in self.data {
            if let (Some(lon), Some(lat)) = (coord.decimal_longitude, coord.decimal_latitude) {
                let elev = coord.elevation_in_meter.unwrap_or(0.0);
                let point = PointZ::new(lon, lat, elev, 0.0);

                let mut record = dbase::Record::default();
                record.insert(
                    "nameId".to_string(),
                    dbase::FieldValue::Character(Some(coord.name_id.clone())),
                );
                if let Some(notes) = &coord.notes {
                    record.insert(
                        "notes".to_string(),
                        dbase::FieldValue::Character(Some(notes.clone())),
                    );
                }

                writer
                    .write_shape_and_record(&point, &record)
                    .map_err(|e| e.to_string())?;
            }
        }

        // We must drop the writer so it flushes the files to disk.
        drop(writer);

        let prj_path = temp_dir.path().join("coordinates.prj");
        let wgs84_prj = concat!(
            r#"GEOGCS["GCS_WGS_1984",DATUM["D_WGS_1984","#,
            r#"SPHEROID["WGS_1984",6378137.0,298.257223563]],"#,
            r#"PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]]"#,
        );
        fs::write(&prj_path, wgs84_prj).map_err(|e| e.to_string())?;

        let files: Vec<PathBuf> = fs::read_dir(temp_dir.path())
            .map_err(|e| e.to_string())?
            .map(|entry| {
                entry
                    .map(|value| value.path())
                    .map_err(|error| error.to_string())
            })
            .collect::<Result<_, _>>()?;

        let archiver = ZipArchive::new(temp_dir.path(), None, output_zip_path, &files);
        archiver.write().map_err(|e| e.to_string())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_export_shp() {
        let coord = CoordinateData {
            name_id: "test".to_string(),
            decimal_latitude: Some(1.0),
            decimal_longitude: Some(2.0),
            elevation_in_meter: Some(3.0),
            notes: Some("test notes".to_string()),
        };

        let temp_dir = tempdir().unwrap();
        let path = temp_dir.path().join("test.zip");

        let coords = [coord];
        let exporter = ShapefileExporter::new(&coords);
        exporter
            .export(&path)
            .expect("Shapefile export should succeed");

        assert!(path.exists());
    }
}
