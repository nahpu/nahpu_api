# Contributing to Nahpu API

First off, thank you for considering contributing to Nahpu API! It's people like you that make open source software such a great thing.

## Getting Started

### Prerequisites

* [Rust](https://www.rust-lang.org/tools/install) (latest stable version)
* [Cargo](https://doc.rust-lang.org/cargo/) (comes with Rust)

### Setup

1. Fork the repository on GitHub.
2. Clone your forked repository locally:

    ```sh
    git clone https://github.com/your-username/nahpu_api.git
    cd nahpu_api
    ```

3. Build the project:

    ```sh
    cargo build
    ```

4. Run the tests:

    ```sh
    cargo test
    ```

## Project Structure

The project is organized as follows:

* `src/`: This directory contains all the source code for the Nahpu API client.
  * `lib.rs`: The main library crate.
  * `archive.rs`: Contains logic for handling archive files.
  * `types.rs`: Defines the data structures used in the project.
  * `databases/`: Contains modules for interacting with different database formats.
* `Cargo.toml`: The package manifest for Rust. It contains metadata and dependencies for the project.
* `README.md`: The main README file for the project.
* `CONTRIBUTING.md`: This file, which provides guidelines for contributing to the project.

## Submitting Changes

1. Create a new branch for your changes:

    ```sh
    git checkout -b my-feature-branch
    ```

2. Make your changes and commit them with a descriptive message.
3. Push your changes to your forked repository:

    ```sh
    git push origin my-feature-branch
    ```

4. Open a pull request on the main repository.

## Code of Conduct

This project and everyone participating in it is governed by the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please report unacceptable behavior to [herubiolog@gmail.com](mailto:herubiolog@gmail.com).
