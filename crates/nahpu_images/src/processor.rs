//! Image decode, transform, encode, and file-writing pipeline.

use std::{
    fs,
    io::{Cursor, Write},
    path::Path,
};

use image::{
    DynamicImage, ExtendedColorType, GenericImageView, ImageDecoder, ImageEncoder, ImageFormat,
    ImageReader, RgbImage,
    codecs::{jpeg::JpegEncoder, png::PngDecoder, png::PngEncoder, webp::WebPDecoder},
    imageops::FilterType,
};
use tempfile::Builder;

use crate::{
    ImageError, ImageFileFormat, ImageInfo, ImageOptions, ProcessedImage, ResizeMode,
    ResizeOptions, RgbColor, metadata::exif,
};

/// Stateless entry point for image conversion and resizing.
pub struct ImageProcessor;

impl ImageProcessor {
    /// Converts and optionally resizes an encoded image held in memory.
    pub fn process_bytes(input: &[u8], options: &ImageOptions) -> Result<ProcessedImage, ImageError> {
        validate_options(options)?;
        let source_format = detect_format(input)?;
        reject_animation(input, source_format)?;

        let reader = ImageReader::with_format(Cursor::new(input), image_format(source_format));
        let mut decoder = reader.into_decoder().map_err(ImageError::Decode)?;
        let orientation = decoder.orientation().map_err(ImageError::Decode)?;
        let has_exif = decoder
            .exif_metadata()
            .map_err(ImageError::Decode)?
            .is_some();
        let mut image = DynamicImage::from_decoder(decoder).map_err(ImageError::Decode)?;
        image.apply_orientation(orientation);

        let (source_width, source_height) = image.dimensions();
        let image = resize_image(image, options.resize);
        let (output_width, output_height) = image.dimensions();
        let mut bytes = encode(&image, options)?;

        let exif_preserved = if has_exif {
            let mut metadata = exif::read(input, source_format)?;
            exif::normalize(&mut metadata, output_width, output_height);
            exif::write(&metadata, &mut bytes, options.output_format)?;
            true
        } else {
            false
        };

        let output_bytes = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        Ok(ProcessedImage {
            bytes,
            info: ImageInfo {
                source_format,
                source_width,
                source_height,
                output_format: options.output_format,
                output_width,
                output_height,
                output_bytes,
                resized: (source_width, source_height) != (output_width, output_height),
                exif_preserved,
            },
        })
    }

    /// Processes an image file and safely writes the encoded result to `output_path`.
    ///
    /// Existing destinations are rejected unless `overwrite` is true. The complete image is
    /// written to a temporary file beside the destination before it is persisted, preventing a
    /// decode or encode failure from leaving a partial output.
    pub fn process_file(
        input_path: impl AsRef<Path>,
        output_path: impl AsRef<Path>,
        options: &ImageOptions,
        overwrite: bool,
    ) -> Result<ImageInfo, ImageError> {
        let input_path = input_path.as_ref();
        let output_path = output_path.as_ref();
        validate_output_extension(output_path, options.output_format)?;

        if output_path.exists() && !overwrite {
            return Err(ImageError::DestinationExists(output_path.to_path_buf()));
        }

        let input = fs::read(input_path).map_err(|source| ImageError::Io {
            operation: "read input image",
            path: input_path.to_path_buf(),
            source,
        })?;
        let processed = Self::process_bytes(&input, options)?;
        let parent = output_path.parent().unwrap_or_else(|| Path::new("."));
        let mut temporary = Builder::new()
            .prefix(".nahpu-image-")
            .tempfile_in(parent)
            .map_err(|source| ImageError::Io {
                operation: "create temporary output",
                path: output_path.to_path_buf(),
                source,
            })?;
        temporary
            .write_all(&processed.bytes)
            .and_then(|()| temporary.as_file().sync_all())
            .map_err(|source| ImageError::Io {
                operation: "write temporary output",
                path: output_path.to_path_buf(),
                source,
            })?;
        temporary
            .persist(output_path)
            .map_err(|error| ImageError::Io {
                operation: "persist output image",
                path: output_path.to_path_buf(),
                source: error.error,
            })?;
        Ok(processed.info)
    }
}

