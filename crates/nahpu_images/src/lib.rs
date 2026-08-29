#![warn(missing_docs)]
//! Image conversion and resizing support for NAHPU.
//!
//! The crate processes static JPEG, PNG, and WebP images. It exposes an in-memory API for
//! reusable Rust callers and a path-based API that is suitable for a future Flutter bridge.

mod error;
mod metadata;
mod processor;
mod types;

pub use error::ImageError;
pub use processor::ImageProcessor;
pub use types::{
    ImageFileFormat, ImageInfo, ImageOptions, ProcessedImage, ResizeMode, ResizeOptions, RgbColor,
};

/// Version of the compiled `nahpu_images` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
