# Changelog

## 0.3.2

- Treat feature-specific NAHPU Data Package tables as optional.
- Accept both canonical `environment` and legacy `weather` table names.
- Validate table-resource consistency and safe table identifiers.
- Bump the NAHPU Data Package format contract to 3.2.

## 0.3.1

- Include arthropod attributes as a required NAHPU Data Package table.
- Bump the NAHPU Data Package format contract to 3.1.

## 0.3.0

- Replace the SQLite snapshot with the versioned `nahpu-project.json` project
  transfer payload.
- Scope tabular resources and packaged media to the same active project.
- Bump the NAHPU Data Package format contract to 3.0.

## 0.2.0

- Use the canonical specimen attribute resource names.
- Bump the NAHPU Data Package format contract to 2.0.

## 0.1.0

- Add NAHPU Data Package planning, validation, ZIP export, and tar.gz export.
- Add Frictionless `datapackage.json` and reproducibility metadata in `nahpu.toml`.
- Add SQLite enum-index mappings and site, event, and specimen controlled-vocabulary CSVs.
