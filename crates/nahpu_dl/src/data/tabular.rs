use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::data::{LabeledDataset, Sample, SampleInput, Target};
use crate::manifest::Task;
use crate::{DlError, Result};

/// Cell values read as missing.
const MISSING_VALUES: [&str; 5] = ["", "na", "nan", "null", "none"];

/// Reads tabular samples from a CSV file with a header row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabularCsv {
    /// Path to the CSV file.
    pub path: PathBuf,
    /// Column holding sample identifiers. Rows are numbered when unset.
    pub id_column: Option<String>,
    /// Column holding the target, for labeled datasets.
    pub target_column: Option<String>,
    /// Feature columns, in order. All other columns are used when unset.
    pub feature_columns: Option<Vec<String>>,
    /// Field delimiter.
    pub delimiter: u8,
}

impl TabularCsv {
    /// Creates a reader for a comma-separated file.
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            id_column: None,
            target_column: None,
            feature_columns: None,
            delimiter: b',',
        }
    }

    /// Sets the identifier column.
    pub fn with_id_column(mut self, column: impl Into<String>) -> Self {
        self.id_column = Some(column.into());
        self
    }

    /// Sets the target column.
    pub fn with_target_column(mut self, column: impl Into<String>) -> Self {
        self.target_column = Some(column.into());
        self
    }

    /// Sets the feature columns.
    pub fn with_feature_columns(mut self, columns: Vec<String>) -> Self {
        self.feature_columns = Some(columns);
        self
    }

    /// Sets the field delimiter, for example `b'\t'`.
    pub fn with_delimiter(mut self, delimiter: u8) -> Self {
        self.delimiter = delimiter;
        self
    }

    /// Reads a labeled dataset. Rows with a missing target are skipped and counted.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read, the target column is unset or absent,
    /// a feature value is not numeric, or a regression target is not numeric.
    pub fn read_labeled(&self, task: Task) -> Result<LabeledDataset> {
        let target_column = self.target_column.as_deref().ok_or_else(|| {
            DlError::Config("a target column is required for labeled data".to_string())
        })?;
        let table = self.read_table()?;
        let target_index = table.column_index(target_column)?;
        let feature_names = self.resolve_features(&table.headers)?;
        let mut dataset = LabeledDataset {
            feature_names: feature_names.clone(),
            ..Default::default()
        };
        for (row_number, record) in table.records.iter().enumerate() {
            let raw_target = record.get(target_index).unwrap_or_default().trim();
            if is_missing(raw_target) {
                dataset.skipped_rows += 1;
                continue;
            }
            let target = parse_target(raw_target, task, row_number)?;
            dataset
                .samples
                .push(table.sample(self, record, &feature_names, row_number)?);
            dataset.targets.push(target);
        }
        Ok(dataset)
    }

    /// Reads unlabeled samples whose features follow `feature_names`, the model's order.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read, a feature column is absent, or a value
    /// is not numeric.
    pub fn read_samples(&self, feature_names: &[String]) -> Result<Vec<Sample>> {
        let table = self.read_table()?;
        table
            .records
            .iter()
            .enumerate()
            .map(|(row_number, record)| table.sample(self, record, feature_names, row_number))
            .collect()
    }

    fn read_table(&self) -> Result<Table> {
        let csv_error = |source| DlError::Csv {
            path: self.path.clone(),
            source,
        };
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(self.delimiter)
            .trim(csv::Trim::All)
            .from_path(&self.path)
            .map_err(csv_error)?;
        let headers: Vec<String> = reader
            .headers()
            .map_err(csv_error)?
            .iter()
            .map(str::to_string)
            .collect();
        let records = reader
            .records()
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(csv_error)?;
        Ok(Table::new(headers, records))
    }

    fn resolve_features(&self, headers: &[String]) -> Result<Vec<String>> {
        if let Some(columns) = &self.feature_columns {
            return Ok(columns.clone());
        }
        let excluded = [self.id_column.as_deref(), self.target_column.as_deref()];
        let features: Vec<String> = headers
            .iter()
            .filter(|header| !excluded.contains(&Some(header.as_str())))
            .cloned()
            .collect();
        if features.is_empty() {
            return Err(DlError::Data("no feature columns found".to_string()));
        }
        Ok(features)
    }
}

