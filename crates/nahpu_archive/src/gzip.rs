//! Gzip compression utilities for single-file Darwin Core Data Package envelopes.

use std::{fs::File, io, path::Path};

use flate2::{Compression, read::GzDecoder, write::GzEncoder};

/// Compresses one file as a gzip stream.
pub fn compress(input_path: &Path, output_path: &Path) -> io::Result<()> {
    let mut input = File::open(input_path)?;
    let output = File::create(output_path)?;
    let mut encoder = GzEncoder::new(output, Compression::default());
    io::copy(&mut input, &mut encoder)?;
    encoder.finish()?;
    Ok(())
}

/// Decompresses one gzip stream into a file.
pub fn decompress(input_path: &Path, output_path: &Path) -> io::Result<()> {
    let input = File::open(input_path)?;
    let mut decoder = GzDecoder::new(input);
    let mut output = File::create(output_path)?;
    io::copy(&mut decoder, &mut output)?;
    Ok(())
}
