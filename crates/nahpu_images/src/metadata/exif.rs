//! EXIF extraction, normalization, and reinsertion.

use little_exif::{exif_tag::ExifTag, filetype::FileExtension, metadata::Metadata};

use crate::{ImageError, ImageFileFormat};

pub(crate) fn read(input: &[u8], format: ImageFileFormat) -> Result<Metadata, ImageError> {
    Metadata::new_from_vec(&input.to_vec(), file_extension(format)).map_err(ImageError::Metadata)
}

pub(crate) fn normalize(metadata: &mut Metadata, width: u32, height: u32) {
    metadata.set_tag(ExifTag::Orientation(vec![1]));
    metadata.set_tag(ExifTag::ImageWidth(vec![width]));
    metadata.set_tag(ExifTag::ImageHeight(vec![height]));
    metadata.set_tag(ExifTag::ExifImageWidth(vec![width]));
    metadata.set_tag(ExifTag::ExifImageHeight(vec![height]));
}

pub(crate) fn write(
    metadata: &Metadata,
    output: &mut Vec<u8>,
    format: ImageFileFormat,
) -> Result<(), ImageError> {
    metadata
        .write_to_vec(output, file_extension(format))
        .map_err(ImageError::Metadata)
}

fn file_extension(format: ImageFileFormat) -> FileExtension {
    match format {
        ImageFileFormat::Jpeg => FileExtension::JPEG,
        ImageFileFormat::Png => FileExtension::PNG {
            as_zTXt_chunk: false,
        },
        ImageFileFormat::WebP => FileExtension::WEBP,
    }
}
