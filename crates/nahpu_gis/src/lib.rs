#![warn(missing_docs)]
//! Coordinate conversion and GIS exchange support for NAHPU.

pub mod conversion;
mod error;
mod exchange;
mod io;
mod types;

pub use error::GisError;
pub use exchange::{
    CoordinateExporter, CoordinateFormat, CoordinateImporter, VectorLayerConverter,
};
pub use io::coordinates::CoordinateImportResult;
pub use io::layers::ConvertedVectorLayer;
pub use types::{CoordinateData, GeographicBounds};

/// Version of the compiled `nahpu_gis` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
