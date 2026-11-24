//! Descriptive statistics for one numeric column.

use statrs::statistics::{Data, Median, Statistics};

/// Summary of the usable values in one column.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnSummary {
    /// Number of usable values.
    pub n: usize,
    /// Number of missing or non-finite values.
    pub missing: usize,
    /// Arithmetic mean, when at least one value is usable.
    pub mean: Option<f64>,
    /// Sample standard deviation (n - 1), when at least two values are usable.
    pub sd: Option<f64>,
    /// Smallest usable value.
    pub min: Option<f64>,
    /// Largest usable value.
    pub max: Option<f64>,
    /// Median of the usable values.
    pub median: Option<f64>,
}

impl ColumnSummary {
    /// Summarizes a column, counting `None` and non-finite values as missing.
    pub fn from_values(values: &[Option<f64>]) -> Self {
        let usable: Vec<f64> = values
            .iter()
            .flatten()
            .copied()
            .filter(|value| value.is_finite())
            .collect();
        let n = usable.len();
        let missing = values.len() - n;
        if n == 0 {
            return Self {
                n,
                missing,
                mean: None,
                sd: None,
                min: None,
                max: None,
                median: None,
            };
        }
        Self {
            n,
            missing,
            mean: Some(Statistics::mean(&usable)),
            sd: (n > 1).then(|| Statistics::std_dev(&usable)),
            min: Some(Statistics::min(&usable)),
            max: Some(Statistics::max(&usable)),
            median: Some(Data::new(usable).median()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(left: f64, right: f64) -> bool {
        (left - right).abs() < 1e-9
    }

    #[test]
    fn test_summary_by_hand() {
        let summary = ColumnSummary::from_values(&[
            Some(2.0),
            None,
            Some(4.0),
            Some(4.0),
            Some(f64::NAN),
            Some(6.0),
        ]);
        assert_eq!(summary.n, 4);
        assert_eq!(summary.missing, 2);
        assert!(close(summary.mean.unwrap(), 4.0));
        assert!(close(summary.sd.unwrap(), (8.0_f64 / 3.0).sqrt()));
        assert!(close(summary.min.unwrap(), 2.0));
        assert!(close(summary.max.unwrap(), 6.0));
        assert!(close(summary.median.unwrap(), 4.0));
    }

    #[test]
    fn test_summary_of_single_value_has_no_sd() {
        let summary = ColumnSummary::from_values(&[Some(3.5)]);
        assert_eq!(summary.n, 1);
        assert_eq!(summary.sd, None);
        assert!(close(summary.median.unwrap(), 3.5));
    }

    #[test]
    fn test_summary_of_empty_column() {
        let summary = ColumnSummary::from_values(&[None, None]);
        assert_eq!(summary.n, 0);
        assert_eq!(summary.missing, 2);
        assert_eq!(summary.mean, None);
    }
}
