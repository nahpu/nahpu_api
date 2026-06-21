//! Read and write Keyhole Markup Language (KML).

use kml::{
    Kml, KmlDocument, KmlWriter,
    types::{Coord, Geometry, Icon, IconStyle, LineStyle, Placemark, Point, PolyStyle, Style},
};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CoordinateData {
    pub name_id: String,
    pub notes: Option<String>,
    pub decimal_longitude: Option<f64>,
    pub decimal_latitude: Option<f64>,
    pub elevation_in_meter: Option<f64>,
}

pub struct KmlExporter<'a> {
    data: &'a [CoordinateData],
}

impl<'a> KmlExporter<'a> {
    pub fn new(data: &'a [CoordinateData]) -> Self {
        Self { data }
    }

    pub fn export_kml(&self, path: &Path) -> Result<(), String> {
        let mut doc = KmlDocument::default();

        let style = Style {
            id: Some("nahpu_style".to_string()),
            icon: Some(IconStyle {
                icon: Icon {
                    href: "http://maps.google.com/mapfiles/kml/shapes/placemark_circle.png"
                        .to_string(),
                    ..Default::default()
                },
                ..Default::default()
            }),
            line: Some(LineStyle {
                color: "ff0000ff".to_string(),
                width: 2.0,
                ..Default::default()
            }),
            poly: Some(PolyStyle {
                color: "7f00ff00".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        doc.elements.push(Kml::Style(style));

        for coord in self.data {
            if let (Some(lon), Some(lat)) = (coord.decimal_longitude, coord.decimal_latitude) {
                let point = Point {
                    coord: Coord {
                        x: lon,
                        y: lat,
                        z: coord.elevation_in_meter,
                    },
                    ..Default::default()
                };

                let placemark = Placemark {
                    name: Some(coord.name_id.clone()),
                    description: coord.notes.clone(),
                    geometry: Some(Geometry::Point(point)),
                    style_url: Some("#nahpu_style".to_string()),
                    ..Default::default()
                };

                doc.elements.push(Kml::Placemark(placemark));
            }
        }

        let file = File::create(path).map_err(|e| e.to_string())?;
        let mut writer = KmlWriter::from_writer(file);
        writer
            .write(&Kml::KmlDocument(doc))
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_export_kml() {
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
        let exporter = KmlExporter::new(&data);

        let path = Path::new("test_output.kml");
        exporter.export_kml(path).unwrap();

        assert!(path.exists());
        let content = fs::read_to_string(path).unwrap();
        assert!(content.contains("<name>Site 1</name>"));
        assert!(content.contains("<description>Test note</description>"));
        assert!(content.contains("<coordinates>10,20,100</coordinates>"));
        assert!(content.contains("<coordinates>-10,-20</coordinates>"));
        assert!(
            content.contains("http://maps.google.com/mapfiles/kml/shapes/placemark_circle.png")
        );

        fs::remove_file(path).unwrap();
    }
}