struct Table {
    headers: Vec<String>,
    index: HashMap<String, usize>,
    records: Vec<csv::StringRecord>,
}

impl Table {
    fn new(headers: Vec<String>, records: Vec<csv::StringRecord>) -> Self {
        let index = headers
            .iter()
            .enumerate()
            .map(|(i, header)| (header.clone(), i))
            .collect();
        Self {
            headers,
            index,
            records,
        }
    }

    fn column_index(&self, column: &str) -> Result<usize> {
        self.index
            .get(column)
            .copied()
            .ok_or_else(|| DlError::Data(format!("column `{column}` not found")))
    }

    fn sample(
        &self,
        reader: &TabularCsv,
        record: &csv::StringRecord,
        feature_names: &[String],
        row_number: usize,
    ) -> Result<Sample> {
        let id = match &reader.id_column {
            Some(column) => record
                .get(self.column_index(column)?)
                .unwrap_or_default()
                .to_string(),
            // Header is line 1, so data rows start at line 2.
            None => format!("row_{}", row_number + 2),
        };
        let features = feature_names
            .iter()
            .map(|name| {
                let raw = record.get(self.column_index(name)?).unwrap_or_default();
                parse_feature(raw, name, row_number)
            })
            .collect::<Result<Vec<f32>>>()?;
        Ok(Sample::new(id, SampleInput::Tabular(features)))
    }
}

fn is_missing(value: &str) -> bool {
    MISSING_VALUES.contains(&value.to_ascii_lowercase().as_str())
}

fn parse_feature(raw: &str, column: &str, row_number: usize) -> Result<f32> {
    if is_missing(raw) {
        return Ok(f32::NAN);
    }
    raw.parse().map_err(|_| {
        DlError::Data(format!(
            "line {}: `{raw}` in column `{column}` is not a number",
            row_number + 2
        ))
    })
}

fn parse_target(raw: &str, task: Task, row_number: usize) -> Result<Target> {
    match task {
        Task::Classification => Ok(Target::Class(raw.to_string())),
        Task::Regression => raw.parse().map(Target::Value).map_err(|_| {
            DlError::Data(format!(
                "line {}: regression target `{raw}` is not a number",
                row_number + 2
            ))
        }),
        Task::Embedding => Err(DlError::Config(
            "embedding models are not trained from labeled data".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_csv(text: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.csv");
        std::fs::write(&path, text).unwrap();
        (dir, path)
    }

    #[test]
    fn test_read_labeled_skips_missing_targets() {
        let (_dir, path) = write_csv("id,length,mass,species\nA,1.5,NA,x\nB,2,3,\nC,4,5,y\n");
        let dataset = TabularCsv::new(&path)
            .with_id_column("id")
            .with_target_column("species")
            .read_labeled(Task::Classification)
            .unwrap();
        assert_eq!(dataset.feature_names, vec!["length", "mass"]);
        assert_eq!(dataset.len(), 2);
        assert_eq!(dataset.skipped_rows, 1);
        assert_eq!(dataset.samples[0].id, "A");
        let SampleInput::Tabular(row) = &dataset.samples[0].input else {
            panic!("expected tabular input");
        };
        assert!(row[1].is_nan());
    }

    #[test]
    fn test_read_samples_follows_model_feature_order() {
        let (_dir, path) = write_csv("mass,length\n3,1\n");
        let features = vec!["length".to_string(), "mass".to_string()];
        let samples = TabularCsv::new(&path).read_samples(&features).unwrap();
        assert_eq!(samples[0].id, "row_2");
        assert_eq!(samples[0].input, SampleInput::Tabular(vec![1.0, 3.0]));
    }

    #[test]
    fn test_parse_error_reports_line_and_column() {
        let (_dir, path) = write_csv("length,species\nlong,x\n");
        let err = TabularCsv::new(&path)
            .with_target_column("species")
            .read_labeled(Task::Classification)
            .unwrap_err();
        assert!(err.to_string().contains("line 2"));
        assert!(err.to_string().contains("length"));
    }

    #[test]
    fn test_regression_target_must_be_numeric() {
        let (_dir, path) = write_csv("length,mass\n1,heavy\n");
        let result = TabularCsv::new(&path)
            .with_target_column("mass")
            .read_labeled(Task::Regression);
        assert!(result.is_err());
    }
}
