# nahpu_gis

A utility crate for spatial analysis and GIS data processing, mainly used for [Nahpu](https://www.nahpu.app/), but can be used as a standalone crate.

## Supported Formats

- **GeoJSON**: Standard `.geojson` point collections.
- **TopoJSON**: Lightweight `.topojson` topologies.
- **KML**: Google Earth compatible `.kml` documents.
- **Shapefile**: Bundled ESRI Shapefiles (`.shp`, `.shx`, `.dbf`, `.prj`) zipped together automatically.

## Example Usage

### Exporting to GeoJSON

```rust
use nahpu_gis::types::CoordinateData;
use nahpu_gis::io::geojson::GeoJsonExporter;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let coords = vec![
        CoordinateData {
            name_id: "Site-1".to_string(),
            decimal_latitude: Some(34.0522),
            decimal_longitude: Some(-118.2437),
            elevation_in_meter: Some(71.0),
            notes: Some("Sample notes".to_string()),
        }
    ];

    let exporter = GeoJsonExporter::new(&coords);
    exporter.export_geojson(Path::new("export.geojson"))?;

    Ok(())
}
```

### Exporting to Shapefile

```rust
use nahpu_gis::types::CoordinateData;
use nahpu_gis::io::shp::ShapefileExporter;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let coords = vec![
        CoordinateData {
            name_id: "Site-1".to_string(),
            decimal_latitude: Some(34.0522),
            decimal_longitude: Some(-118.2437),
            elevation_in_meter: Some(71.0),
            notes: Some("Sample notes".to_string()),
        }
    ];

    let exporter = ShapefileExporter::new(&coords);
    
    // Automatically generates .shp, .shx, .dbf, and .prj bundled inside a .zip
    exporter.export_shp(Path::new("export.zip"))?;

    Ok(())
}
```
