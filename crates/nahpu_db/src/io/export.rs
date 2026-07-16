use csv::WriterBuilder;
use rust_xlsxwriter::Workbook;
use serde_json::Value;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs::File;
use std::path::Path;

pub struct RecordExporter {
    data: Vec<Value>,
    columns: Vec<String>,
}

/// Positional tabular exporter for formats whose visible headers may repeat.
///
/// Darwin Core MeasurementOrFact groups intentionally repeat the same three
/// headers. JSON objects and map-based rows cannot retain those duplicates, so
/// this exporter writes rows by column position instead of by key.
pub struct TabularRecordExporter {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl TabularRecordExporter {
    pub fn new(headers: Vec<String>, rows: Vec<Vec<String>>) -> Result<Self, String> {
        for (index, row) in rows.iter().enumerate() {
            if row.len() != headers.len() {
                return Err(format!(
                    "Tabular row {} has {} cells but {} headers were supplied",
                    index,
                    row.len(),
                    headers.len()
                ));
            }
        }
        Ok(Self { headers, rows })
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

        for (column_index, header) in self.headers.iter().enumerate() {
            worksheet
                .write_string(0, column_index as u16, header)
                .map_err(|error| error.to_string())?;
        }
        for (row_index, row) in self.rows.iter().enumerate() {
            for (column_index, value) in row.iter().enumerate() {
                worksheet
                    .write_string(row_index as u32 + 1, column_index as u16, value)
                    .map_err(|error| error.to_string())?;
            }
        }
        workbook.save(path).map_err(|error| error.to_string())
    }

    fn write_delimited(&self, path: &Path, delimiter: u8) -> Result<(), String> {
        let mut writer = WriterBuilder::new()
            .delimiter(delimiter)
            .from_path(path)
            .map_err(|error| error.to_string())?;
        writer
            .write_record(&self.headers)
            .map_err(|error| error.to_string())?;
        for row in &self.rows {
            writer
                .write_record(row)
                .map_err(|error| error.to_string())?;
        }
        writer.flush().map_err(|error| error.to_string())
    }
}

impl RecordExporter {
    pub fn new(data: &[Value], columns: &[String], concatenate_multi_entries: bool) -> Self {
        if concatenate_multi_entries {
            Self {
                data: data.to_vec(),
                columns: columns.to_vec(),
            }
        } else {
            Self::expand_multi_entries(data, columns)
        }
    }

    fn expand_multi_entries(data: &[Value], columns: &[String]) -> Self {
        let mut max_splits: HashMap<String, usize> = HashMap::new();
        let mut expandable_cols = HashSet::new();
        let mut labeled_cols = HashSet::new();

        for row in data {
            if let Some(map) = row.as_object() {
                for col in columns {
                    if let Some(Value::String(s)) = map.get(col) {
                        if !s.contains('|') && !s.contains(": ") {
                            continue;
                        }
                        expandable_cols.insert(col.clone());
                        if s.contains(": ") {
                            labeled_cols.insert(col.clone());
                        }
                        let splits = s.split('|').count();
                        let max = max_splits.entry(col.clone()).or_insert(1);
                        if splits > *max {
                            *max = splits;
                        }
                    }
                }
            }
        }

        let mut new_data = Vec::new();
        let mut col_dynamic_keys: HashMap<String, BTreeSet<String>> = HashMap::new();

        for row in data {
            if let Some(map) = row.as_object() {
                let mut new_row = serde_json::Map::new();
                for col in columns {
                    if expandable_cols.contains(col) {
                        if let Some(Value::String(s)) = map.get(col) {
                            let table_prefix = if let Some(pos) = col.find("::") {
                                &col[..pos + 2]
                            } else {
                                ""
                            };
                            let parts: Vec<&str> = s.split('|').collect();
                            for (i, part) in parts.iter().enumerate() {
                                let idx = i + 1;
                                let is_labeled = labeled_cols.contains(col);

                                if !is_labeled {
                                    let col_name = format!("{}{}", col, idx);
                                    new_row.insert(col_name, Value::String(part.to_string()));
                                } else {
                                    for sub_item in part.split(';') {
                                        let sub_item = sub_item.trim();
                                        if let Some(pos) = sub_item.find(": ") {
                                            let key = sub_item[..pos].trim();
                                            let val = sub_item[pos + 2..].trim();
                                            let mut camel_key = key.to_string();
                                            if let Some(c) = camel_key.get_mut(0..1) {
                                                c.make_ascii_lowercase();
                                            }
                                            let dyn_col_name =
                                                format!("{}{}{}", table_prefix, camel_key, idx);
                                            col_dynamic_keys
                                                .entry(col.clone())
                                                .or_insert_with(BTreeSet::new)
                                                .insert(dyn_col_name.clone());
                                            new_row.insert(
                                                dyn_col_name,
                                                Value::String(val.to_string()),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        if let Some(v) = map.get(col) {
                            new_row.insert(col.clone(), v.clone());
                        }
                    }
                }
                new_data.push(Value::Object(new_row));
            }
        }

        let mut new_columns = Vec::new();
        for col in columns {
            if expandable_cols.contains(col) {
                if labeled_cols.contains(col) {
                    if let Some(keys) = col_dynamic_keys.get(col) {
                        let mut sorted_keys: Vec<String> = keys.iter().cloned().collect();
                        sorted_keys.sort_by(|a, b| {
                            let extract_num = |s: &str| -> u32 {
                                let num_str: String =
                                    s.chars().rev().take_while(|c| c.is_ascii_digit()).collect();
                                num_str
                                    .chars()
                                    .rev()
                                    .collect::<String>()
                                    .parse()
                                    .unwrap_or(0)
                            };
                            let num_a = extract_num(a);
                            let num_b = extract_num(b);
                            if num_a != num_b {
                                num_a.cmp(&num_b)
                            } else {
                                a.cmp(b)
                            }
                        });
                        new_columns.extend(sorted_keys);
                    }
                } else {
                    let max = max_splits.get(col).unwrap_or(&1);
                    for i in 1..=*max {
                        new_columns.push(format!("{}{}", col, i));
                    }
                }
            } else {
                new_columns.push(col.clone());
            }
        }

        Self {
            data: new_data,
            columns: new_columns,
        }
    }

    pub fn export_csv(&self, path: &Path) -> Result<(), String> {
        self.write_delimited(path, b',')
    }

    pub fn export_tsv(&self, path: &Path) -> Result<(), String> {
        self.write_delimited(path, b'\t')
    }

    pub fn export_json(&self, path: &Path) -> Result<(), String> {
        let file = File::create(path).map_err(|e| e.to_string())?;
        serde_json::to_writer_pretty(file, &self.data).map_err(|e| e.to_string())
    }

    pub fn export_excel(&self, path: &Path) -> Result<(), String> {
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();

        for (col_idx, name) in self.columns.iter().enumerate() {
            worksheet
                .write_string(0, col_idx as u16, name)
                .map_err(|e| e.to_string())?;
        }

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

    fn write_delimited(&self, path: &Path, delimiter: u8) -> Result<(), String> {
        let mut wtr = WriterBuilder::new()
            .delimiter(delimiter)
            .from_path(path)
            .map_err(|e| e.to_string())?;

        wtr.write_record(&self.columns).map_err(|e| e.to_string())?;

        for row in &self.data {
            let mut string_record = Vec::new();
            if let Some(map) = row.as_object() {
                for col in &self.columns {
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
                    .write_string(row_idx, col_idx, other.to_string())
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }
}
