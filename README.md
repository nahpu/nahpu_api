# NAHPU API

A cross-platform Application Programming Interface (API) for the NAHPU specimen cataloging app (work in progress). It handles heavy computation for the app and provides a unified interface for the planned NAHPU suites of command line tools and Python and R libraries. The API is designed based on the SEGUL API [segul.app](https://segul.app).

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
