#![warn(missing_docs)]
//! Document rendering for NAHPU Markdown, Typst, and PDF exports.

/// Structures and formats exports as Typst and Markdown documents.
pub mod document;
mod error;
/// Contains the data models used for deserializing database records.
pub mod models;
/// Provides the integration with the Typst compiler to render `.typ` code into `.pdf` binaries.
mod typst_compiler;

/// The current version of the `nahpu_export` crate, derived from the Cargo package version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub use document::DocumentRenderer;
pub use document::markdown_to_typst;
pub use error::ExportError;
pub use models::*;
pub use typst_compiler::TypstCompiler;
