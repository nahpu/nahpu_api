# nahpu_export

`nahpu_export` is a Rust crate that handles formatting and generating document exports for the NAHPU specimen cataloging app.

It allows NAHPU project data (such as narratives, sites, collecting events, and specimen records) to be seamlessly exported into standard presentation formats. The crate converts serialized JSON records from the database into raw document codes, with native support for escaping structural syntax and parsing rich data into standard typologies.

## Features

- **Markdown (`.md`) Generation**: Safely escapes syntax and creates structured markdown documents.
- **Typst (`.typ`) Generation**: Generates programmatic document codes for the modern Typst typesetting engine.
- **PDF Compilation**: Natively embeds a virtual `SimpleWorld` environment to directly compile generated `.typ` files into `.pdf` byte buffers (integrating custom asset fonts without requiring host-system installation).

## Architecture

This crate heavily depends on `nahpu_db` for mapping database records onto generated struct bindings and leverages `typst` libraries for heavy lifting on the PDF compilation steps.

- **`models.rs`**: Serializes and binds `nahpu_db` records.
- **`document.rs`**: Core generation engine housing `to_markdown`, `to_typst`, and `to_pdf` logic alongside specialized escaping systems.
- **`typst_compiler.rs`**: Hosts the `SimpleWorld` implementation required by Typst to bundle `.ttf` fonts and run the compiler against dynamically generated strings.
