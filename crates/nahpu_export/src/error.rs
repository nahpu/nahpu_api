//! Document export errors.

/// Error returned while parsing, rendering, or compiling a document.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    /// Serialized document records could not be decoded.
    #[error("failed to parse document JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
    /// Typst source could not be compiled.
    #[error("Typst compilation failed: {0}")]
    TypstCompilation(String),
    /// A compiled Typst document could not be encoded as PDF.
    #[error("PDF generation failed: {0}")]
    PdfGeneration(String),
}
