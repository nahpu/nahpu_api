//! Read and write Keyhole Markup Language (KML).

use crate::types::CoordinateData;
use kml::{
    Kml, KmlDocument, KmlWriter,
    types::{Coord, Geometry, Icon, IconStyle, LineStyle, Placemark, Point, PolyStyle, Style},
};
use std::fs::File;
use std::path::Path;

pub(crate) struct KmlExporter<'a> {
    data: &'a [CoordinateData],
}

impl<'a> KmlExporter<'a> {
    pub(crate) fn new(data: &'a [CoordinateData]) -> Self {
        Self { data }
    }

    pub(crate) fn export(&self, path: &Path) -> Result<(), String> {
        let mut elements = vec![];

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

        elements.push(Kml::Style(style));

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

                elements.push(Kml::Placemark(placemark));
            }
        }

        let doc_element = Kml::Document {
            attrs: std::collections::HashMap::new(),
            elements,
        };

        let mut kml_doc = KmlDocument::default();
        kml_doc.elements.push(doc_element);

        let file = File::create(path).map_err(|e| e.to_string())?;
        let mut writer = KmlWriter::from_writer(file);
        writer
            .write(&Kml::KmlDocument(kml_doc))
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
        exporter.export(path).expect("KML export should succeed");

        assert!(path.exists());
        let content = fs::read_to_string(path).expect("KML should be readable");
        assert!(content.contains("<name>Site 1</name>"));
        assert!(content.contains("<description>Test note</description>"));
        assert!(content.contains("<coordinates>10,20,100</coordinates>"));
        assert!(content.contains("<coordinates>-10,-20</coordinates>"));
        assert!(
            content.contains("http://maps.google.com/mapfiles/kml/shapes/placemark_circle.png")
        );

        fs::remove_file(path).expect("test output should be removable");
    }
}
