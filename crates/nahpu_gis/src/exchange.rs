//! High-level GIS import, conversion, and export interfaces.

use std::path::{Path, PathBuf};

use crate::{
    error::GisError,
    io::{
        coordinates::CoordinateImportResult,
        geojson::GeoJsonExporter,
        kml::KmlExporter,
        layers::{ConvertedVectorLayer, convert_vector_to_geojson},
        shp::ShapefileExporter,
        topojson::TopoJsonExporter,
    },
    types::CoordinateData,
};

/// Coordinate file formats supported by the exporter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordinateFormat {
    /// GeoJSON feature collection.
    GeoJson,
    /// TopoJSON point topology.
    TopoJson,
    /// Keyhole Markup Language document.
    Kml,
    /// Zipped ESRI Shapefile dataset.
    Shapefile,
}

/// Validated exporter for a collection of point coordinates.
pub struct CoordinateExporter<'a> {
    coordinates: &'a [CoordinateData],
}

impl<'a> CoordinateExporter<'a> {
    /// Creates an exporter after validating every coordinate.
    pub fn new(coordinates: &'a [CoordinateData]) -> Result<Self, GisError> {
        if coordinates.is_empty() {
            return Err(GisError::InvalidCoordinate(
                "at least one coordinate is required".to_owned(),
            ));
        }
        for (index, coordinate) in coordinates.iter().enumerate() {
            coordinate.validate_for_export(index)?;
        }
        Ok(Self { coordinates })
    }

    /// Writes the coordinates in `format` to `path`.
    pub fn export(&self, format: CoordinateFormat, path: impl AsRef<Path>) -> Result<(), GisError> {
        let path = path.as_ref();
        let result = match format {
            CoordinateFormat::GeoJson => GeoJsonExporter::new(self.coordinates).export(path),
            CoordinateFormat::TopoJson => TopoJsonExporter::new(self.coordinates).export(path),
            CoordinateFormat::Kml => KmlExporter::new(self.coordinates).export(path),
            CoordinateFormat::Shapefile => ShapefileExporter::new(self.coordinates).export(path),
        };
        result.map_err(GisError::Operation)
    }
}

/// Imports point coordinates from a supported GIS file.
pub struct CoordinateImporter {
    path: PathBuf,
}

impl CoordinateImporter {
    /// Creates an importer for `path`.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Imports GeoJSON, KML, GPX, or a zipped Shapefile.
    pub fn import(&self) -> Result<CoordinateImportResult, GisError> {
        CoordinateImportResult::import(&self.path).map_err(GisError::Operation)
    }
}

/// Converts a vector layer to normalized WGS84 GeoJSON.
pub struct VectorLayerConverter {
    input_path: PathBuf,
    output_path: PathBuf,
}

impl VectorLayerConverter {
    /// Creates a converter with explicit input and output paths.
    pub fn new(input_path: impl Into<PathBuf>, output_path: impl Into<PathBuf>) -> Self {
        Self {
            input_path: input_path.into(),
            output_path: output_path.into(),
        }
    }

    /// Converts the input and returns metadata for the normalized layer.
    pub fn convert(&self) -> Result<ConvertedVectorLayer, GisError> {
        convert_vector_to_geojson(&self.input_path, &self.output_path).map_err(GisError::Operation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_an_empty_export() {
        let error = CoordinateExporter::new(&[])
            .err()
            .expect("export should fail");
        assert!(error.to_string().contains("at least one coordinate"));
    }

    #[test]
    fn rejects_an_invalid_coordinate_before_writing() {
        let coordinate = CoordinateData {
            name_id: "invalid".to_owned(),
            notes: None,
            decimal_longitude: Some(10.0),
            decimal_latitude: Some(91.0),
            elevation_in_meter: None,
        };
        let error = CoordinateExporter::new(&[coordinate])
            .err()
            .expect("export should fail");
        assert!(error.to_string().contains("latitude"));
    }

    #[test]
    fn rejects_missing_and_non_finite_export_values() {
        let cases = [
            CoordinateData {
                name_id: "missing".to_owned(),
                notes: None,
                decimal_longitude: None,
                decimal_latitude: Some(1.0),
                elevation_in_meter: None,
            },
            CoordinateData {
                name_id: "longitude".to_owned(),
                notes: None,
                decimal_longitude: Some(f64::NAN),
                decimal_latitude: Some(1.0),
                elevation_in_meter: None,
            },
            CoordinateData {
                name_id: "elevation".to_owned(),
                notes: None,
                decimal_longitude: Some(1.0),
                decimal_latitude: Some(1.0),
                elevation_in_meter: Some(f64::INFINITY),
            },
        ];

        for coordinate in cases {
            assert!(CoordinateExporter::new(&[coordinate]).is_err());
        }
    }
}
