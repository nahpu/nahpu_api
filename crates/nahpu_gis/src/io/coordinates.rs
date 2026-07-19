//! Import coordinate points from NAHPU GIS exchange formats.

use crate::io::layers::{read_geojson, read_zipped_shapefile};
use crate::types::CoordinateData;
use geojson::{Feature, GeometryValue};
use kml::{Kml, KmlReader, types::Geometry as KmlGeometry};
use quick_xml::{Reader, events::Event};
use std::{fs, path::Path};

#[derive(Debug, Clone)]
/// Coordinates and diagnostics produced by a GIS file import.
pub struct CoordinateImportResult {
    /// Valid point coordinates read from the input.
    pub coordinates: Vec<CoordinateData>,
    /// Number of unsupported or invalid records skipped during import.
    pub skipped_count: u64,
    /// Human-readable import diagnostics.
    pub warnings: Vec<String>,
}

impl CoordinateImportResult {
    pub(crate) fn import(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        match extension.as_str() {
            "geojson" | "json" => from_features(read_geojson(path)?.features, path),
            "zip" => from_features(read_zipped_shapefile(path)?.features, path),
            "kml" => from_kml(path),
            "gpx" => from_gpx(path),
            _ => Err(
                "Supported coordinate formats are GeoJSON, KML, zipped Shapefile, and GPX"
                    .to_owned(),
            ),
        }
    }
}

fn from_features(features: Vec<Feature>, path: &Path) -> Result<CoordinateImportResult, String> {
    let stem = file_stem(path);
    let mut coordinates = Vec::new();
    let mut skipped_count = 0;
    for (index, feature) in features.into_iter().enumerate() {
        let Some(geometry) = feature.geometry else {
            skipped_count += 1;
            continue;
        };
        let GeometryValue::Point { coordinates: point } = geometry.value else {
            skipped_count += 1;
            continue;
        };
        if point.len() < 2 || !is_valid_wgs84(point[1], point[0]) {
            skipped_count += 1;
            continue;
        }
        let properties = feature.properties.unwrap_or_default();
        let name = string_property(&properties, &["name", "nameId"])
            .unwrap_or_else(|| format!("{stem}-{}", index + 1));
        let notes = string_property(&properties, &["description", "notes"]);
        coordinates.push(CoordinateData {
            name_id: name,
            notes,
            decimal_longitude: Some(point[0]),
            decimal_latitude: Some(point[1]),
            elevation_in_meter: point
                .as_slice()
                .get(2)
                .copied()
                .filter(|value| value.is_finite()),
        });
    }
    let warnings = skipped_warning(skipped_count);
    Ok(CoordinateImportResult {
        coordinates,
        skipped_count,
        warnings,
    })
}

fn from_kml(path: &Path) -> Result<CoordinateImportResult, String> {
    let root = KmlReader::<_, f64>::from_path(path)
        .map_err(|error| error.to_string())?
        .read()
        .map_err(|error| error.to_string())?;
    let mut result = CoordinateImportResult {
        coordinates: Vec::new(),
        skipped_count: 0,
        warnings: Vec::new(),
    };
    visit_kml(root, &file_stem(path), &mut result);
    result.warnings = skipped_warning(result.skipped_count);
    Ok(result)
}

fn visit_kml(value: Kml<f64>, stem: &str, result: &mut CoordinateImportResult) {
    match value {
        Kml::KmlDocument(document) => {
            for element in document.elements {
                visit_kml(element, stem, result);
            }
        }
        Kml::Document { elements, .. } => {
            for element in elements {
                visit_kml(element, stem, result);
            }
        }
        Kml::Folder(folder) => {
            for element in folder.elements {
                visit_kml(element, stem, result);
            }
        }
        Kml::Placemark(placemark) => match placemark.geometry {
            Some(KmlGeometry::Point(point)) => {
                if !is_valid_wgs84(point.coord.y, point.coord.x) {
                    result.skipped_count += 1;
                    return;
                }
                let fallback = format!("{stem}-{}", result.coordinates.len() + 1);
                result.coordinates.push(CoordinateData {
                    name_id: placemark.name.unwrap_or(fallback),
                    notes: placemark.description,
                    decimal_longitude: Some(point.coord.x),
                    decimal_latitude: Some(point.coord.y),
                    elevation_in_meter: point.coord.z,
                });
            }
            Some(_) => result.skipped_count += 1,
            None => result.skipped_count += 1,
        },
        Kml::Point(point) => {
            if is_valid_wgs84(point.coord.y, point.coord.x) {
                result.coordinates.push(CoordinateData {
                    name_id: format!("{stem}-{}", result.coordinates.len() + 1),
                    notes: None,
                    decimal_longitude: Some(point.coord.x),
                    decimal_latitude: Some(point.coord.y),
                    elevation_in_meter: point.coord.z,
                });
            } else {
                result.skipped_count += 1;
            }
        }
        _ => {}
    }
}

