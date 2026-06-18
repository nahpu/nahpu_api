use csv::WriterBuilder;
use rust_xlsxwriter::Workbook;
use serde_json::Value;
use std::path::Path;

pub struct RecordExporter<'a> {
    data: &'a [Value],
    columns: &'a [String],
}

impl<'a> RecordExporter<'a> {
    pub fn new(data: &'a [Value], columns: &'a [String]) -> Self {
        Self { data, columns }
    }

    pub fn export_csv(&self, path: &Path) -> Result<(), String> {
        self.write_delimited(path, b',')
    }

    pub fn export_tsv(&self, path: &Path) -> Result<(), String> {
        self.write_delimited(path, b'\t')
    }

    pub fn export_excel(&self, path: &Path) -> Result<(), String> {
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();

        // Write headers
        for (col_idx, name) in self.columns.iter().enumerate() {
            worksheet
                .write_string(0, col_idx as u16, name)
                .map_err(|e| e.to_string())?;
        }

        // Write rows
        for (row_idx, row) in self.data.iter().enumerate() {
            if let Some(map) = row.as_object() {
                for (col_idx, col_name) in self.columns.iter().enumerate() {
                    let cell_value = map.get(col_name);
                    self.write_excel_cell(
                        worksheet,
                        row_idx as u32 + 1,
                        col_idx as u16,
                        cell_value,
                    )?;
                }
            }
        }
        workbook.save(path).map_err(|e| e.to_string())?;
        Ok(())
    }

    // --- Private Helper Methods ---

    fn write_delimited(&self, path: &Path, delimiter: u8) -> Result<(), String> {
        let mut wtr = WriterBuilder::new()
            .delimiter(delimiter)
            .from_path(path)
            .map_err(|e| e.to_string())?;

        // Write headers
        wtr.write_record(self.columns).map_err(|e| e.to_string())?;

        // Write rows
        for row in self.data {
            let mut string_record = Vec::new();
            if let Some(map) = row.as_object() {
                for col in self.columns {
                    let cell_value = self.json_value_to_string(map.get(col));
                    string_record.push(cell_value);
                }
            }
            wtr.write_record(&string_record)
                .map_err(|e| e.to_string())?;
        }
        wtr.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn json_value_to_string(&self, val: Option<&Value>) -> String {
        match val {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Number(n)) => n.to_string(),
            Some(Value::Bool(b)) => b.to_string(),
            Some(Value::Null) | None => String::new(),
            Some(other) => other.to_string(),
        }
    }

    fn write_excel_cell(
        &self,
        worksheet: &mut rust_xlsxwriter::Worksheet,
        row_idx: u32,
        col_idx: u16,
        val: Option<&Value>,
    ) -> Result<(), String> {
        match val {
            Some(Value::String(s)) => {
                worksheet
                    .write_string(row_idx, col_idx, s)
                    .map_err(|e| e.to_string())?;
            }
            Some(Value::Number(n)) => {
                if let Some(f) = n.as_f64() {
                    worksheet
                        .write_number(row_idx, col_idx, f)
                        .map_err(|e| e.to_string())?;
                }
            }
            Some(Value::Bool(b)) => {
                worksheet
                    .write_boolean(row_idx, col_idx, *b)
                    .map_err(|e| e.to_string())?;
            }
            Some(Value::Null) | None => {}
            Some(other) => {
                worksheet
                    .write_string(row_idx, col_idx, &other.to_string())
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }
}
