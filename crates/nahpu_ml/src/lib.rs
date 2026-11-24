//! Machine learning methods to analyze and visualize biodiversity data.
//!
//! The crate works on plain numeric tables, so callers decide how records are read. Missing
//! measurements are passed as `None`.
//!
//! ```
//! use nahpu_ml::pca::{Pca, PcaOptions};
//!
//! let rows = vec![
//!     vec![Some(1.0), Some(2.0)],
//!     vec![Some(2.0), Some(1.0)],
//!     vec![Some(3.0), Some(4.0)],
//!     vec![Some(4.0), None],
//! ];
//! let result = Pca::new(PcaOptions::default()).fit(&rows).unwrap();
//! assert_eq!(result.kept_rows, vec![0, 1, 2]);
//! assert_eq!(result.excluded_rows, 1);
//! ```

pub mod error;
pub mod pca;
pub mod stats;

pub use error::{MlError, Result};

/// Version of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
