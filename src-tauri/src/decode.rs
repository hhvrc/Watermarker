//! Decoding of supported source images.
//!
//! Supported inputs for v1: png, jpg/jpeg, tiff/tif, webp. (mp4 is planned: a future
//! frame-source would decode to the same [`image::RgbaImage`] and feed the same
//! compositor.)

use std::io::{BufRead, Seek};
use std::path::Path;

use image::{DynamicImage, ImageDecoder, ImageReader, RgbaImage};

/// File extensions accepted as image inputs (lowercase, without dot).
pub const SUPPORTED_INPUTS: &[&str] = &["png", "jpg", "jpeg", "tiff", "tif", "webp"];

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
}

/// Decode guards against malicious inputs (decompression bombs / absurd
/// dimensions): cap each side and the total allocation so a tiny crafted file
/// can't make the decoder allocate gigabytes and OOM-kill the app.
const MAX_DECODE_DIM: u32 = 30_000;
const MAX_DECODE_ALLOC: u64 = 2 * 1024 * 1024 * 1024; // 2 GiB

fn decode_limits() -> image::Limits {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_DECODE_DIM);
    limits.max_image_height = Some(MAX_DECODE_DIM);
    limits.max_alloc = Some(MAX_DECODE_ALLOC);
    limits
}

/// Whether `path`'s extension is a supported image input.
pub fn is_supported_input(path: &Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => SUPPORTED_INPUTS.contains(&ext.to_ascii_lowercase().as_str()),
        None => false,
    }
}

/// Decode an image file into straight RGBA at full resolution, baking EXIF
/// orientation into the pixels so the result is always upright.
///
/// Because orientation is applied to the pixels here, any preserved EXIF block
/// still carries the original orientation tag; PNG viewers overwhelmingly ignore
/// EXIF orientation, so this stays visually correct.
pub fn load_rgba(path: &Path) -> Result<RgbaImage, DecodeError> {
    decode_reader(image::ImageReader::open(path)?.with_guessed_format()?)
}

/// Decode an in-memory image (e.g. a drag-dropped file or raw bitmap) into
/// straight RGBA, applying the same orientation handling as [`load_rgba`].
pub fn load_rgba_from_bytes(bytes: &[u8]) -> Result<RgbaImage, DecodeError> {
    let reader = ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()?;
    decode_reader(reader)
}

fn decode_reader<R: BufRead + Seek>(mut reader: ImageReader<R>) -> Result<RgbaImage, DecodeError> {
    reader.limits(decode_limits());
    let mut decoder = reader.into_decoder()?;
    // Read orientation before consuming the decoder; default to identity if absent.
    let orientation = decoder
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut img = DynamicImage::from_decoder(decoder)?;
    img.apply_orientation(orientation);
    Ok(img.to_rgba8())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn recognizes_supported_extensions() {
        for ext in ["png", "PNG", "jpg", "Jpeg", "tiff", "tif", "webp"] {
            let p = PathBuf::from(format!("a.{ext}"));
            assert!(is_supported_input(&p), "{ext} should be supported");
        }
    }

    #[test]
    fn rejects_unsupported_extensions() {
        for name in ["a.gif", "a.bmp", "a.mp4", "noext", "a.svg"] {
            assert!(
                !is_supported_input(&PathBuf::from(name)),
                "{name} should be rejected"
            );
        }
    }

    #[test]
    fn decodes_a_normal_image_within_limits() {
        // A small valid PNG must still decode after the decode-limit guard.
        let mut bytes = Vec::new();
        let img = image::RgbaImage::from_pixel(4, 3, image::Rgba([10, 20, 30, 255]));
        image::DynamicImage::ImageRgba8(img)
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .unwrap();
        let out = load_rgba_from_bytes(&bytes).unwrap();
        assert_eq!(out.dimensions(), (4, 3));
    }
}
