# NAHPU DB

A utility crate for the NAHPU project that handles the database schema and models.

## Features

- Fetches the latest SQLite schema (Drift file) from the main NAHPU repository during the build process.
- Auto-generates Rust `struct` definitions corresponding to the SQLite tables using `sqlparser`.
- Provides `serde`-compatible models with camelCase JSON representation for seamless integration with other tools (e.g. Darwin Core conversion).

## Usage

This crate is primarily used internally by other NAHPU utility crates, such as `nahpu_dwc`, to ensure type safety and schema alignment.
