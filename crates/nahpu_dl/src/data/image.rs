//! Image decoding, resizing, augmentation, and normalization.

use std::path::Path;

use image::imageops::{self, FilterType};
use image::{DynamicImage, RgbImage};
use rayon::prelude::*;

use crate::data::split::SplitMix64;
use crate::data::{Sample, SampleInput};
use crate::manifest::ImagePreprocessing;
use crate::{DlError, Result};

/// File extensions read as images.
pub const IMAGE_EXTENSIONS: [&str; 5] = ["jpg", "jpeg", "png", "webp", "jfif"];

/// Range of the crop area, as a fraction of the image, for training crops.
const CROP_AREA_RANGE: (f64, f64) = (0.25, 1.0);

/// Range of the crop aspect ratio for training crops.
const CROP_RATIO_RANGE: (f64, f64) = (3.0 / 4.0, 4.0 / 3.0);

/// Maximum relative change of brightness and contrast for training images.
const COLOR_JITTER: f32 = 0.2;

/// Whether a path has an image extension.
pub fn is_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| IMAGE_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
}

/// Decodes and transforms samples into one `[batch, 3, size, size]` buffer, in parallel.
///
/// With `augment_seed`, each image gets a random crop, flip, and color jitter derived
/// from the seed and its position, so a seed reproduces the same batch.
pub(crate) fn encode_pixels(
    samples: &[Sample],
    preprocessing: &ImagePreprocessing,
    augment_seed: Option<u64>,
) -> Result<Vec<f32>> {
    let images: Vec<Vec<f32>> = samples
        .par_iter()
        .enumerate()
        .map(|(index, sample)| {
            let image = decode(sample)?;
            let pixels = match augment_seed {
                Some(seed) => {
                    let mut rng = SplitMix64::new(seed ^ (index as u64).wrapping_mul(0x9E37));
                    train_transform(&image, preprocessing, &mut rng)
                }
                None => eval_transform(&image, preprocessing),
            };
            Ok(pixels)
        })
        .collect::<Result<_>>()?;
    Ok(images.concat())
}

fn decode(sample: &Sample) -> Result<RgbImage> {
    let image = match &sample.input {
        SampleInput::ImagePath(path) => image::open(path)
            .map_err(|e| DlError::Data(format!("cannot read image {}: {e}", path.display())))?,
        SampleInput::ImageBytes(bytes) => image::load_from_memory(bytes)
            .map_err(|e| DlError::Data(format!("cannot decode image `{}`: {e}", sample.id)))?,
        SampleInput::Tabular(_) | SampleInput::Text(_) => {
            return Err(DlError::Data(format!(
                "sample `{}` is not an image",
                sample.id
            )));
        }
    };
    Ok(DynamicImage::to_rgb8(&image))
}

/// Resize the shorter side, center crop, normalize.
pub(crate) fn eval_transform(image: &RgbImage, preprocessing: &ImagePreprocessing) -> Vec<f32> {
    let size = preprocessing.size;
    let resize_to = ((size as f32 / preprocessing.crop_fraction).round() as u32).max(size);
    let (width, height) = image.dimensions();
    let scale = resize_to as f64 / width.min(height) as f64;
    let new_width = ((width as f64 * scale).round() as u32).max(size);
    let new_height = ((height as f64 * scale).round() as u32).max(size);
    let resized = imageops::resize(image, new_width, new_height, FilterType::Triangle);
    let left = (new_width - size) / 2;
    let top = (new_height - size) / 2;
    let cropped = imageops::crop_imm(&resized, left, top, size, size).to_image();
    normalize(&cropped, preprocessing, (1.0, 1.0))
}

/// Random resized crop, horizontal flip, and brightness and contrast jitter.
fn train_transform(
    image: &RgbImage,
    preprocessing: &ImagePreprocessing,
    rng: &mut SplitMix64,
) -> Vec<f32> {
    let size = preprocessing.size;
    let (left, top, width, height) = random_crop_box(image.dimensions(), rng);
    let cropped = imageops::crop_imm(image, left, top, width, height).to_image();
    let mut resized = imageops::resize(&cropped, size, size, FilterType::Triangle);
    if rng.next_f64() < 0.5 {
        imageops::flip_horizontal_in_place(&mut resized);
    }
    let jitter = |rng: &mut SplitMix64| 1.0 + COLOR_JITTER * (2.0 * rng.next_f64() as f32 - 1.0);
    let brightness = jitter(rng);
    let contrast = jitter(rng);
    normalize(&resized, preprocessing, (brightness, contrast))
}

