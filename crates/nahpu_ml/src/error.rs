/// Errors returned by `nahpu_ml`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MlError {
    /// The table has too few variables to analyze.
    #[error("{found} variables given; at least {required} are needed")]
    TooFewVariables {
        /// Number of variables in the table.
        found: usize,
        /// Fewest variables the analysis accepts.
        required: usize,
    },
    /// Too few rows have a value for every variable.
    #[error("only {complete} complete rows; at least {required} are needed")]
    TooFewRows {
        /// Number of rows with a usable value in every variable.
        complete: usize,
        /// Fewest complete rows the analysis accepts.
        required: usize,
    },
    /// A row has a different number of values than the first row.
    #[error("row {row} has {found} values; expected {expected}")]
    RaggedRow {
        /// Index of the offending row.
        row: usize,
        /// Number of values in that row.
        found: usize,
        /// Number of values in the first row.
        expected: usize,
    },
    /// A variable does not vary across the complete rows.
    #[error("variable {index} has the same value in every complete row")]
    ConstantVariable {
        /// Index of the variable.
        index: usize,
    },
    /// An analysis option is invalid.
    #[error("invalid configuration: {0}")]
    Config(String),
}

/// Result type used across `nahpu_ml`.
pub type Result<T> = std::result::Result<T, MlError>;
