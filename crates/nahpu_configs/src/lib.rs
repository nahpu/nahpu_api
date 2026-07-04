//! Configuration storage library.
//!
//! Provides data models and a Redb storage layer for handling project settings
//! and export templates natively in Rust.

pub mod db;
pub mod json_lines;
pub mod models;

pub use db::ConfigDb;
pub use models::{
    ConfigCombinedField, ConfigExportPreset, ConfigPresetEntry, DocumentLayoutBlock,
    DocumentLayoutPreset, DocumentLayoutStatus, TemplatePresetEntry, UserConfigsExport,
};
