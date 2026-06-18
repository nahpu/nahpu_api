use calamine::{DataType, Reader, Xlsx, open_workbook};
use polars::prelude::{CsvParseOptions, CsvReadOptions, DataFrame, JsonReader, SerReader};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Import a CSV into a DataFrame.
pub fn import_csv(path: &Path) -> Result<DataFrame, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    CsvReadOptions::default()
        .with_has_header(true)
        .into_reader_with_file_handle(file)
        .finish()
        .map_err(|e| e.to_string())
}

/// Import a TSV into a DataFrame.
pub fn import_tsv(path: &Path) -> Result<DataFrame, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    CsvReadOptions::default()
        .with_has_header(true)
        .with_parse_options(CsvParseOptions::default().with_separator(b'\t'))
        .into_reader_with_file_handle(file)
        .finish()
        .map_err(|e| e.to_string())
}

/// Import an Excel (.xlsx) file into a DataFrame using Calamine and Serde JSON.
pub fn import_excel(path: &Path, sheet_name: &str) -> Result<DataFrame, String> {
    let mut workbook: Xlsx<BufReader<File>> =
        open_workbook(path).map_err(|e: calamine::XlsxError| e.to_string())?;
    let range = workbook
        .worksheet_range(sheet_name)
        .map_err(|e| e.to_string())?;

    let mut rows = range.rows();
    let header_row = rows.next().ok_or("Empty sheet")?;

    let headers: Vec<String> = header_row.iter().map(|c| c.to_string()).collect();

    let mut json_array = Vec::new();

    for row in rows {
        let mut map = serde_json::Map::new();
        for (i, cell) in row.iter().enumerate() {
            if i >= headers.len() {
                break;
            }
            let key = headers[i].clone();
            let val = match cell {
                calamine::Data::Empty => serde_json::Value::Null,
                calamine::Data::String(s) => serde_json::Value::String(s.clone()),
                calamine::Data::Float(f) => {
                    if f.fract() == 0.0 {
                        serde_json::Value::Number(serde_json::Number::from(*f as i64))
                    } else {
                        serde_json::Number::from_f64(*f)
                            .map(serde_json::Value::Number)
                            .unwrap_or(serde_json::Value::Null)
                    }
                }
                calamine::Data::Int(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
                calamine::Data::Bool(b) => serde_json::Value::Bool(*b),
                calamine::Data::Error(e) => serde_json::Value::String(e.to_string()),
                calamine::Data::DateTime(_) => {
                    if let Some(dt) = cell.as_datetime() {
                        serde_json::Value::String(dt.to_string())
                    } else if let Some(s) = cell.as_string() {
                        serde_json::Value::String(s)
                    } else {
                        serde_json::Value::Null
                    }
                }
                calamine::Data::DateTimeIso(s) | calamine::Data::DurationIso(s) => {
                    serde_json::Value::String(s.clone())
                }
            };
            map.insert(key, val);
        }
        json_array.push(serde_json::Value::Object(map));
    }

    let json_bytes = serde_json::to_vec(&json_array).map_err(|e| e.to_string())?;
    let cursor = std::io::Cursor::new(json_bytes);
    JsonReader::new(cursor).finish().map_err(|e| e.to_string())
}
