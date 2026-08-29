use std::{fs, io::Cursor};

use image::{
    DynamicImage, ExtendedColorType, GenericImageView, ImageEncoder, ImageFormat, Rgba,
    RgbaImage,
    codecs::{jpeg::JpegEncoder, png::PngEncoder, webp::WebPEncoder},
};
use little_exif::{exif_tag::ExifTag, filetype::FileExtension, metadata::Metadata};
use nahpu_images::{
    ImageError, ImageFileFormat, ImageOptions, ImageProcessor, ResizeOptions, RgbColor,
};

#[test]
fn converts_every_supported_input_to_every_supported_output() {
    let formats = [
        ImageFileFormat::Jpeg,
        ImageFileFormat::Png,
        ImageFileFormat::WebP,
    ];

    for source_format in formats {
        let source = encode_fixture(source_format, false);
        for output_format in formats {
            let result = ImageProcessor::process_bytes(
                &source,
                &ImageOptions::new(output_format),
            )
            .expect("supported conversion should succeed");

            assert_eq!(result.info.source_format, source_format);
            assert_eq!(result.info.output_format, output_format);
            assert_eq!((result.info.output_width, result.info.output_height), (8, 4));
            assert_eq!(detected_format(&result.bytes), image_format(output_format));
            assert_eq!(
                image::load_from_memory(&result.bytes)
                    .expect("output should decode")
                    .dimensions(),
                (8, 4)
            );
        }
    }
}

#[test]
fn supports_fit_fill_exact_and_explicit_upscaling() {
    let source = encode_fixture(ImageFileFormat::Png, false);
    let cases = [
        (ResizeOptions::fit(Some(4), Some(4)), (4, 2)),
        (ResizeOptions::fill(3, 3), (3, 3)),
        (ResizeOptions::exact(3, 2), (3, 2)),
        (
            ResizeOptions::exact(16, 12).with_upscaling(true),
            (16, 12),
        ),
        (ResizeOptions::exact(16, 12), (8, 4)),
    ];

    for (resize, expected) in cases {
        let result = ImageProcessor::process_bytes(
            &source,
            &ImageOptions::new(ImageFileFormat::Png).with_resize(resize),
        )
        .expect("resize should succeed");
        assert_eq!((result.info.output_width, result.info.output_height), expected);
    }
}

#[test]
fn validates_resize_and_jpeg_quality_options() {
    let source = encode_fixture(ImageFileFormat::Png, false);
    let invalid_resizes = [
        ResizeOptions::fit(None, None),
        ResizeOptions::fit(Some(0), Some(4)),
        ResizeOptions {
            width: Some(4),
            height: None,
            mode: nahpu_images::ResizeMode::Fill,
            allow_upscale: false,
        },
    ];

    for resize in invalid_resizes {
        let error = ImageProcessor::process_bytes(
            &source,
            &ImageOptions::new(ImageFileFormat::Png).with_resize(resize),
        )
        .expect_err("invalid resize should fail");
        assert!(matches!(error, ImageError::InvalidOptions(_)));
    }

    let error = ImageProcessor::process_bytes(
        &source,
        &ImageOptions::new(ImageFileFormat::Jpeg).with_jpeg_quality(0),
    )
    .expect_err("invalid JPEG quality should fail");
    assert!(matches!(error, ImageError::InvalidOptions(_)));
}

#[test]
fn composites_jpeg_alpha_onto_the_configured_background() {
    let source = encode_transparent_pixel();
    let options = ImageOptions::new(ImageFileFormat::Jpeg)
        .with_jpeg_quality(100)
        .with_jpeg_background(RgbColor::new(0, 0, 255));
    let result = ImageProcessor::process_bytes(&source, &options).expect("conversion should work");
    let output = image::load_from_memory(&result.bytes)
        .expect("JPEG should decode")
        .to_rgb8();
    let pixel = output.get_pixel(0, 0).0;

    assert!((i16::from(pixel[0]) - 128).abs() <= 4);
    assert!(pixel[1] <= 4);
    assert!((i16::from(pixel[2]) - 127).abs() <= 4);
}

#[test]
fn retains_alpha_in_png_and_webp_outputs() {
    let source = encode_transparent_pixel();
    for format in [ImageFileFormat::Png, ImageFileFormat::WebP] {
        let result = ImageProcessor::process_bytes(&source, &ImageOptions::new(format))
            .expect("conversion should work");
        let pixel = image::load_from_memory(&result.bytes)
            .expect("output should decode")
            .to_rgba8()
            .get_pixel(0, 0)
            .0;
        assert_eq!(pixel, [255, 0, 0, 128]);
    }
}

#[test]
fn normalizes_orientation_and_preserves_exif_across_formats() {
    let mut source = encode_fixture(ImageFileFormat::Jpeg, false);
    let mut metadata = Metadata::new();
    metadata.set_tag(ExifTag::Orientation(vec![6]));
    metadata.set_tag(ExifTag::ImageDescription("NAHPU field image".to_owned()));
    metadata
        .write_to_vec(&mut source, FileExtension::JPEG)
        .expect("fixture EXIF should write");

    let result = ImageProcessor::process_bytes(
        &source,
        &ImageOptions::new(ImageFileFormat::Png)
            .with_resize(ResizeOptions::fit(Some(2), Some(2))),
    )
    .expect("oriented image should process");

    assert_eq!((result.info.source_width, result.info.source_height), (4, 8));
    assert_eq!((result.info.output_width, result.info.output_height), (1, 2));
    assert!(result.info.exif_preserved);

    let output_metadata = Metadata::new_from_vec(
        &result.bytes,
        FileExtension::PNG {
            as_zTXt_chunk: false,
        },
    )
    .expect("output EXIF should decode");
    assert!(has_tag(&output_metadata, |tag| matches!(
        tag,
        ExifTag::Orientation(values) if values == &[1]
    )));
    assert!(has_tag(&output_metadata, |tag| matches!(
        tag,
        ExifTag::ExifImageWidth(values) if values == &[1]
    )));
    assert!(has_tag(&output_metadata, |tag| matches!(
        tag,
        ExifTag::ExifImageHeight(values) if values == &[2]
    )));
    assert!(has_tag(&output_metadata, |tag| matches!(
        tag,
        ExifTag::ImageDescription(value) if value == "NAHPU field image"
    )));
}

