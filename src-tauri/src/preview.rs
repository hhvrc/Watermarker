//! Downscaling and encoding of images for display in the frontend.
//!
//! The frontend never receives full-resolution pixels. Source images are decoded
//! and composited at full resolution in Rust, then downscaled to fit within a
//! [`MAX_PREVIEW_DIM`] square and PNG-encoded. The bytes travel to the webview as a
//! raw IPC response, which the frontend wraps in a `Blob` URL for display.

use std::io::Cursor;

use image::{ImageEncoder, RgbaImage};

/// Maximum size, in pixels, of any preview dimension. The image is downscaled to
/// fit within a `MAX_PREVIEW_DIM` x `MAX_PREVIEW_DIM` square, preserving aspect
/// ratio (so the longer side becomes at most this many pixels). Adjustable.
pub const MAX_PREVIEW_DIM: u32 = 2000;

/// Downscale `img` to fit within `max_w` x `max_h`, preserving aspect ratio.
/// Images already within bounds are returned unchanged (no upscaling).
pub fn downscale_to_fit(img: &RgbaImage, max_w: u32, max_h: u32) -> RgbaImage {
    let (w, h) = img.dimensions();
    if w <= max_w && h <= max_h {
        return img.clone();
    }
    let scale = (max_w as f32 / w as f32).min(max_h as f32 / h as f32);
    let nw = ((w as f32 * scale).round() as u32).max(1);
    let nh = ((h as f32 * scale).round() as u32).max(1);
    image::imageops::resize(img, nw, nh, image::imageops::FilterType::Triangle)
}

/// Downscale so the longer side is at most [`MAX_PREVIEW_DIM`], preserving aspect.
pub fn downscale_for_display(img: &RgbaImage) -> RgbaImage {
    downscale_to_fit(img, MAX_PREVIEW_DIM, MAX_PREVIEW_DIM)
}

/// Encode an [`RgbaImage`] to PNG bytes.
pub fn encode_png(img: &RgbaImage) -> Result<Vec<u8>, image::ImageError> {
    let mut buf = Cursor::new(Vec::new());
    image::codecs::png::PngEncoder::new(&mut buf).write_image(
        img.as_raw(),
        img.width(),
        img.height(),
        image::ExtendedColorType::Rgba8,
    )?;
    Ok(buf.into_inner())
}

/// Encode an image to the preview wire format (PNG).
pub fn encode_preview(img: &RgbaImage) -> Result<Vec<u8>, image::ImageError> {
    encode_png(img)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downscale_fits_within_bounds_and_keeps_aspect() {
        let img = RgbaImage::new(4000, 2000);
        let out = downscale_to_fit(&img, 1920, 1080);
        let (w, h) = out.dimensions();
        assert!(w <= 1920 && h <= 1080, "got {w}x{h}");
        // 4000x2000 (2:1) limited by width -> 1920x960.
        assert_eq!((w, h), (1920, 960));
    }

    #[test]
    fn small_images_are_not_upscaled() {
        let img = RgbaImage::new(640, 480);
        let out = downscale_to_fit(&img, 1920, 1080);
        assert_eq!(out.dimensions(), (640, 480));
    }

    #[test]
    fn display_caps_longer_side_to_max_dim() {
        // 5000x2500 -> longer side capped to 2000 -> 2000x1000.
        let img = RgbaImage::new(5000, 2500);
        let out = downscale_for_display(&img);
        let (w, h) = out.dimensions();
        assert!(w <= MAX_PREVIEW_DIM && h <= MAX_PREVIEW_DIM, "got {w}x{h}");
        assert_eq!((w, h), (2000, 1000));
    }

    #[test]
    fn height_limited_downscale() {
        let img = RgbaImage::new(2000, 4000);
        let out = downscale_to_fit(&img, 1920, 1080);
        let (w, h) = out.dimensions();
        assert!(w <= 1920 && h <= 1080, "got {w}x{h}");
        // Limited by height: scale = 1080/4000 = 0.27 -> 540x1080.
        assert_eq!((w, h), (540, 1080));
    }

    #[test]
    fn encode_preview_produces_decodable_png() {
        let img = RgbaImage::from_pixel(16, 16, image::Rgba([10, 20, 30, 255]));
        let bytes = encode_preview(&img).unwrap();
        let decoded = image::load_from_memory(&bytes).unwrap().to_rgba8();
        assert_eq!(decoded.dimensions(), (16, 16));
        assert_eq!(decoded.get_pixel(0, 0).0, [10, 20, 30, 255]);
    }
}
