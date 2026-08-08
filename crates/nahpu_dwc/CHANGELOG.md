# Changelog

## 0.4.0

- Audit schema v17 mappings against current Darwin Core and Dublin Core terms.
- Add verbatim coordinate, determiner, agent, taxonomy, herpetofauna, fossil,
  parasite, associated-data, and relationship mappings.
- Add row-level measurement unit sources for mammal, bird, and herpetofauna
  weights.
- Keep semantically unmatched NAHPU fields out of forced Darwin Core mappings.

## 0.3.0

- Map the canonical mammal, bird, and herpetofauna attribute namespaces.
- Preserve Darwin Core mappings for legacy measurement namespace aliases.

## 0.2.0

- Export Darwin Core Data Packages as tar.gz or ZIP archives.
- Make tar.gz the default Data Package container.
- Report ZIP as a DwC-DP compatibility option.

All notable changes to `nahpu_dwc` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2026-06-17

### Fixed
- Handled network fetch errors in `build.rs` for `docs.rs` builds.

## [0.1.0] - 2026-06-16

### Added
- Initial Darwin Core conversion utilities and auto-generated data structs.
