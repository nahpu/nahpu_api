use calamine::{DataType, Reader, Xlsx, open_workbook};
use serde_json::Value;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub struct RecordImporter<'a> {
    path: &'a Path,
}

impl<'a> RecordImporter<'a> {
    pub fn new(path: &'a Path) -> Self {
        Self { path }
    }

    /// Import a CSV into a JSON array.
    pub fn import_csv(&self) -> Result<Vec<Value>, String> {
        self.import_delimited(b',')
    }

    /// Import a TSV into a JSON array.
    pub fn import_tsv(&self) -> Result<Vec<Value>, String> {
        self.import_delimited(b'\t')
    }

    /// Import an Excel sheet into a JSON array.
    pub fn import_excel(&self, sheet_name: &str) -> Result<Vec<Value>, String> {
        let mut workbook: Xlsx<BufReader<File>> =
            open_workbook(self.path).map_err(|e: calamine::XlsxError| e.to_string())?;
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
                let val = self.parse_excel_cell(cell);
                map.insert(key, val);
            }
            json_array.push(Value::Object(map));
        }

        Ok(json_array)
    }

    fn import_delimited(&self, delimiter: u8) -> Result<Vec<Value>, String> {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .delimiter(delimiter)
            .from_path(self.path)
            .map_err(|e| e.to_string())?;

        let mut json_array = Vec::new();
        let headers = rdr.headers().map_err(|e| e.to_string())?.clone();

        for result in rdr.records() {
            let record = result.map_err(|e| e.to_string())?;
            let mut map = serde_json::Map::new();
            for (i, val) in record.iter().enumerate() {
                if i >= headers.len() {
                    break;
                }
                let key = headers[i].to_string();
                let value = self.parse_delimited_cell(val);
                map.insert(key, value);
            }
            json_array.push(Value::Object(map));
        }
        Ok(json_array)
    }

    fn parse_delimited_cell(&self, val: &str) -> Value {
        if val.is_empty() {
            Value::Null
        } else if let Ok(n) = val.parse::<i64>() {
            Value::Number(n.into())
        } else if let Ok(f) = val.parse::<f64>() {
            if let Some(num) = serde_json::Number::from_f64(f) {
                Value::Number(num)
            } else {
                Value::String(val.to_string())
            }
        } else if let Ok(b) = val.parse::<bool>() {
            Value::Bool(b)
        } else {
            Value::String(val.to_string())
        }
    }

    fn parse_excel_cell(&self, cell: &calamine::Data) -> Value {
        match cell {
            calamine::Data::Empty => Value::Null,
            calamine::Data::String(s) => Value::String(s.clone()),
            calamine::Data::Float(f) => {
                if f.fract() == 0.0 {
                    Value::Number(serde_json::Number::from(*f as i64))
                } else {
                    serde_json::Number::from_f64(*f)
                        .map(Value::Number)
                        .unwrap_or(Value::Null)
                }
            }
            calamine::Data::Int(i) => Value::Number(serde_json::Number::from(*i)),
            calamine::Data::Bool(b) => Value::Bool(*b),
            calamine::Data::Error(e) => Value::String(e.to_string()),
            calamine::Data::DateTime(_) => {
                if let Some(dt) = cell.as_datetime() {
                    Value::String(dt.to_string())
                } else if let Some(s) = cell.as_string() {
                    Value::String(s)
                } else {
                    Value::Null
                }
            }
            calamine::Data::DateTimeIso(s) | calamine::Data::DurationIso(s) => {
                Value::String(s.clone())
            }
        }
    }
}
