#![warn(missing_docs)]
//! `nahpu_export` provides functionality for rendering database exports into formats like Markdown, Typst, and PDF.

/// Contains the core logic for structuring and formatting exports into Typst and Markdown documents.
pub mod document;
/// Contains the data models used for deserializing database records.
pub mod models;
/// Provides the integration with the Typst compiler to render `.typ` code into `.pdf` binaries.
pub mod typst_compiler;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub use document::DocumentExport;
pub use document::markdown_to_typst;
pub use models::*;
