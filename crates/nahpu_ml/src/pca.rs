//! Principal component analysis of a numeric table with missing values.

use nalgebra::{DMatrix, SymmetricEigen};
use statrs::statistics::Statistics;

use crate::error::{MlError, Result};
use crate::stats::ColumnSummary;

/// Fewest complete rows a PCA accepts.
pub const MIN_COMPLETE_ROWS: usize = 3;
/// Fewest variables a PCA accepts.
pub const MIN_VARIABLES: usize = 2;

/// Matrix the components are extracted from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PcaScaling {
    /// Standardize every variable, so each contributes equally.
    #[default]
    Correlation,
    /// Keep the original variances, so large-valued variables dominate.
    Covariance,
}

/// Options of a PCA run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcaOptions {
    /// Matrix the components are extracted from.
    pub scaling: PcaScaling,
    /// Take the natural logarithm of every value before the analysis.
    pub log_transform: bool,
    /// Treat zero and negative values as missing, as for body measurements.
    pub positive_only: bool,
    /// Largest number of components to return; `None` returns all of them.
    pub max_components: Option<usize>,
}

impl Default for PcaOptions {
    fn default() -> Self {
        Self {
            scaling: PcaScaling::default(),
            log_transform: false,
            positive_only: true,
            max_components: None,
        }
    }
}

/// Outcome of a PCA run.
#[derive(Debug, Clone, PartialEq)]
pub struct PcaResult {
    /// Indexes of the input rows used in the analysis, in input order.
    pub kept_rows: Vec<usize>,
    /// Number of input rows dropped for a missing value.
    pub excluded_rows: usize,
    /// Summary of each input column before transformation, over its usable values.
    pub column_summaries: Vec<ColumnSummary>,
    /// Variance of each component, largest first.
    pub eigenvalues: Vec<f64>,
    /// Share of the total variance each component explains.
    pub explained_variance: Vec<f64>,
    /// Running total of `explained_variance`.
    pub cumulative_variance: Vec<f64>,
    /// Unit-length loadings, indexed `[variable][component]`.
    pub loadings: Vec<Vec<f64>>,
    /// Component scores, indexed `[kept row][component]`.
    pub scores: Vec<Vec<f64>>,
    /// Pearson correlations among the analyzed variables, indexed `[variable][variable]`.
    pub correlations: Vec<Vec<f64>>,
}

/// Principal component analysis using complete rows only.
#[derive(Debug, Clone, Default)]
pub struct Pca {
    options: PcaOptions,
}

impl Pca {
    /// Creates an analysis with the given options.
    pub fn new(options: PcaOptions) -> Self {
        Self { options }
    }

    /// Runs the analysis on `rows`, where each row holds one value per variable.
    ///
    /// Rows with a missing value in any variable are dropped. The sign of each component is
    /// fixed so its largest loading is positive, which keeps repeated runs comparable.
    pub fn fit(&self, rows: &[Vec<Option<f64>>]) -> Result<PcaResult> {
        if self.options.max_components == Some(0) {
            return Err(MlError::Config(
                "max_components must be at least 1".to_string(),
            ));
        }
        let variables = Self::variable_count(rows)?;
        let column_summaries = self.summarize(rows, variables);
        let (kept_rows, values) = self.complete_cases(rows);
        let observations = kept_rows.len();
        if observations < MIN_COMPLETE_ROWS {
            return Err(MlError::TooFewRows {
                complete: observations,
                required: MIN_COMPLETE_ROWS,
            });
        }

        let mut data = DMatrix::from_row_slice(observations, variables, &values);
        let deviations = Self::center(&mut data)?;
        let divisor = (observations - 1) as f64;
        let covariance = (data.transpose() * &data) / divisor;
        let correlation = DMatrix::from_fn(variables, variables, |row, column| {
            covariance[(row, column)] / (deviations[row] * deviations[column])
        });
        let target = match self.options.scaling {
            PcaScaling::Correlation => {
                for (column, deviation) in deviations.iter().enumerate() {
                    data.column_mut(column).unscale_mut(*deviation);
                }
                correlation.clone()
            }
            PcaScaling::Covariance => covariance,
        };

        let eigen = SymmetricEigen::new(target);
        let mut order: Vec<usize> = (0..variables).collect();
        order.sort_by(|left, right| eigen.eigenvalues[*right].total_cmp(&eigen.eigenvalues[*left]));
        let total: f64 = eigen.eigenvalues.iter().map(|value| value.max(0.0)).sum();
        let components = self
            .options
            .max_components
            .unwrap_or(variables)
            .min(variables);
        order.truncate(components);

        let mut rotation = DMatrix::zeros(variables, components);
        for (component, source) in order.iter().enumerate() {
            let vector = eigen.eigenvectors.column(*source);
            let sign = Self::orientation(vector.iter());
            rotation.set_column(component, &(vector * sign));
        }
        let eigenvalues: Vec<f64> = order
            .iter()
            .map(|source| eigen.eigenvalues[*source].max(0.0))
            .collect();
        let explained_variance: Vec<f64> = eigenvalues.iter().map(|value| value / total).collect();
        let cumulative_variance = explained_variance
            .iter()
            .scan(0.0, |sum, share| {
                *sum += share;
                Some(*sum)
            })
            .collect();
        let scores = &data * &rotation;

        Ok(PcaResult {
            excluded_rows: rows.len() - observations,
            kept_rows,
            column_summaries,
            eigenvalues,
            explained_variance,
            cumulative_variance,
            loadings: Self::to_rows(&rotation),
            scores: Self::to_rows(&scores),
            correlations: Self::to_rows(&correlation),
        })
    }

