//! NAHPU Data Package creation and validation.

mod package;

pub use package::{
    ArchiveFormat, ControlledVocabulary, EnumMapping, PackageColumn, PackageFile,
    PackageForeignKey, PackageManifest, PackageRequest, PackageTable, plan_package_json,
    validate_package_json, write_package_json,
};

/// Version of the compiled `nahpu_dp` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Version of the NAHPU Data Package file-format contract.
pub const FORMAT_VERSION: &str = "1.0";