#[test]
fn path_api_protects_destinations_and_supports_in_place_processing() {
    let directory = tempfile::tempdir().expect("temporary directory should exist");
    let input_path = directory.path().join("input.png");
    let output_path = directory.path().join("output.webp");
    fs::write(&input_path, encode_fixture(ImageFileFormat::Png, false))
        .expect("fixture should write");
    fs::write(&output_path, b"keep me").expect("existing output should write");
    let options = ImageOptions::new(ImageFileFormat::WebP);

    let error = ImageProcessor::process_file(&input_path, &output_path, &options, false)
        .expect_err("existing destination should be protected");
    assert!(matches!(error, ImageError::DestinationExists(_)));
    assert_eq!(fs::read(&output_path).expect("output should remain"), b"keep me");

    let info = ImageProcessor::process_file(&input_path, &output_path, &options, true)
        .expect("explicit overwrite should work");
    assert_eq!(info.output_format, ImageFileFormat::WebP);
    assert_eq!(detected_format(&fs::read(&output_path).expect("output should exist")), ImageFormat::WebP);

    let in_place_options = ImageOptions::new(ImageFileFormat::Png)
        .with_resize(ResizeOptions::exact(4, 2));
    let info = ImageProcessor::process_file(
        &input_path,
        &input_path,
        &in_place_options,
        true,
    )
    .expect("in-place processing should work");
    assert_eq!((info.output_width, info.output_height), (4, 2));
}

#[test]
fn path_api_validates_extension_before_writing() {
    let directory = tempfile::tempdir().expect("temporary directory should exist");
    let input_path = directory.path().join("input.png");
    let output_path = directory.path().join("output.jpg");
    fs::write(&input_path, encode_fixture(ImageFileFormat::Png, false))
        .expect("fixture should write");

    let error = ImageProcessor::process_file(
        &input_path,
        &output_path,
        &ImageOptions::new(ImageFileFormat::Png),
        false,
    )
    .expect_err("mismatched extension should fail");
    assert!(matches!(error, ImageError::OutputExtension { .. }));
    assert!(!output_path.exists());
}

#[test]
fn rejects_unsupported_or_malformed_input_without_creating_output() {
    let error = ImageProcessor::process_bytes(
        b"not an image",
        &ImageOptions::new(ImageFileFormat::Png),
    )
    .expect_err("invalid input should fail");
    assert!(matches!(error, ImageError::UnsupportedFormat));

    let directory = tempfile::tempdir().expect("temporary directory should exist");
    let input_path = directory.path().join("broken.png");
    let output_path = directory.path().join("output.png");
    fs::write(&input_path, b"not an image").expect("fixture should write");
    ImageProcessor::process_file(
        &input_path,
        &output_path,
        &ImageOptions::new(ImageFileFormat::Png),
        false,
    )
    .expect_err("invalid input should fail");
    assert!(!output_path.exists());
}

fn encode_fixture(format: ImageFileFormat, transparent: bool) -> Vec<u8> {
    let image = RgbaImage::from_fn(8, 4, |x, y| {
        let alpha = if transparent && x == 0 { 128 } else { 255 };
        Rgba([(x * 24) as u8, (y * 48) as u8, 160, alpha])
    });
    encode_rgba(&image, format)
}

fn encode_transparent_pixel() -> Vec<u8> {
    encode_rgba(
        &RgbaImage::from_pixel(1, 1, Rgba([255, 0, 0, 128])),
        ImageFileFormat::Png,
    )
}

fn encode_rgba(image: &RgbaImage, format: ImageFileFormat) -> Vec<u8> {
    let mut output = Vec::new();
    match format {
        ImageFileFormat::Jpeg => {
            let rgb = DynamicImage::ImageRgba8(image.clone()).to_rgb8();
            JpegEncoder::new_with_quality(&mut output, 95)
                .write_image(
                    rgb.as_raw(),
                    image.width(),
                    image.height(),
                    ExtendedColorType::Rgb8,
                )
                .expect("JPEG fixture should encode");
        }
        ImageFileFormat::Png => {
            PngEncoder::new(&mut output)
                .write_image(
                    image.as_raw(),
                    image.width(),
                    image.height(),
                    ExtendedColorType::Rgba8,
                )
                .expect("PNG fixture should encode");
        }
        ImageFileFormat::WebP => {
            WebPEncoder::new_lossless(&mut output)
                .write_image(
                    image.as_raw(),
                    image.width(),
                    image.height(),
                    ExtendedColorType::Rgba8,
                )
                .expect("WebP fixture should encode");
        }
    }
    output
}

fn detected_format(bytes: &[u8]) -> ImageFormat {
    image::guess_format(bytes).expect("format should be detectable")
}

fn image_format(format: ImageFileFormat) -> ImageFormat {
    match format {
        ImageFileFormat::Jpeg => ImageFormat::Jpeg,
        ImageFileFormat::Png => ImageFormat::Png,
        ImageFileFormat::WebP => ImageFormat::WebP,
    }
}

fn has_tag(metadata: &Metadata, predicate: impl Fn(&ExifTag) -> bool) -> bool {
    metadata.into_iter().any(predicate)
}