    fn variable_count(rows: &[Vec<Option<f64>>]) -> Result<usize> {
        let variables = rows.first().map_or(0, Vec::len);
        if variables < MIN_VARIABLES {
            return Err(MlError::TooFewVariables {
                found: variables,
                required: MIN_VARIABLES,
            });
        }
        match rows.iter().position(|row| row.len() != variables) {
            Some(row) => Err(MlError::RaggedRow {
                row,
                found: rows[row].len(),
                expected: variables,
            }),
            None => Ok(variables),
        }
    }

    fn summarize(&self, rows: &[Vec<Option<f64>>], variables: usize) -> Vec<ColumnSummary> {
        (0..variables)
            .map(|column| {
                let values: Vec<Option<f64>> =
                    rows.iter().map(|row| self.usable(row[column])).collect();
                ColumnSummary::from_values(&values)
            })
            .collect()
    }

    /// Returns the indexes of the complete rows and their values in row-major order.
    fn complete_cases(&self, rows: &[Vec<Option<f64>>]) -> (Vec<usize>, Vec<f64>) {
        let mut kept = Vec::new();
        let mut values = Vec::new();
        for (index, row) in rows.iter().enumerate() {
            let complete: Option<Vec<f64>> = row
                .iter()
                .map(|value| self.usable(*value).and_then(|value| self.transform(value)))
                .collect();
            if let Some(complete) = complete {
                kept.push(index);
                values.extend(complete);
            }
        }
        (kept, values)
    }

    fn usable(&self, value: Option<f64>) -> Option<f64> {
        value.filter(|value| value.is_finite() && (!self.options.positive_only || *value > 0.0))
    }

    fn transform(&self, value: f64) -> Option<f64> {
        if !self.options.log_transform {
            return Some(value);
        }
        (value > 0.0).then(|| value.ln())
    }

    /// Centers every column in place and returns the sample standard deviations.
    fn center(data: &mut DMatrix<f64>) -> Result<Vec<f64>> {
        let mut deviations = Vec::with_capacity(data.ncols());
        for (index, mut column) in data.column_iter_mut().enumerate() {
            let mean = Statistics::mean(column.iter());
            let deviation = Statistics::std_dev(column.iter());
            if !deviation.is_finite() || deviation <= f64::EPSILON * mean.abs().max(1.0) {
                return Err(MlError::ConstantVariable { index });
            }
            column.add_scalar_mut(-mean);
            deviations.push(deviation);
        }
        Ok(deviations)
    }

    /// Returns -1 when the largest-magnitude entry is negative, otherwise 1.
    fn orientation<'a>(vector: impl Iterator<Item = &'a f64>) -> f64 {
        let dominant = vector.fold(0.0_f64, |dominant, value| {
            if value.abs() > dominant.abs() {
                *value
            } else {
                dominant
            }
        });
        if dominant < 0.0 { -1.0 } else { 1.0 }
    }

    fn to_rows(matrix: &DMatrix<f64>) -> Vec<Vec<f64>> {
        matrix
            .row_iter()
            .map(|row| row.iter().copied().collect())
            .collect()
    }
}
