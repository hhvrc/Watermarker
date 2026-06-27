//! Watermark sources and rasterization.
//!
//! A watermark can be a raster image (png/jpg/tiff/webp, decoded elsewhere into an
//! [`image::RgbaImage`]) or a vector SVG. Both are unified behind [`WatermarkSource`]
//! so the compositor can ask for the watermark *rasterized at the exact drawn size*.
//! For SVG this means re-rasterizing at the target pixel size, keeping it crisp at
//! full-resolution export instead of upscaling a small bitmap.

use std::sync::Arc;

use image::{imageops::FilterType, Rgba, RgbaImage};
use resvg::tiny_skia;
use resvg::usvg;

/// A watermark, either raster or vector.
pub enum WatermarkSource {
    /// A decoded raster image in straight (non-premultiplied) RGBA. Shared via
    /// `Arc` so cloning the source per export item is cheap.
    Raster(Arc<RgbaImage>),
    /// A parsed SVG document. Boxed because `usvg::Tree` is large (~400 bytes),
    /// which would otherwise bloat the whole enum (and every `Raster` value).
    Vector(Box<usvg::Tree>),
}

#[derive(Debug, thiserror::Error)]
pub enum WatermarkError {
    #[error("failed to parse SVG: {0}")]
    Svg(#[from] usvg::Error),
}

impl WatermarkSource {
    /// Parse an SVG document from raw bytes.
    ///
    /// The external-image href resolver is disabled so a crafted watermark SVG
    /// can't pull an arbitrary local file (e.g. `<image href="C:\...\private.png">`)
    /// into the rasterized output. Embedded `data:` URIs are still honored.
    pub fn from_svg_bytes(data: &[u8]) -> Result<Self, WatermarkError> {
        let opt = usvg::Options {
            image_href_resolver: usvg::ImageHrefResolver {
                resolve_data: usvg::ImageHrefResolver::default_data_resolver(),
                resolve_string: Box::new(|_href, _opts| None),
            },
            ..usvg::Options::default()
        };
        let tree = usvg::Tree::from_data(data, &opt)?;
        Ok(WatermarkSource::Vector(Box::new(tree)))
    }

    /// The content size in pixels (raster: trimmed dimensions; vector: content
    /// bounding-box size). This is the tight box used for placement.
    pub fn content_size(&self) -> (u32, u32) {
        match self {
            WatermarkSource::Raster(img) => img.dimensions(),
            WatermarkSource::Vector(tree) => {
                let (_, _, w, h) = svg_content_rect(tree);
                (w.round().max(1.0) as u32, h.round().max(1.0) as u32)
            }
        }
    }

    /// The watermark's content aspect ratio (`width / height`).
    pub fn aspect(&self) -> f32 {
        match self {
            WatermarkSource::Raster(img) => {
                let (w, h) = img.dimensions();
                w as f32 / h as f32
            }
            WatermarkSource::Vector(tree) => {
                let (_, _, w, h) = svg_content_rect(tree);
                w / h
            }
        }
    }