fn validate_options(options: &ImageOptions) -> Result<(), ImageError> {
    if options.output_format == ImageFileFormat::Jpeg && !(1..=100).contains(&options.jpeg_quality) {
        return Err(ImageError::InvalidOptions(
            "JPEG quality must be between 1 and 100".to_owned(),
        ));
    }
    let Some(resize) = options.resize else {
        return Ok(());
    };
    if resize.width == Some(0) || resize.height == Some(0) {
        return Err(ImageError::InvalidOptions(
            "resize dimensions must be greater than zero".to_owned(),
        ));
    }
    match resize.mode {
        ResizeMode::Fit if resize.width.is_none() && resize.height.is_none() => {
            Err(ImageError::InvalidOptions(
                "fit resizing requires a width or height bound".to_owned(),
            ))
        }
        ResizeMode::Fill | ResizeMode::Exact
            if resize.width.is_none() || resize.height.is_none() =>
        {
            Err(ImageError::InvalidOptions(
                "fill and exact resizing require both width and height".to_owned(),
            ))
        }
        _ => Ok(()),
    }
}

fn detect_format(input: &[u8]) -> Result<ImageFileFormat, ImageError> {
    match image::guess_format(input).map_err(|_| ImageError::UnsupportedFormat)? {
        ImageFormat::Jpeg => Ok(ImageFileFormat::Jpeg),
        ImageFormat::Png => Ok(ImageFileFormat::Png),
        ImageFormat::WebP => Ok(ImageFileFormat::WebP),
        _ => Err(ImageError::UnsupportedFormat),
    }
}

fn reject_animation(input: &[u8], format: ImageFileFormat) -> Result<(), ImageError> {
    let animated = match format {
        ImageFileFormat::Jpeg => false,
        ImageFileFormat::Png => PngDecoder::new(Cursor::new(input))
            .map_err(ImageError::Decode)?
            .is_apng()
            .map_err(ImageError::Decode)?,
        ImageFileFormat::WebP => WebPDecoder::new(Cursor::new(input))
            .map_err(ImageError::Decode)?
            .has_animation(),
    };
    if animated {
        Err(ImageError::UnsupportedAnimation(format))
    } else {
        Ok(())
    }
}

fn resize_image(image: DynamicImage, resize: Option<ResizeOptions>) -> DynamicImage {
    let Some(resize) = resize else {
        return image;
    };
    let (source_width, source_height) = image.dimensions();
    let (target_width, target_height) = target_dimensions(source_width, source_height, resize);
    match resize.mode {
        ResizeMode::Fit => image.resize(target_width, target_height, FilterType::Lanczos3),
        ResizeMode::Fill => {
            image.resize_to_fill(target_width, target_height, FilterType::Lanczos3)
        }
        ResizeMode::Exact => {
            image.resize_exact(target_width, target_height, FilterType::Lanczos3)
        }
    }
}

fn target_dimensions(source_width: u32, source_height: u32, resize: ResizeOptions) -> (u32, u32) {
    match resize.mode {
        ResizeMode::Fit => {
            let width_ratio = resize
                .width
                .map(|width| f64::from(width) / f64::from(source_width))
                .unwrap_or(f64::INFINITY);
            let height_ratio = resize
                .height
                .map(|height| f64::from(height) / f64::from(source_height))
                .unwrap_or(f64::INFINITY);
            let mut ratio = width_ratio.min(height_ratio);
            if !resize.allow_upscale {
                ratio = ratio.min(1.0);
            }
            (
                scaled_dimension(source_width, ratio),
                scaled_dimension(source_height, ratio),
            )
        }
        ResizeMode::Fill | ResizeMode::Exact => {
            let width = resize.width.expect("validated resize width");
            let height = resize.height.expect("validated resize height");
            if resize.allow_upscale {
                (width, height)
            } else {
                (width.min(source_width), height.min(source_height))
            }
        }
    }
}

