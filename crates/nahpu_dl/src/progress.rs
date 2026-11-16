/// Receives progress updates from long-running operations.
///
/// The CLI implements it with progress bars; the app can forward updates to its UI. All
/// methods have no-op defaults.
pub trait ProgressReporter {
    /// A new stage starts with `total` steps (batches or epochs).
    fn start(&self, _stage: &str, _total: usize) {}

    /// `steps` more steps of the current stage are done.
    fn advance(&self, _steps: usize) {}

    /// Reports a status message, such as the metrics of the last epoch.
    fn message(&self, _message: &str) {}

    /// The current stage is done.
    fn finish(&self) {}
}

/// A [`ProgressReporter`] that ignores every update.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct NoProgress;

impl ProgressReporter for NoProgress {}
