//! Errors returned by image processing operations.

use std::{io, path::PathBuf};

use crate::ImageFileFormat;

/// Error returned by image conversion or resizing operations.
#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    /// The requested processing options are invalid.
    #[error("invalid image options: {0}")]
    InvalidOptions(String),
    /// The input is not one of the supported image formats.
    #[error("unsupported image format; expected JPEG, PNG, or WebP")]
    UnsupportedFormat,
    /// Animated inputs are outside the static image processing contract.
    #[error("animated {0} images are not supported")]
    UnsupportedAnimation(ImageFileFormat),
    /// The image could not be decoded.
    #[error("failed to decode image: {0}")]
    Decode(#[source] image::ImageError),
    /// The processed image could not be encoded.
    #[error("failed to encode image: {0}")]
    Encode(#[source] image::ImageError),
    /// EXIF metadata could not be read, normalized, or written.
    #[error("failed to preserve EXIF metadata: {0}")]
    Metadata(#[source] io::Error),
    /// A filesystem operation failed.
    #[error("failed to {operation} '{}': {source}", path.display())]
    Io {
        /// Description of the attempted filesystem operation.
        operation: &'static str,
        /// Path involved in the failed operation.
        path: PathBuf,
        /// Underlying filesystem error.
        #[source]
        source: io::Error,
    },
    /// The destination already exists and overwrite was not enabled.
    #[error("output file already exists: '{}'", .0.display())]
    DestinationExists(PathBuf),
    /// The destination extension does not match the requested output format.
    #[error("output path '{}' must use a {expected} extension", path.display())]
    OutputExtension {
        /// Destination path supplied by the caller.
        path: PathBuf,
        /// Human-readable extensions accepted for the requested format.
        expected: &'static str,
    },
}
