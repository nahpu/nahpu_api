//! # NAHPU Archive
//!
//! `nahpu_archive` provides compression and extraction utilities for NAHPU project archives.
//!
//! It implements lightweight wrappers around ZIP, gzip, and tar.gz formats.

pub mod gzip;
pub mod tar_gzip;
pub mod zip;

/// Version of the compiled `nahpu_archive` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Compatibility module for existing NAHPU callers. New code should import `zip`.
pub mod archive {
    pub use crate::zip::{ZipArchive, ZipExtractor};
}
