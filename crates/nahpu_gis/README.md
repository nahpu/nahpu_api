# nahpu_gis

GIS exchange and coordinate conversion for NAHPU.

The crate imports point coordinates from GeoJSON, KML, GPX, and zipped WGS84
Shapefiles. It exports validated point collections as GeoJSON, TopoJSON, KML,
or zipped Shapefiles and normalizes vector layers to WGS84 GeoJSON.

```rust
use nahpu_gis::{CoordinateData, CoordinateExporter, CoordinateFormat};

let coordinates = [CoordinateData {
    name_id: "Site-1".to_owned(),
    decimal_latitude: Some(34.0522),
    decimal_longitude: Some(-118.2437),
    elevation_in_meter: Some(71.0),
    notes: Some("Sample notes".to_owned()),
}];

CoordinateExporter::new(&coordinates)?
    .export(CoordinateFormat::GeoJson, "export.geojson")?;
# Ok::<(), nahpu_gis::GisError>(())
```

Exports validate the entire collection before writing. Missing, non-finite, or
out-of-range values return a contextual `GisError` instead of being omitted.
