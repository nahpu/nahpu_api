//! Import and normalization helpers for user-provided vector map layers.

use std::{
    fs,
    path::{Path, PathBuf},
};

use dbase::FieldValue;
use geo::Geometry;
use geojson::{Feature, FeatureCollection, GeoJson, Geometry as GeoJsonGeometry, GeometryValue};
use nahpu_archive::zip::ZipExtractor;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value as JsonValue};
use tempfile::tempdir;

/// Metadata returned after a vector layer has been normalized to GeoJSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConvertedVectorLayer {
    pub output_path: String,
    pub feature_count: u64,
    pub bounds: Option<Vec<f64>>,
    pub source_crs: String,
}

/// Converts a GeoJSON file or zipped WGS84 Shapefile into normalized GeoJSON.
pub fn convert_vector_to_geojson(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> Result<ConvertedVectorLayer, String> {
    let input_path = input_path.as_ref();
    let output_path = output_path.as_ref();
    let extension = input_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let collection = match extension.as_str() {
        "geojson" | "json" => read_geojson(input_path)?,
        "zip" => read_zipped_shapefile(input_path)?,
        _ => return Err("Supported vector formats are GeoJSON and zipped Shapefile".to_owned()),
    };
    let bounds = feature_collection_bounds(&collection);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(output_path, GeoJson::from(collection.clone()).to_string())
        .map_err(|error| error.to_string())?;
    Ok(ConvertedVectorLayer {
        output_path: output_path.to_string_lossy().into_owned(),
        feature_count: collection.features.len() as u64,
        bounds,
        source_crs: "EPSG:4326".to_owned(),
    })
}

pub(crate) fn read_geojson(path: &Path) -> Result<FeatureCollection, String> {
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    match content
        .parse::<GeoJson>()
        .map_err(|error| error.to_string())?
    {
        GeoJson::FeatureCollection(collection) => Ok(collection),
        GeoJson::Feature(feature) => Ok(FeatureCollection {
            bbox: feature.bbox.clone(),
            features: vec![feature],
            foreign_members: None,
        }),
        GeoJson::Geometry(geometry) => Ok(FeatureCollection {
            bbox: geometry.bbox.clone(),
            features: vec![Feature {
                geometry: Some(geometry),
                ..Feature::default()
            }],
            foreign_members: None,
        }),
    }
}

pub(crate) fn read_zipped_shapefile(path: &Path) -> Result<FeatureCollection, String> {
    let directory = tempdir().map_err(|error| error.to_string())?;
    ZipExtractor::new(path, directory.path())
        .extract()
        .map_err(|error| error.to_string())?;
    let shapefiles = find_files_with_extension(directory.path(), "shp")?;
    if shapefiles.len() != 1 {
        return Err(format!(
            "A Shapefile ZIP must contain exactly one .shp file; found {}",
            shapefiles.len()
        ));
    }
    let shapefile = &shapefiles[0];
    validate_wgs84_projection(shapefile)?;
    let mut reader = shapefile::Reader::from_path(shapefile).map_err(|error| error.to_string())?;
    let mut features = Vec::new();
    for item in reader.iter_shapes_and_records() {
        let (shape, record) = item.map_err(|error| error.to_string())?;
        let geometry = Geometry::<f64>::try_from(shape).map_err(|error| error.to_string())?;
        let properties = record
            .into_iter()
            .map(|(key, value)| (key, field_value_to_json(value)))
            .collect::<Map<String, JsonValue>>();
        features.push(Feature {
            geometry: Some(GeoJsonGeometry::new(GeometryValue::from(&geometry))),
            properties: Some(properties),
            ..Feature::default()
        });
    }
    Ok(FeatureCollection {
        features,
        ..FeatureCollection::default()
    })
}

fn find_files_with_extension(directory: &Path, extension: &str) -> Result<Vec<PathBuf>, String> {
    let mut matches = Vec::new();
    for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            matches.extend(find_files_with_extension(&path, extension)?);
        } else if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case(extension))
        {
            matches.push(path);
        }
    }
    Ok(matches)
}

