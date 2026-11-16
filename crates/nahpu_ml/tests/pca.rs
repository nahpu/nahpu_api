use nahpu_ml::MlError;
use nahpu_ml::pca::{Pca, PcaOptions, PcaScaling};

fn close(left: f64, right: f64) -> bool {
    (left - right).abs() < 1e-9
}

fn table(rows: &[&[f64]]) -> Vec<Vec<Option<f64>>> {
    rows.iter()
        .map(|row| row.iter().copied().map(Some).collect())
        .collect()
}

/// Two variables with means 3, sample variances 2.5 and correlation 0.8.
fn two_variables() -> Vec<Vec<Option<f64>>> {
    table(&[
        &[1.0, 2.0],
        &[2.0, 1.0],
        &[3.0, 4.0],
        &[4.0, 3.0],
        &[5.0, 5.0],
    ])
}

#[test]
fn test_correlation_pca_by_hand() {
    let result = Pca::default().fit(&two_variables()).unwrap();
    let half = 0.5_f64.sqrt();

    assert_eq!(result.kept_rows, vec![0, 1, 2, 3, 4]);
    assert_eq!(result.excluded_rows, 0);
    assert!(close(result.eigenvalues[0], 1.8));
    assert!(close(result.eigenvalues[1], 0.2));
    assert!(close(result.explained_variance[0], 0.9));
    assert!(close(result.cumulative_variance[1], 1.0));
    assert!(close(result.loadings[0][0], half));
    assert!(close(result.loadings[1][0], half));
    assert!(close(result.loadings[0][1].abs(), half));
    assert!(close(result.loadings[0][1], -result.loadings[1][1]));
    assert!(close(result.correlations[0][1], 0.8));
    assert!(close(result.correlations[1][1], 1.0));
}

#[test]
fn test_covariance_pca_by_hand() {
    let options = PcaOptions {
        scaling: PcaScaling::Covariance,
        ..PcaOptions::default()
    };
    let result = Pca::new(options).fit(&two_variables()).unwrap();

    assert!(close(result.eigenvalues[0], 4.5));
    assert!(close(result.eigenvalues[1], 0.5));
    assert!(close(result.correlations[0][1], 0.8));
}

#[test]
fn test_scores_match_projection() {
    let result = Pca::default().fit(&two_variables()).unwrap();
    let deviation = 2.5_f64.sqrt();
    let half = 0.5_f64.sqrt();

    // Row 4 is (5, 5): both standardized values are 2 / sd.
    assert!(close(result.scores[4][0], 2.0 * (2.0 / deviation) * half));
    assert!(close(result.scores[4][1], 0.0));
    for component in 0..2 {
        let mean: f64 = result.scores.iter().map(|row| row[component]).sum::<f64>() / 5.0;
        let variance: f64 = result
            .scores
            .iter()
            .map(|row| (row[component] - mean).powi(2))
            .sum::<f64>()
            / 4.0;
        assert!(close(mean, 0.0));
        assert!(close(variance, result.eigenvalues[component]));
    }
}

#[test]
fn test_perfectly_correlated_columns_leave_one_component() {
    let rows = table(&[&[1.0, 2.0], &[2.0, 4.0], &[3.0, 6.0], &[4.0, 8.0]]);
    let result = Pca::default().fit(&rows).unwrap();

    assert!(close(result.eigenvalues[0], 2.0));
    assert!(close(result.eigenvalues[1], 0.0));
    assert!(close(result.explained_variance[0], 1.0));
}

#[test]
fn test_incomplete_rows_are_dropped() {
    let mut rows = two_variables();
    rows.insert(1, vec![Some(9.0), None]);
    rows.push(vec![Some(0.0), Some(7.0)]);
    rows.push(vec![Some(f64::NAN), Some(7.0)]);
    let result = Pca::default().fit(&rows).unwrap();

    assert_eq!(result.kept_rows, vec![0, 2, 3, 4, 5]);
    assert_eq!(result.excluded_rows, 3);
    assert_eq!(result.scores.len(), 5);
    assert!(close(result.eigenvalues[0], 1.8));
    // The summary covers every usable value, including rows the PCA dropped.
    assert_eq!(result.column_summaries[0].n, 6);
    assert_eq!(result.column_summaries[0].missing, 2);
    assert_eq!(result.column_summaries[1].n, 7);
    assert_eq!(result.column_summaries[1].missing, 1);
}

#[test]
fn test_zero_is_kept_when_positive_only_is_off() {
    let mut rows = two_variables();
    rows.push(vec![Some(0.0), Some(7.0)]);
    let options = PcaOptions {
        positive_only: false,
        ..PcaOptions::default()
    };
    let result = Pca::new(options).fit(&rows).unwrap();

    assert_eq!(result.kept_rows.len(), 6);
}

