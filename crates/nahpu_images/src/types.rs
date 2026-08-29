//! Public image processing options and result types.

use std::fmt;

/// Static image file formats supported for input and output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFileFormat {
    /// JPEG image (`.jpg` or `.jpeg`).
    Jpeg,
    /// Portable Network Graphics image (`.png`).
    Png,
    /// WebP image (`.webp`).
    WebP,
}

impl fmt::Display for ImageFileFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Jpeg => formatter.write_str("JPEG"),
            Self::Png => formatter.write_str("PNG"),
            Self::WebP => formatter.write_str("WebP"),
        }
    }
}

/// Strategy used to map a source image into requested dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeMode {
    /// Preserve aspect ratio and fit entirely inside the supplied bounds.
    Fit,
    /// Preserve aspect ratio, fill the supplied dimensions, and center-crop overflow.
    Fill,
    /// Resize independently to the supplied width and height.
    Exact,
}

/// Optional resize operation applied before encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResizeOptions {
    /// Requested width, or no horizontal bound for [`ResizeMode::Fit`].
    pub width: Option<u32>,
    /// Requested height, or no vertical bound for [`ResizeMode::Fit`].
    pub height: Option<u32>,
    /// Dimension mapping strategy.
    pub mode: ResizeMode,
    /// Whether a source smaller than the request may be enlarged.
    pub allow_upscale: bool,
}

impl ResizeOptions {
    /// Creates an aspect-preserving resize constrained by optional width and height bounds.
    pub const fn fit(width: Option<u32>, height: Option<u32>) -> Self {
        Self {
            width,
            height,
            mode: ResizeMode::Fit,
            allow_upscale: false,
        }
    }

    /// Creates an aspect-preserving resize that fills and center-crops to exact dimensions.
    pub const fn fill(width: u32, height: u32) -> Self {
        Self {
            width: Some(width),
            height: Some(height),
            mode: ResizeMode::Fill,
            allow_upscale: false,
        }
    }

    /// Creates a resize that stretches or compresses to exact dimensions.
    pub const fn exact(width: u32, height: u32) -> Self {
        Self {
            width: Some(width),
            height: Some(height),
            mode: ResizeMode::Exact,
            allow_upscale: false,
        }
    }

    /// Enables or disables enlargement of images smaller than the requested dimensions.
    pub const fn with_upscaling(mut self, allow_upscale: bool) -> Self {
        self.allow_upscale = allow_upscale;
        self
    }
}

/// An eight-bit RGB color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbColor {
    /// Red channel.
    pub red: u8,
    /// Green channel.
    pub green: u8,
    /// Blue channel.
    pub blue: u8,
}

impl RgbColor {
    /// Creates an RGB color from its channel values.
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }
}

impl Default for RgbColor {
    fn default() -> Self {
        Self::new(255, 255, 255)
    }
}

/// Options controlling conversion, resizing, and output encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageOptions {
    /// File format to encode.
    pub output_format: ImageFileFormat,
    /// Optional resize operation; `None` retains the oriented source dimensions.
    pub resize: Option<ResizeOptions>,
    /// JPEG quality in the inclusive range `1..=100`; ignored for PNG and WebP.
    pub jpeg_quality: u8,
    /// Background used when a transparent image is converted to JPEG.
    pub jpeg_background: RgbColor,
}

impl ImageOptions {
    /// Creates options for `output_format` with no resize and conservative encoding defaults.
    pub const fn new(output_format: ImageFileFormat) -> Self {
        Self {
            output_format,
            resize: None,
            jpeg_quality: 85,
            jpeg_background: RgbColor::new(255, 255, 255),
        }
    }

    /// Adds a resize operation.
    pub const fn with_resize(mut self, resize: ResizeOptions) -> Self {
        self.resize = Some(resize);
        self
    }

    /// Sets JPEG quality. Values are validated when JPEG output is requested.
    pub const fn with_jpeg_quality(mut self, quality: u8) -> Self {
        self.jpeg_quality = quality;
        self
    }

    /// Sets the background used to composite transparent pixels for JPEG output.
    pub const fn with_jpeg_background(mut self, background: RgbColor) -> Self {
        self.jpeg_background = background;
        self
    }
}

/// Metadata describing an image processing result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageInfo {
    /// Detected source format.
    pub source_format: ImageFileFormat,
    /// Source pixel width after applying EXIF orientation.
    pub source_width: u32,
    /// Source pixel height after applying EXIF orientation.
    pub source_height: u32,
    /// Encoded output format.
    pub output_format: ImageFileFormat,
    /// Encoded output width.
    pub output_width: u32,
    /// Encoded output height.
    pub output_height: u32,
    /// Encoded output size in bytes.
    pub output_bytes: u64,
    /// Whether the output dimensions differ from the oriented source dimensions.
    pub resized: bool,
    /// Whether source EXIF metadata was present and copied to the output.
    pub exif_preserved: bool,
}

/// Encoded bytes and their processing metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessedImage {
    /// Complete encoded image file.
    pub bytes: Vec<u8>,
    /// Metadata describing the conversion.
    pub info: ImageInfo,
}
