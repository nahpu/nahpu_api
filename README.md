# NAHPU API

A cross-platform Application Programming Interface (API) for the NAHPU specimen cataloging app (work in progress). It handles heavy computation for the app and provides a unified interface for the planned NAHPU suites of command line tools and Python and R libraries. The API is designed based on the SEGUL API [segul.app](https://segul.app).

## Installation

### For Rust developers

```bash
cargo install nahpu_api
```

## Contributing

Contributions are welcome! Please see the [CONTRIBUTING.md](CONTRIBUTING.md) file for details on how to contribute to this project.

## Technologies

This project is built with Rust and uses the following main dependencies:

*   [flate2](https://crates.io/crates/flate2)
*   [zip](https://crates.io/crates/zip)