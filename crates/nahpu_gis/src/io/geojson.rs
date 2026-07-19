//! Read and write GeoJSON vector geographic data.

use crate::types::CoordinateData;
use geojson::{Feature, FeatureCollection, Geometry};
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub(crate) struct GeoJsonExporter<'a> {
    data: &'a [CoordinateData],
}

impl<'a> GeoJsonExporter<'a> {
    pub(crate) fn new(data: &'a [CoordinateData]) -> Self {
        Self { data }
    }

    fn to_feature_collection(&self) -> FeatureCollection {
        let mut features = Vec::new();

        for coord in self.data {
            if let (Some(lon), Some(lat)) = (coord.decimal_longitude, coord.decimal_latitude) {
                let point = match coord.elevation_in_meter {
                    Some(elev) => geojson::GeometryValue::Point {
                        coordinates: vec![lon, lat, elev].into(),
                    },
                    None => geojson::GeometryValue::Point {
                        coordinates: vec![lon, lat].into(),
                    },
                };

                let geometry = Geometry::new(point);
                let mut feature = Feature {
                    bbox: None,
                    geometry: Some(geometry),
                    id: None,
                    properties: None,
                    foreign_members: None,
                };

                let mut properties = serde_json::Map::new();
                properties.insert(
                    "name".to_string(),
                    serde_json::Value::String(coord.name_id.clone()),
                );
                if let Some(notes) = &coord.notes {
                    properties.insert(
                        "description".to_string(),
                        serde_json::Value::String(notes.clone()),
                    );
                }
                feature.properties = Some(properties);

                features.push(feature);
            }
        }

        FeatureCollection {
            bbox: None,
            features,
            foreign_members: None,
        }
    }

    pub(crate) fn export(&self, path: &Path) -> Result<(), String> {
        let feature_collection = self.to_feature_collection();
        let json_string = feature_collection.to_string();

        let mut file = File::create(path).map_err(|e| e.to_string())?;
        file.write_all(json_string.as_bytes())
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_export_geojson() {
        let coord1 = CoordinateData {
            name_id: "Site 1".to_string(),
            notes: Some("Test note".to_string()),
            decimal_longitude: Some(10.0),
            decimal_latitude: Some(20.0),
            elevation_in_meter: Some(100.0),
        };

        let coord2 = CoordinateData {
            name_id: "Site 2".to_string(),
            notes: None,
            decimal_longitude: Some(-10.0),
            decimal_latitude: Some(-20.0),
            elevation_in_meter: None,
        };

        let data = vec![coord1, coord2];
        let exporter = GeoJsonExporter::new(&data);

        let path = Path::new("test_output.geojson");
        exporter
            .export(path)
            .expect("GeoJSON export should succeed");

        assert!(path.exists());
        let content = fs::read_to_string(path).expect("GeoJSON should be readable");
        assert!(content.contains("Site 1"));
        assert!(content.contains("Test note"));
        assert!(content.contains("[10.0,20.0,100.0]"));
        assert!(content.contains("[-10.0,-20.0]"));

        fs::remove_file(path).expect("test output should be removable");
    }
}
