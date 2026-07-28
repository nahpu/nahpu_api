//! Configuration storage library.
//!
//! Provides data models and a Redb storage layer for handling project settings
//! and export templates natively in Rust.

pub mod db;
pub mod models;

pub use db::ConfigDb;
/// Version of the compiled `nahpu_configs` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Current schema version for serialized user configuration exports.
pub const USER_CONFIG_SCHEMA_VERSION: u32 = 3;
pub use models::{
    ConfigCombinedField, ConfigExportPreset, ConfigPresetEntry, DocumentLayoutBlock,
    DocumentLayoutPreset, DocumentLayoutStatus, DocumentSortDirection,
    TemplatePresetDeletionResult, TemplatePresetEntry, TemplatePresetUsage, UserConfigSection,
    UserConfigsExport,
};
