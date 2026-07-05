# NAHPU API <img src="assets/nahpu-api.svg" alt="nahpu logo" align="right" width="150"/>

![nahpu-test](https://github.com/nahpu/nahpu_api/actions/workflows/test.yml/badge.svg)

A cross-platform Application Programming Interface (API) for the NAHPU specimen cataloging app (work in progress). It handles heavy computation for the app and provides a unified interface for the planned NAHPU suites of command line tools and Python and R libraries.

## Crates

This workspace consists of several modular crates:
- **`nahpu_archive`**: A utility crate for archiving and extracting Nahpu project data using zip compression.
- **`nahpu_db`**: Handles database schema and models, auto-generating Rust structs from the NAHPU SQLite Drift schema.
- **`nahpu_dwc`**: Maps and converts NAHPU project data into Darwin Core (DwC) compliant JSON and XML formats.
- **`nahpu_export`**: Renders database records into formatted documents such as Markdown, Typst, and PDF.
- **`nahpu_gis`**: Handles GIS data processing and conversion.
- **`nahpu_configs`**: Manages reproducibility-related user configuration, export presets, template presets, and document layouts in `redb`.

## Development Status

All crates are experimental. Please expect breaking changes and API instability.

## Installation

### For Rust developers

```bash
cargo install nahpu_api
```

## Development Guidelines

### Crate Naming Style
When creating new crates in this workspace, please use `snake_case` for crate names and their corresponding directories (e.g., `nahpu_dwc`).

## Contributing

Contributions are welcome! Please see the [CONTRIBUTING.md](CONTRIBUTING.md) file for details on how to contribute to this project.