/// Picks a crop box of random area and aspect ratio, falling back to the whole image.
fn random_crop_box((width, height): (u32, u32), rng: &mut SplitMix64) -> (u32, u32, u32, u32) {
    let area = width as f64 * height as f64;
    let (log_min, log_max) = (CROP_RATIO_RANGE.0.ln(), CROP_RATIO_RANGE.1.ln());
    for _ in 0..10 {
        let target = area * lerp(CROP_AREA_RANGE, rng.next_f64());
        let ratio = lerp((log_min, log_max), rng.next_f64()).exp();
        let crop_width = (target * ratio).sqrt().round() as u32;
        let crop_height = (target / ratio).sqrt().round() as u32;
        if (1..=width).contains(&crop_width) && (1..=height).contains(&crop_height) {
            let left = (rng.next_f64() * (width - crop_width + 1) as f64) as u32;
            let top = (rng.next_f64() * (height - crop_height + 1) as f64) as u32;
            return (
                left.min(width - crop_width),
                top.min(height - crop_height),
                crop_width,
                crop_height,
            );
        }
    }
    (0, 0, width, height)
}

fn lerp((low, high): (f64, f64), t: f64) -> f64 {
    low + (high - low) * t
}

/// Scales pixels to [0, 1], applies brightness and contrast, then normalizes per channel.
/// Output is channel-first (CHW), as the convolution layers expect.
fn normalize(
    image: &RgbImage,
    preprocessing: &ImagePreprocessing,
    (brightness, contrast): (f32, f32),
) -> Vec<f32> {
    let (width, height) = image.dimensions();
    let plane = (width * height) as usize;
    let mut output = vec![0.0; 3 * plane];
    for (index, pixel) in image.pixels().enumerate() {
        for channel in 0..3 {
            let value = f32::from(pixel[channel]) / 255.0;
            let value = (((value * brightness) - 0.5) * contrast + 0.5).clamp(0.0, 1.0);
            output[channel * plane + index] =
                (value - preprocessing.mean[channel]) / preprocessing.std[channel];
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gradient(width: u32, height: u32) -> RgbImage {
        RgbImage::from_fn(width, height, |x, y| image::Rgb([x as u8, y as u8, 128]))
    }

    #[test]
    fn test_eval_transform_shape_and_normalization() {
        let preprocessing = ImagePreprocessing {
            size: 32,
            crop_fraction: 1.0,
            mean: [0.0; 3],
            std: [1.0; 3],
        };
        let pixels = eval_transform(&gradient(64, 32), &preprocessing);
        assert_eq!(pixels.len(), 3 * 32 * 32);
        // The blue channel is constant at 128.
        let blue = &pixels[2 * 32 * 32..];
        assert!(blue.iter().all(|v| (v - 128.0 / 255.0).abs() < 1e-6));
    }

    #[test]
    fn test_train_transform_is_reproducible() {
        let preprocessing = ImagePreprocessing::default().with_size(16);
        let image = gradient(40, 30);
        let first = train_transform(&image, &preprocessing, &mut SplitMix64::new(3));
        let second = train_transform(&image, &preprocessing, &mut SplitMix64::new(3));
        let other = train_transform(&image, &preprocessing, &mut SplitMix64::new(4));
        assert_eq!(first, second);
        assert_ne!(first, other);
        assert_eq!(first.len(), 3 * 16 * 16);
    }

    #[test]
    fn test_random_crop_box_stays_inside() {
        let mut rng = SplitMix64::new(9);
        for _ in 0..100 {
            let (left, top, width, height) = random_crop_box((50, 20), &mut rng);
            assert!(left + width <= 50 && top + height <= 20);
            assert!(width > 0 && height > 0);
        }
    }

    #[test]
    fn test_is_image_path() {
        assert!(is_image_path(Path::new("a/b.JPG")));
        assert!(!is_image_path(Path::new("a/b.csv")));
    }
}