fn from_gpx(path: &Path) -> Result<CoordinateImportResult, String> {
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let mut reader = Reader::from_str(&content);
    reader.config_mut().trim_text(true);
    let stem = file_stem(path);
    let mut coordinates = Vec::new();
    let mut current: Option<CoordinateData> = None;
    let mut field: Option<Vec<u8>> = None;
    let mut skipped_count = 0;
    loop {
        match reader.read_event().map_err(|error| error.to_string())? {
            Event::Start(element) if element.name().as_ref() == b"wpt" => {
                let mut lat = None;
                let mut lon = None;
                for attribute in element.attributes() {
                    let attribute = attribute.map_err(|error| error.to_string())?;
                    let value = attribute
                        .decode_and_unescape_value(reader.decoder())
                        .map_err(|error| error.to_string())?;
                    match attribute.key.as_ref() {
                        b"lat" => lat = value.parse::<f64>().ok(),
                        b"lon" => lon = value.parse::<f64>().ok(),
                        _ => {}
                    }
                }
                current = Some(CoordinateData {
                    name_id: String::new(),
                    notes: None,
                    decimal_longitude: lon,
                    decimal_latitude: lat,
                    elevation_in_meter: None,
                });
            }
            Event::Start(element) if current.is_some() => {
                field = Some(element.name().as_ref().to_vec())
            }
            Event::Text(text) if current.is_some() => {
                let value = text
                    .decode()
                    .map_err(|error| error.to_string())?
                    .into_owned();
                if let Some(coordinate) = current.as_mut() {
                    match field.as_deref() {
                        Some(b"name") => coordinate.name_id = value,
                        Some(b"desc") | Some(b"cmt") if coordinate.notes.is_none() => {
                            coordinate.notes = Some(value);
                        }
                        Some(b"ele") => coordinate.elevation_in_meter = value.parse().ok(),
                        _ => {}
                    }
                }
            }
            Event::End(element) if element.name().as_ref() == b"wpt" => {
                if let Some(mut coordinate) = current.take() {
                    if coordinate
                        .decimal_latitude
                        .zip(coordinate.decimal_longitude)
                        .is_some_and(|(latitude, longitude)| is_valid_wgs84(latitude, longitude))
                    {
                        if coordinate.name_id.trim().is_empty() {
                            coordinate.name_id = format!("{stem}-{}", coordinates.len() + 1);
                        }
                        coordinates.push(coordinate);
                    } else {
                        skipped_count += 1;
                    }
                }
                field = None;
            }
            Event::End(_) => field = None,
            Event::Eof => break,
            _ => {}
        }
    }
    let has_tracks = content.contains("<trk") || content.contains("<rte");
    let mut warnings = skipped_warning(skipped_count);
    if has_tracks {
        warnings
            .push("GPX routes and tracks were ignored; only waypoints are imported.".to_owned());
    }
    Ok(CoordinateImportResult {
        coordinates,
        skipped_count,
        warnings,
    })
}

fn string_property(
    properties: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> Option<String> {
    keys.iter().find_map(|key| {
        properties
            .get(*key)?
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    })
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("coordinate")
        .to_owned()
}

fn skipped_warning(skipped_count: u64) -> Vec<String> {
    if skipped_count == 0 {
        Vec::new()
    } else {
        vec![format!(
            "{skipped_count} unsupported or invalid coordinate records were skipped."
        )]
    }
}

fn is_valid_wgs84(latitude: f64, longitude: f64) -> bool {
    latitude.is_finite()
        && longitude.is_finite()
        && (-90.0..=90.0).contains(&latitude)
        && (-180.0..=180.0).contains(&longitude)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn imports_geojson_points_and_skips_lines() {
        let directory = tempdir().expect("temporary directory should be created");
        let path = directory.path().join("field.geojson");
        fs::write(
            &path,
            concat!(
                r#"{"type":"FeatureCollection","features":["#,
                r#"{"type":"Feature","geometry":{"type":"Point","#,
                r#""coordinates":[2,1,3]},"properties":{"name":"A"}},"#,
                r#"{"type":"Feature","geometry":{"type":"LineString","#,
                r#""coordinates":[[0,0],[1,1]]},"properties":{}}]}"#,
            ),
        )
        .expect("test GeoJSON should be written");
        let result =
            CoordinateImportResult::import(&path).expect("coordinate import should succeed");
        assert_eq!(result.coordinates.len(), 1);
        assert_eq!(result.coordinates[0].name_id, "A");
        assert_eq!(result.skipped_count, 1);
    }

    #[test]
    fn imports_gpx_waypoints_only() {
        let directory = tempdir().expect("temporary directory should be created");
        let path = directory.path().join("field.gpx");
        fs::write(
            &path,
            concat!(
                r#"<gpx><wpt lat="1" lon="2">"#,
                "<ele>3</ele><name>A</name></wpt>",
                "<trk><name>ignored</name></trk></gpx>",
            ),
        )
        .expect("test GPX should be written");
        let result =
            CoordinateImportResult::import(&path).expect("coordinate import should succeed");
        assert_eq!(result.coordinates.len(), 1);
        assert_eq!(result.coordinates[0].elevation_in_meter, Some(3.0));
        assert!(
            result
                .warnings
                .iter()
                .any(|warning| warning.contains("routes and tracks"))
        );
    }
}