fn scaled_dimension(source: u32, ratio: f64) -> u32 {
    ((f64::from(source) * ratio).round() as u32).max(1)
}

fn encode(image: &DynamicImage, options: &ImageOptions) -> Result<Vec<u8>, ImageError> {
    let mut output = Vec::new();
    let (width, height) = image.dimensions();
    match options.output_format {
        ImageFileFormat::Jpeg => {
            let rgb = composite_for_jpeg(image, options.jpeg_background);
            JpegEncoder::new_with_quality(&mut output, options.jpeg_quality)
                .write_image(rgb.as_raw(), width, height, ExtendedColorType::Rgb8)
                .map_err(ImageError::Encode)?;
        }
        ImageFileFormat::Png => {
            PngEncoder::new(&mut output)
                .write_image(image.as_bytes(), width, height, image.color().into())
                .map_err(ImageError::Encode)?;
        }
        ImageFileFormat::WebP => {
            let rgba = image.to_rgba8();
            image::codecs::webp::WebPEncoder::new_lossless(&mut output)
                .write_image(rgba.as_raw(), width, height, ExtendedColorType::Rgba8)
                .map_err(ImageError::Encode)?;
        }
    }
    Ok(output)
}

fn composite_for_jpeg(image: &DynamicImage, background: RgbColor) -> RgbImage {
    let rgba = image.to_rgba8();
    RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let pixel = rgba.get_pixel(x, y).0;
        let alpha = u16::from(pixel[3]);
        let inverse_alpha = 255 - alpha;
        image::Rgb([
            blend(pixel[0], background.red, alpha, inverse_alpha),
            blend(pixel[1], background.green, alpha, inverse_alpha),
            blend(pixel[2], background.blue, alpha, inverse_alpha),
        ])
    })
}

fn blend(foreground: u8, background: u8, alpha: u16, inverse_alpha: u16) -> u8 {
    let value = u16::from(foreground) * alpha + u16::from(background) * inverse_alpha;
    ((value + 127) / 255) as u8
}

fn validate_output_extension(path: &Path, format: ImageFileFormat) -> Result<(), ImageError> {
    let extension = path.extension().and_then(|value| value.to_str());
    let valid = extension.is_some_and(|extension| match format {
        ImageFileFormat::Jpeg => {
            extension.eq_ignore_ascii_case("jpg") || extension.eq_ignore_ascii_case("jpeg")
        }
        ImageFileFormat::Png => extension.eq_ignore_ascii_case("png"),
        ImageFileFormat::WebP => extension.eq_ignore_ascii_case("webp"),
    });
    if valid {
        Ok(())
    } else {
        Err(ImageError::OutputExtension {
            path: path.to_path_buf(),
            expected: match format {
                ImageFileFormat::Jpeg => ".jpg or .jpeg",
                ImageFileFormat::Png => ".png",
                ImageFileFormat::WebP => ".webp",
            },
        })
    }
}

fn image_format(format: ImageFileFormat) -> ImageFormat {
    match format {
        ImageFileFormat::Jpeg => ImageFormat::Jpeg,
        ImageFileFormat::Png => ImageFormat::Png,
        ImageFileFormat::WebP => ImageFormat::WebP,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_dimensions_respect_one_bound() {
        let resize = ResizeOptions::fit(Some(200), None);
        assert_eq!(target_dimensions(400, 200, resize), (200, 100));
    }

    #[test]
    fn fit_does_not_upscale_by_default() {
        let resize = ResizeOptions::fit(Some(800), Some(800));
        assert_eq!(target_dimensions(400, 200, resize), (400, 200));
    }

    #[test]
    fn exact_clamps_each_axis_without_upscaling() {
        let resize = ResizeOptions::exact(800, 100);
        assert_eq!(target_dimensions(400, 200, resize), (400, 100));
    }
}
