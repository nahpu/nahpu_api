//! Error types for GIS operations.

use std::{io, path::PathBuf};

/// Error returned by coordinate conversion and GIS exchange operations.
#[derive(Debug, thiserror::Error)]
pub enum GisError {
    /// A coordinate or coordinate collection failed validation.
    #[error("invalid coordinate: {0}")]
    InvalidCoordinate(String),
    /// An input or output format is not supported.
    #[error("unsupported GIS format: {0}")]
    UnsupportedFormat(String),
    /// A file operation failed.
    #[error("failed to access '{}': {source}", path.display())]
    Io {
        /// Path involved in the failed operation.
        path: PathBuf,
        /// Underlying filesystem error.
        #[source]
        source: io::Error,
    },
    /// A data format could not be parsed or serialized.
    #[error("invalid GIS data: {0}")]
    Data(String),
    /// A GIS import, conversion, or export operation failed.
    #[error("GIS operation failed: {0}")]
    Operation(String),
}