fn validate_wgs84_projection(shapefile: &Path) -> Result<(), String> {
    let projection = shapefile.with_extension("prj");
    if !projection.exists() {
        return Err("The Shapefile ZIP must include a .prj file declaring WGS84".to_owned());
    }
    let value = fs::read_to_string(projection)
        .map_err(|error| error.to_string())?
        .to_ascii_uppercase();
    if value.contains("WGS_1984") || value.contains("WGS 84") || value.contains("EPSG\",4326") {
        Ok(())
    } else {
        Err("Only WGS84 (EPSG:4326) Shapefiles are currently supported".to_owned())
    }
}

fn field_value_to_json(value: FieldValue) -> JsonValue {
    match value {
        FieldValue::Character(value) => value.map_or(JsonValue::Null, JsonValue::String),
        FieldValue::Numeric(value) => number_or_null(value),
        FieldValue::Logical(value) => value.map_or(JsonValue::Null, JsonValue::Bool),
        FieldValue::Float(value) => number_or_null(value.map(f64::from)),
        FieldValue::Integer(value) => JsonValue::Number(Number::from(value)),
        FieldValue::Currency(value) | FieldValue::Double(value) => number_or_null(Some(value)),
        FieldValue::Memo(value) => JsonValue::String(value),
        FieldValue::Date(value) => value
            .map(|value| JsonValue::String(value.to_string()))
            .unwrap_or(JsonValue::Null),
        FieldValue::DateTime(value) => JsonValue::String(format!("{value:?}")),
    }
}

fn number_or_null(value: Option<f64>) -> JsonValue {
    value
        .and_then(Number::from_f64)
        .map(JsonValue::Number)
        .unwrap_or(JsonValue::Null)
}

fn feature_collection_bounds(collection: &FeatureCollection) -> Option<Vec<f64>> {
    let mut bounds = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for geometry in collection
        .features
        .iter()
        .filter_map(|feature| feature.geometry.as_ref())
    {
        visit_coordinates(&geometry.value, &mut bounds);
    }
    bounds[0].is_finite().then(|| bounds.to_vec())
}

fn visit_coordinates(value: &GeometryValue, bounds: &mut [f64; 4]) {
    match value {
        GeometryValue::Point { coordinates } => update_bounds(coordinates.as_slice(), bounds),
        GeometryValue::MultiPoint { coordinates } | GeometryValue::LineString { coordinates } => {
            for point in coordinates {
                update_bounds(point.as_slice(), bounds);
            }
        }
        GeometryValue::MultiLineString { coordinates } | GeometryValue::Polygon { coordinates } => {
            for line in coordinates {
                for point in line {
                    update_bounds(point.as_slice(), bounds);
                }
            }
        }
        GeometryValue::MultiPolygon { coordinates } => {
            for polygon in coordinates {
                for line in polygon {
                    for point in line {
                        update_bounds(point.as_slice(), bounds);
                    }
                }
            }
        }
        GeometryValue::GeometryCollection { geometries } => {
            for geometry in geometries {
                visit_coordinates(&geometry.value, bounds);
            }
        }
    }
}

fn update_bounds(point: &[f64], bounds: &mut [f64; 4]) {
    if point.len() < 2 || !point[0].is_finite() || !point[1].is_finite() {
        return;
    }
    bounds[0] = bounds[0].min(point[0]);
    bounds[1] = bounds[1].min(point[1]);
    bounds[2] = bounds[2].max(point[0]);
    bounds[3] = bounds[3].max(point[1]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_geojson_and_reports_metadata() {
        let directory = tempdir().unwrap();
        let input = directory.path().join("input.geojson");
        let output = directory.path().join("output.geojson");
        fs::write(
            &input,
            concat!(
                r#"{"type":"Feature","geometry":{"type":"Point","#,
                r#""coordinates":[-91.5,36.2]},"properties":{"name":"site"}}"#,
            ),
        )
        .unwrap();

        let result = convert_vector_to_geojson(&input, &output).unwrap();

        assert_eq!(result.feature_count, 1);
        assert_eq!(result.bounds, Some(vec![-91.5, 36.2, -91.5, 36.2]));
        assert!(output.exists());
    }
}
