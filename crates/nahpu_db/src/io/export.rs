use polars::prelude::{AnyValue, CsvWriter, DataFrame, SerWriter};
use rust_xlsxwriter::Workbook;
use std::fs::File;
use std::path::Path;

/// Export a DataFrame to CSV.
pub fn export_csv(df: &mut DataFrame, path: &Path) -> Result<(), String> {
    let mut file = File::create(path).map_err(|e| e.to_string())?;
    CsvWriter::new(&mut file)
        .include_header(true)
        .finish(df)
        .map_err(|e| e.to_string())
}

/// Export a DataFrame to TSV.
pub fn export_tsv(df: &mut DataFrame, path: &Path) -> Result<(), String> {
    let mut file = File::create(path).map_err(|e| e.to_string())?;
    CsvWriter::new(&mut file)
        .include_header(true)
        .with_separator(b'\t')
        .finish(df)
        .map_err(|e| e.to_string())
}

/// Export a DataFrame to Excel (.xlsx).
pub fn export_excel(df: &mut DataFrame, path: &Path) -> Result<(), String> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    // Write headers
    let col_names = df.get_column_names();
    for (col_idx, name) in col_names.iter().enumerate() {
        worksheet
            .write_string(0, col_idx as u16, name.as_str())
            .map_err(|e| e.to_string())?;
    }

    // Write rows.
    for (col_idx, name) in col_names.iter().enumerate() {
        let series = df.column(*name).map_err(|e| e.to_string())?;
        for row_idx in 0..series.len() {
            let val = series.get(row_idx).map_err(|e| e.to_string())?;
            match val {
                AnyValue::Null => {}
                AnyValue::Int32(v) => {
                    worksheet
                        .write_number(row_idx as u32 + 1, col_idx as u16, v)
                        .map_err(|e| e.to_string())?;
                }
                AnyValue::Int64(v) => {
                    worksheet
                        .write_number(row_idx as u32 + 1, col_idx as u16, v as f64)
                        .map_err(|e| e.to_string())?;
                }
                AnyValue::Float32(v) => {
                    worksheet
                        .write_number(row_idx as u32 + 1, col_idx as u16, v)
                        .map_err(|e| e.to_string())?;
                }
                AnyValue::Float64(v) => {
                    worksheet
                        .write_number(row_idx as u32 + 1, col_idx as u16, v)
                        .map_err(|e| e.to_string())?;
                }
                AnyValue::String(v) => {
                    worksheet
                        .write_string(row_idx as u32 + 1, col_idx as u16, v)
                        .map_err(|e| e.to_string())?;
                }
                AnyValue::Boolean(v) => {
                    worksheet
                        .write_boolean(row_idx as u32 + 1, col_idx as u16, v)
                        .map_err(|e| e.to_string())?;
                }
                _ => {
                    // Fallback to string representation for other types
                    let s = format!("{}", val);
                    worksheet
                        .write_string(row_idx as u32 + 1, col_idx as u16, &s)
                        .map_err(|e| e.to_string())?;
                }
            }
        }
    }

    workbook.save(path).map_err(|e| e.to_string())?;
    Ok(())
}
