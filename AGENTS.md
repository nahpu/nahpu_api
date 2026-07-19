# Repository Guidelines

## Project Structure & Module Organization

This Rust workspace powers the NAHPU API. The root `Cargo.toml` lists member crates under `crates/`. Each crate keeps implementation in `src/`, usually with `lib.rs` and focused modules such as `io/`, `types/`, `export/`, or `dwc/`.

- `crates/nahpu_archive`: zip archive utilities.
- `crates/nahpu_db`: database schema and SQLite types.
- `crates/nahpu_dwc`: Darwin Core exports.
- `crates/nahpu_export`: document rendering.
- `crates/nahpu_gis`: GIS import and conversion.
- `crates/nahpu_configs`: user configuration and preset storage.
- `assets/`: repository images and branding assets.

Integration tests live in crate-local `tests/` directories, for example `crates/nahpu_archive/tests/`.

## Build, Test, and Development Commands

- `cargo build --workspace`: build every workspace crate.
- `cargo test --workspace`: run all unit and integration tests.
- `cargo build --verbose --workspace`: match CI builds.
- `cargo test --verbose --workspace`: match CI tests.
- `cargo test -p nahpu_dwc`: run tests for one crate while iterating.
- `cargo fmt --all`: format the workspace before committing.
- `cargo clippy --workspace --all-targets`: check for common Rust issues.

## Rust Style & Best Practices

Use `rustfmt`, follow the Rust API Guidelines, and keep Rust code, comments, and docstrings to 100 characters per line. Wrap long signatures cleanly. Organize related state and behavior into structs with impl blocks. Prefer methods over free functions when the behavior naturally belongs to a type, but use free functions when no type is a clear owner. Do not define functions inside other functions; extract helpers into private methods or module-level private functions. Expose only the methods required by the public API. Within each impl block, place public methods before private methods.

Use `?`, pattern matching, and references instead of unnecessary clones. Avoid `.unwrap()` unless justified. Use `snake_case` for functions and variables, `PascalCase` for types and traits, and `SCREAMING_SNAKE_CASE` for constants.

## Testing Guidelines

Use Rust’s built-in test framework. Place integration tests under `crates/<crate>/tests/` and unit tests near the code they validate. Name tests after behavior, for example `test_archive_and_extract`. Add regressions for conversion, serialization, archives, and compatibility fixes.

## Security & Configuration Tips

Do not commit local databases, generated archives, credentials, or machine-specific paths. Keep sample data minimal.

## Agent-Specific Instructions

Preserve user changes and avoid unrelated refactors. Always write the product name as `NAHPU`. Do not create commits, branches, pushes, or pull requests; leave Git operations under user control.
