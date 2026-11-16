use std::path::PathBuf;

/// Errors returned by `nahpu_dl`.
#[derive(Debug, thiserror::Error)]
pub enum DlError {
    /// Reading or writing a file failed.
    #[error("I/O error at {path}: {source}")]
    Io {
        /// The file or directory involved.
        path: PathBuf,
        /// The underlying error.
        source: std::io::Error,
    },
    /// The model manifest could not be parsed or written.
    #[error("invalid manifest {path}: {message}")]
    Manifest {
        /// The manifest path.
        path: PathBuf,
        /// What went wrong.
        message: String,
    },
    /// Saving or loading weights failed.
    #[error("weight store error at {path}: {message}")]
    Store {
        /// The weight file.
        path: PathBuf,
        /// What went wrong.
        message: String,
    },
    /// The input data is malformed.
    #[error("invalid data: {0}")]
    Data(String),
    /// A CSV file could not be read.
    #[error("CSV error in {path}: {source}")]
    Csv {
        /// The CSV file.
        path: PathBuf,
        /// The underlying error.
        source: csv::Error,
    },
    /// The model cannot handle this input or operation.
    #[error("unsupported: {0}")]
    Unsupported(String),
    /// The requested backend was not compiled in.
    #[error("backend `{0}` is not available; rebuild with the matching feature")]
    BackendUnavailable(String),
    /// A configuration value is invalid.
    #[error("invalid configuration: {0}")]
    Config(String),
}

impl DlError {
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    pub(crate) fn store(path: impl Into<PathBuf>, message: impl ToString) -> Self {
        Self::Store {
            path: path.into(),
            message: message.to_string(),
        }
    }
}
