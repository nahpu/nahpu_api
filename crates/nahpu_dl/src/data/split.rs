use std::collections::BTreeMap;

use crate::data::Target;
use crate::{DlError, Result};

/// Indices of the training, validation, and test samples of a dataset.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DataSplit {
    /// Training indices.
    pub train: Vec<usize>,
    /// Validation indices, used for early stopping.
    pub valid: Vec<usize>,
    /// Held-out test indices.
    pub test: Vec<usize>,
}

impl DataSplit {
    /// Splits samples reproducibly from a seed.
    ///
    /// Class targets are split per class (stratified) so every split keeps the class
    /// balance. Each class keeps at least one training sample.
    ///
    /// # Errors
    ///
    /// Returns [`DlError::Config`] when the fractions are negative or sum to one or more.
    pub fn new(
        targets: &[Target],
        valid_fraction: f64,
        test_fraction: f64,
        seed: u64,
    ) -> Result<Self> {
        let fractions_ok = (0.0..1.0).contains(&valid_fraction)
            && (0.0..1.0).contains(&test_fraction)
            && valid_fraction + test_fraction < 1.0;
        if !fractions_ok {
            return Err(DlError::Config(format!(
                "validation ({valid_fraction}) and test ({test_fraction}) fractions must be \
                 non-negative and sum to less than 1"
            )));
        }
        let mut rng = SplitMix64::new(seed);
        let mut split = Self::default();
        for mut group in group_by_class(targets) {
            rng.shuffle(&mut group);
            split.assign_group(&group, valid_fraction, test_fraction);
        }
        Ok(split)
    }

    fn assign_group(&mut self, group: &[usize], valid_fraction: f64, test_fraction: f64) {
        let n = group.len();
        let n_test = fraction_count(n, test_fraction, n.saturating_sub(1));
        let n_valid = fraction_count(n, valid_fraction, n.saturating_sub(n_test + 1));
        let (test, rest) = group.split_at(n_test);
        let (valid, train) = rest.split_at(n_valid);
        self.test.extend_from_slice(test);
        self.valid.extend_from_slice(valid);
        self.train.extend_from_slice(train);
    }
}

/// Rounds `n * fraction`, capped so training keeps at least one sample.
fn fraction_count(n: usize, fraction: f64, max: usize) -> usize {
    ((n as f64 * fraction).round() as usize).min(max)
}

/// Groups indices by class label. Numeric targets form a single group.
fn group_by_class(targets: &[Target]) -> Vec<Vec<usize>> {
    let mut groups: BTreeMap<Option<&str>, Vec<usize>> = BTreeMap::new();
    for (index, target) in targets.iter().enumerate() {
        let key = match target {
            Target::Class(label) => Some(label.as_str()),
            Target::Value(_) => None,
        };
        groups.entry(key).or_default().push(index);
    }
    groups.into_values().collect()
}

/// Small seeded generator for shuffling.
///
/// It is written out here, instead of using `rand`, so that a seed gives the same split on
/// every platform and across dependency upgrades.
#[derive(Debug, Clone)]
pub(crate) struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub(crate) fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub(crate) fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Fisher-Yates shuffle.
    pub(crate) fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = (self.next_u64() % (i as u64 + 1)) as usize;
            items.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn class_targets() -> Vec<Target> {
        (0..30)
            .map(|i| Target::Class(if i < 20 { "a" } else { "b" }.to_string()))
            .collect()
    }

    #[test]
    fn test_split_is_stratified() {
        let targets = class_targets();
        let split = DataSplit::new(&targets, 0.2, 0.1, 7).unwrap();
        let count = |indices: &[usize], label: &str| {
            indices
                .iter()
                .filter(|&&i| targets[i] == Target::Class(label.to_string()))
                .count()
        };
        assert_eq!(count(&split.test, "a"), 2);
        assert_eq!(count(&split.test, "b"), 1);
        assert_eq!(count(&split.valid, "a"), 4);
        assert_eq!(count(&split.valid, "b"), 2);
        assert_eq!(split.train.len() + split.valid.len() + split.test.len(), 30);
    }

    #[test]
    fn test_split_is_reproducible() {
        let targets = class_targets();
        let first = DataSplit::new(&targets, 0.2, 0.1, 7).unwrap();
        let second = DataSplit::new(&targets, 0.2, 0.1, 7).unwrap();
        let other = DataSplit::new(&targets, 0.2, 0.1, 8).unwrap();
        assert_eq!(first, second);
        assert_ne!(first, other);
    }

    #[test]
    fn test_split_keeps_one_training_sample_per_class() {
        let targets = vec![
            Target::Class("a".to_string()),
            Target::Class("b".to_string()),
        ];
        let split = DataSplit::new(&targets, 0.5, 0.4, 1).unwrap();
        assert_eq!(split.train.len(), 2);
    }

    #[test]
    fn test_split_rejects_bad_fractions() {
        let targets = class_targets();
        assert!(DataSplit::new(&targets, 0.6, 0.4, 1).is_err());
        assert!(DataSplit::new(&targets, -0.1, 0.1, 1).is_err());
    }

    #[test]
    fn test_split_regression_targets() {
        let targets: Vec<Target> = (0..10).map(|i| Target::Value(i as f32)).collect();
        let split = DataSplit::new(&targets, 0.2, 0.2, 3).unwrap();
        assert_eq!(
            (split.train.len(), split.valid.len(), split.test.len()),
            (6, 2, 2)
        );
    }
}
