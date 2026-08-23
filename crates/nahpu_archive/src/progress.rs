//! Progress reporting for archive creation and extraction.
//!
//! Archiving a NAHPU media library runs for minutes. The types here let a caller observe
//! that work while it happens instead of waiting on a single opaque call.

use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

/// Uncompressed bytes copied between progress callbacks within a single entry.
///
/// Without this, a multi-gigabyte entry would report nothing until it finished.
const BYTE_REPORT_INTERVAL: u64 = 4 * 1024 * 1024;

/// A snapshot of an archive operation, emitted while entries are processed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArchiveProgress {
    /// Entries fully processed so far.
    pub entries_done: u64,
    /// Entries the operation expects to process, or `0` when the count is unknown.
    pub entries_total: u64,
    /// Bytes processed so far.
    pub bytes_done: u64,
    /// Bytes the operation expects to process, or `0` when the size is unknown.
    pub bytes_total: u64,
    /// Archive-relative path of the entry being processed.
    pub current_path: String,
}

/// Callback invoked with each [`ArchiveProgress`] snapshot.
pub type ProgressFn<'a> = &'a mut dyn FnMut(ArchiveProgress);

/// Sums the on-disk size of `files`, treating unreadable entries as empty.
///
/// The result is an estimate used to scale a progress bar, so a file that disappears
/// between this call and the copy must not fail the whole operation.
pub fn total_file_size(files: &[PathBuf]) -> u64 {
    files
        .iter()
        .map(|file| fs::metadata(file).map(|meta| meta.len()).unwrap_or(0))
        .sum()
}

/// Returns the final component of `path` for display in a progress snapshot.
pub fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// A [`ProgressFn`] body that discards every snapshot.
///
/// The non-reporting entry points pass this so both paths share one implementation.
pub fn discard_progress(_progress: ArchiveProgress) {}

/// Accumulates counts across one archive operation and emits throttled snapshots.
pub struct ProgressTracker<'a> {
    on_progress: ProgressFn<'a>,
    entries_done: u64,
    entries_total: u64,
    bytes_done: u64,
    bytes_total: u64,
    current_path: String,
    last_reported_bytes: u64,
}

impl<'a> ProgressTracker<'a> {
    /// Creates a tracker. Pass `0` for a total that is not known in advance.
    pub fn new(on_progress: ProgressFn<'a>, entries_total: u64, bytes_total: u64) -> Self {
        Self {
            on_progress,
            entries_done: 0,
            entries_total,
            bytes_done: 0,
            bytes_total,
            current_path: String::new(),
            last_reported_bytes: 0,
        }
    }

    /// Announces the entry that is about to be processed and emits a snapshot.
    pub fn start_entry(&mut self, path: &str) {
        self.current_path.clear();
        self.current_path.push_str(path);
        self.emit();
    }

    /// Marks the current entry complete and emits a snapshot.
    pub fn finish_entry(&mut self) {
        self.entries_done += 1;
        self.emit();
    }

    /// Adds bytes processed within the current entry, emitting at most once per interval.
    pub fn add_bytes(&mut self, bytes: u64) {
        self.bytes_done = self.bytes_done.saturating_add(bytes);
        if self.bytes_done.saturating_sub(self.last_reported_bytes) >= BYTE_REPORT_INTERVAL {
            self.emit();
        }
    }

    /// Emits a final snapshot with the totals reconciled to what was actually processed.
    ///
    /// Totals are estimates taken before the operation began; a file that changed size
    /// in between would otherwise leave the caller's bar short of, or past, the end.
    pub fn finish(&mut self) {
        self.current_path.clear();
        self.entries_total = self.entries_done;
        self.bytes_total = self.bytes_done;
        self.emit();
    }

    fn emit(&mut self) {
        self.last_reported_bytes = self.bytes_done;
        (self.on_progress)(ArchiveProgress {
            entries_done: self.entries_done,
            entries_total: self.entries_total,
            bytes_done: self.bytes_done,
            bytes_total: self.bytes_total,
            current_path: self.current_path.clone(),
        });
    }
}

/// A reader that reports every byte it yields to a [`ProgressTracker`].
///
/// Wrapping the source rather than the destination keeps progress measured in the
/// uncompressed bytes the caller can predict from file sizes.
pub struct ProgressReader<'a, 'b, R> {
    inner: R,
    tracker: &'b mut ProgressTracker<'a>,
}

impl<'a, 'b, R: Read> ProgressReader<'a, 'b, R> {
    pub fn new(inner: R, tracker: &'b mut ProgressTracker<'a>) -> Self {
        Self { inner, tracker }
    }
}

impl<R: Read> Read for ProgressReader<'_, '_, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let read = self.inner.read(buf)?;
        self.tracker.add_bytes(read as u64);
        Ok(read)
    }
}