#[test]
fn test_log_transform_linearizes_power_relation() {
    // y = x^2 is a straight line after taking logarithms.
    let rows = table(&[&[1.0, 1.0], &[2.0, 4.0], &[3.0, 9.0], &[4.0, 16.0]]);
    let options = PcaOptions {
        log_transform: true,
        ..PcaOptions::default()
    };
    let result = Pca::new(options).fit(&rows).unwrap();

    assert!(close(result.explained_variance[0], 1.0));
    assert!(close(result.correlations[0][1], 1.0));
}

#[test]
fn test_max_components_limits_output() {
    let rows = table(&[
        &[1.0, 2.0, 9.0],
        &[2.0, 1.0, 7.0],
        &[3.0, 4.0, 8.0],
        &[4.0, 3.0, 2.0],
        &[5.0, 5.0, 4.0],
    ]);
    let all = Pca::default().fit(&rows).unwrap();
    let options = PcaOptions {
        max_components: Some(2),
        ..PcaOptions::default()
    };
    let limited = Pca::new(options).fit(&rows).unwrap();

    assert_eq!(all.eigenvalues.len(), 3);
    assert!(close(all.explained_variance.iter().sum::<f64>(), 1.0));
    assert_eq!(limited.eigenvalues.len(), 2);
    assert_eq!(limited.loadings.len(), 3);
    assert_eq!(limited.loadings[0].len(), 2);
    assert_eq!(limited.scores[0].len(), 2);
    assert!(close(
        limited.explained_variance[0],
        all.explained_variance[0]
    ));
    assert!(limited.cumulative_variance[1] < 1.0);
}

#[test]
fn test_component_signs_are_deterministic() {
    let rows = table(&[
        &[1.0, 9.0, 3.0],
        &[2.0, 7.5, 1.0],
        &[3.0, 6.0, 4.0],
        &[4.0, 4.0, 1.5],
        &[5.0, 1.0, 5.0],
    ]);
    let mut reversed = rows.clone();
    reversed.reverse();
    let forward = Pca::default().fit(&rows).unwrap();
    let backward = Pca::default().fit(&reversed).unwrap();

    for (left, right) in forward.loadings.iter().zip(&backward.loadings) {
        for (left, right) in left.iter().zip(right) {
            assert!(close(*left, *right));
        }
    }
    for component in 0..3 {
        let dominant =
            forward
                .loadings
                .iter()
                .map(|row| row[component])
                .fold(0.0_f64, |dominant, value| {
                    if value.abs() > dominant.abs() {
                        value
                    } else {
                        dominant
                    }
                });
        assert!(dominant > 0.0);
    }
}

#[test]
fn test_too_few_complete_rows_is_an_error() {
    let rows = vec![
        vec![Some(1.0), Some(2.0)],
        vec![Some(2.0), Some(1.0)],
        vec![Some(3.0), None],
    ];
    let error = Pca::default().fit(&rows).unwrap_err();

    assert_eq!(
        error,
        MlError::TooFewRows {
            complete: 2,
            required: 3
        }
    );
    assert!(error.to_string().contains("only 2 complete rows"));
}

#[test]
fn test_single_variable_is_an_error() {
    let rows = table(&[&[1.0], &[2.0], &[3.0]]);

    assert!(matches!(
        Pca::default().fit(&rows),
        Err(MlError::TooFewVariables { found: 1, .. })
    ));
    assert!(matches!(
        Pca::default().fit(&[]),
        Err(MlError::TooFewVariables { found: 0, .. })
    ));
}

#[test]
fn test_ragged_rows_are_an_error() {
    let rows = vec![
        vec![Some(1.0), Some(2.0)],
        vec![Some(2.0)],
        vec![Some(3.0), Some(4.0)],
    ];
    let error = Pca::default().fit(&rows).unwrap_err();

    assert_eq!(
        error,
        MlError::RaggedRow {
            row: 1,
            found: 1,
            expected: 2
        }
    );
}

#[test]
fn test_constant_variable_is_an_error() {
    let rows = table(&[&[1.0, 5.0], &[2.0, 5.0], &[3.0, 5.0], &[4.0, 5.0]]);
    let error = Pca::default().fit(&rows).unwrap_err();

    assert_eq!(error, MlError::ConstantVariable { index: 1 });
}

#[test]
fn test_zero_max_components_is_a_config_error() {
    let options = PcaOptions {
        max_components: Some(0),
        ..PcaOptions::default()
    };

    assert!(matches!(
        Pca::new(options).fit(&two_variables()),
        Err(MlError::Config(_))
    ));
}
