//! Read and write TopoJSON vector geographic data.

use crate::types::CoordinateData;
use serde_json::{Map, Value};
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub struct TopoJsonExporter<'a> {
    data: &'a [CoordinateData],
}

impl<'a> TopoJsonExporter<'a> {
    pub fn new(data: &'a [CoordinateData]) -> Self {
        Self { data }
    }

    pub fn export_topojson(&self, path: &Path) -> Result<(), String> {
        let mut geometries = Vec::new();

        for coord in self.data {
            if let (Some(lon), Some(lat)) = (coord.decimal_longitude, coord.decimal_latitude) {
                let mut geometry = Map::new();
                geometry.insert("type".to_string(), Value::String("Point".to_string()));

                let coords = match coord.elevation_in_meter {
                    Some(elev) => vec![
                        Value::Number(serde_json::Number::from_f64(lon).unwrap()),
                        Value::Number(serde_json::Number::from_f64(lat).unwrap()),
                        Value::Number(serde_json::Number::from_f64(elev).unwrap()),
                    ],
                    None => vec![
                        Value::Number(serde_json::Number::from_f64(lon).unwrap()),
                        Value::Number(serde_json::Number::from_f64(lat).unwrap()),
                    ],
                };
                geometry.insert("coordinates".to_string(), Value::Array(coords));

                let mut properties = Map::new();
                properties.insert("name".to_string(), Value::String(coord.name_id.clone()));
                if let Some(notes) = &coord.notes {
                    properties.insert("description".to_string(), Value::String(notes.clone()));
                }
                geometry.insert("properties".to_string(), Value::Object(properties));

                geometries.push(Value::Object(geometry));
            }
        }

        let mut points_collection = Map::new();
        points_collection.insert(
            "type".to_string(),
            Value::String("GeometryCollection".to_string()),
        );
        points_collection.insert("geometries".to_string(), Value::Array(geometries));

        let mut objects = Map::new();
        objects.insert("points".to_string(), Value::Object(points_collection));

        let mut topology = Map::new();
        topology.insert("type".to_string(), Value::String("Topology".to_string()));
        topology.insert("objects".to_string(), Value::Object(objects));
        topology.insert("arcs".to_string(), Value::Array(Vec::new()));

        let json_string = serde_json::to_string(&topology).map_err(|e| e.to_string())?;

        let mut file = File::create(path).map_err(|e| e.to_string())?;
        file.write_all(json_string.as_bytes())
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_export_topojson() {
        let coord = CoordinateData {
            name_id: "test".to_string(),
            decimal_latitude: Some(1.0),
            decimal_longitude: Some(2.0),
            elevation_in_meter: Some(3.0),
            notes: Some("test notes".to_string()),
        };

        let temp_dir = tempdir().unwrap();
        let path = temp_dir.path().join("test.topojson");

        let coords = [coord];
        let exporter = TopoJsonExporter::new(&coords);
        exporter.export_topojson(&path).unwrap();

        assert!(path.exists());
    }
}