    /// Rasterize the watermark at exactly `w` x `h` pixels in straight RGBA.
    ///
    /// Raster sources are resampled (Lanczos3); vector sources are re-rendered at
    /// the requested size for sharp edges at any scale.
    pub fn rasterize(&self, w: u32, h: u32) -> RgbaImage {
        let w = w.max(1);
        let h = h.max(1);
        match self {
            WatermarkSource::Raster(img) => imageops_resize(img.as_ref(), w, h),
            WatermarkSource::Vector(tree) => rasterize_svg(tree, w, h),
        }
    }
}

fn imageops_resize(img: &RgbaImage, w: u32, h: u32) -> RgbaImage {
    image::imageops::resize(img, w, h, FilterType::Lanczos3)
}

fn rasterize_svg(tree: &usvg::Tree, w: u32, h: u32) -> RgbaImage {
    // `Pixmap::new` returns `None` for degenerate/overflowing dimensions; fall back
    // to a 1x1 transparent stamp instead of panicking on pathological placement.
    let Some(mut pixmap) = tiny_skia::Pixmap::new(w, h) else {
        return RgbaImage::new(1, 1);
    };
    // Map the SVG's tight content bounding box onto the target pixmap, so any
    // transparent padding around the artwork is trimmed.
    let (bx, by, bw, bh) = svg_content_rect(tree);
    let sx = w as f32 / bw;
    let sy = h as f32 / bh;
    let transform = tiny_skia::Transform::from_row(sx, 0.0, 0.0, sy, -bx * sx, -by * sy);
    resvg::render(tree, transform, &mut pixmap.as_mut());
    pixmap_to_rgba(&pixmap)
}

/// The tight `(x, y, w, h)` content bounding box of an SVG, falling back to the
/// full canvas size if no bounding box is available.
fn svg_content_rect(tree: &usvg::Tree) -> (f32, f32, f32, f32) {
    let bbox = tree.root().abs_bounding_box();
    let (w, h) = (bbox.width(), bbox.height());
    if w > 0.0 && h > 0.0 {
        (bbox.x(), bbox.y(), w, h)
    } else {
        let s = tree.size();
        (0.0, 0.0, s.width(), s.height())
    }
}

/// Crop fully-transparent borders from a raster image, returning the tight content.
/// A fully-transparent image is returned unchanged.
pub fn trim_transparent(img: &RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (w, h, 0u32, 0u32);
    let mut found = false;
    for y in 0..h {
        for x in 0..w {
            if img.get_pixel(x, y)[3] > 0 {
                found = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    if !found {
        return img.clone();
    }
    image::imageops::crop_imm(img, min_x, min_y, max_x - min_x + 1, max_y - min_y + 1).to_image()
}

/// Convert a tiny-skia premultiplied pixmap into a straight-alpha [`RgbaImage`].
fn pixmap_to_rgba(pixmap: &tiny_skia::Pixmap) -> RgbaImage {
    let (w, h) = (pixmap.width(), pixmap.height());
    let mut out = RgbaImage::new(w, h);
    for (i, px) in pixmap.pixels().iter().enumerate() {
        let x = (i as u32) % w;
        let y = (i as u32) / w;
        let c = px.demultiply();
        out.put_pixel(x, y, Rgba([c.red(), c.green(), c.blue(), c.alpha()]));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED_SQUARE_SVG: &[u8] =
        br#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="10"><rect width="20" height="10" fill="rgb(255,0,0)"/></svg>"#;

    #[test]
    fn raster_aspect() {
        let img = RgbaImage::new(40, 20);
        let wm = WatermarkSource::Raster(Arc::new(img));
        assert!((wm.aspect() - 2.0).abs() < 1e-3);
    }

    #[test]
    fn trim_removes_transparent_border() {
        // 20x20 fully transparent with a solid 4x6 opaque block at (5,7).
        let mut img = RgbaImage::new(20, 20);
        for y in 7..13 {
            for x in 5..9 {
                img.put_pixel(x, y, Rgba([10, 20, 30, 255]));
            }
        }
        let trimmed = trim_transparent(&img);
        assert_eq!(trimmed.dimensions(), (4, 6));
        assert_eq!(trimmed.get_pixel(0, 0).0, [10, 20, 30, 255]);
    }

    #[test]
    fn trim_keeps_fully_opaque_image() {
        let img = RgbaImage::from_pixel(8, 5, Rgba([1, 2, 3, 255]));
        assert_eq!(trim_transparent(&img).dimensions(), (8, 5));
    }

    #[test]
    fn raster_rasterize_resizes() {
        let img = RgbaImage::from_pixel(10, 10, Rgba([0, 128, 0, 255]));
        let wm = WatermarkSource::Raster(Arc::new(img));
        let out = wm.rasterize(40, 40);
        assert_eq!(out.dimensions(), (40, 40));
        // A solid color survives resampling.
        let p = out.get_pixel(20, 20).0;
        assert!(p[1] > 100 && p[3] > 200, "got {:?}", p);
    }

    #[test]
    fn svg_parses_and_reports_aspect() {
        let wm = WatermarkSource::from_svg_bytes(RED_SQUARE_SVG).unwrap();
        assert!((wm.aspect() - 2.0).abs() < 1e-2, "aspect {}", wm.aspect());
    }

    #[test]
    fn svg_rasterizes_to_requested_size() {
        let wm = WatermarkSource::from_svg_bytes(RED_SQUARE_SVG).unwrap();
        let out = wm.rasterize(40, 20);
        assert_eq!(out.dimensions(), (40, 20));
        // Center pixel should be opaque red.
        let p = out.get_pixel(20, 10).0;
        assert!(
            p[0] > 200 && p[1] < 60 && p[2] < 60 && p[3] > 200,
            "got {:?}",
            p
        );
    }
}
