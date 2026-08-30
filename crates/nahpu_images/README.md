# NAHPU Images

`nahpu_images` provides static JPEG, PNG, and WebP conversion and high-quality resizing for
NAHPU. It supports in-memory processing and safe path-based output, preserves EXIF metadata in
JPEG and PNG output, and normalizes camera orientation before transforming pixels. WebP output
omits EXIF because reinserting it into a lossless WebP container can make the image invalid.

```rust
use nahpu_images::{ImageFileFormat, ImageOptions, ImageProcessor, ResizeOptions};

let options = ImageOptions::new(ImageFileFormat::WebP)
    .with_resize(ResizeOptions::fit(Some(1600), Some(1600)));
let result = ImageProcessor::process_file("photo.jpg", "photo.webp", &options, false)?;
# Ok::<(), nahpu_images::ImageError>(())
```

WebP and PNG output are lossless. JPEG output uses quality 85 by default and allows a configurable
quality and background color for transparent source pixels. Animated images are intentionally not
supported.
