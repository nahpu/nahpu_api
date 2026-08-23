//! Gzip compression utilities for single-file Darwin Core Data Package envelopes.

use std::{
    fs::{self, File},
    io,
    path::Path,
};

use flate2::{Compression, read::GzDecoder, write::GzEncoder};

use crate::progress::{
    ProgressFn, ProgressReader, ProgressTracker, discard_progress, display_name,
};

/// Compresses one file as a gzip stream.
pub fn compress(input_path: &Path, output_path: &Path) -> io::Result<()> {
    compress_with_progress(input_path, output_path, &mut discard_progress)
}

/// Compresses one file as a gzip stream, reporting bytes read to `on_progress`.
pub fn compress_with_progress(
    input_path: &Path,
    output_path: &Path,
    on_progress: ProgressFn<'_>,
) -> io::Result<()> {
    let total = file_size(input_path);
    let mut tracker = ProgressTracker::new(on_progress, 1, total);
    tracker.start_entry(&display_name(input_path));

    let input = File::open(input_path)?;
    let output = File::create(output_path)?;
    let mut encoder = GzEncoder::new(output, Compression::default());
    io::copy(&mut ProgressReader::new(input, &mut tracker), &mut encoder)?;
    encoder.finish()?;

    tracker.finish_entry();
    tracker.finish();
    Ok(())
}

/// Decompresses one gzip stream into a file.
pub fn decompress(input_path: &Path, output_path: &Path) -> io::Result<()> {
    decompress_with_progress(input_path, output_path, &mut discard_progress)
}

/// Decompresses one gzip stream into a file, reporting bytes read to `on_progress`.
///
/// Progress counts compressed bytes consumed from the archive, because the decompressed
/// size is not recorded anywhere the caller can read before the stream ends.
pub fn decompress_with_progress(
    input_path: &Path,
    output_path: &Path,
    on_progress: ProgressFn<'_>,
) -> io::Result<()> {
    let total = file_size(input_path);
    let mut tracker = ProgressTracker::new(on_progress, 1, total);
    tracker.start_entry(&display_name(output_path));

    let input = File::open(input_path)?;
    let mut output = File::create(output_path)?;
    {
        let mut decoder = GzDecoder::new(ProgressReader::new(input, &mut tracker));
        io::copy(&mut decoder, &mut output)?;
    }

    tracker.finish_entry();
    tracker.finish();
    Ok(())
}

fn file_size(path: &Path) -> u64 {
    fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
}
