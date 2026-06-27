//! The shared compositing core.
//!
//! [`composite`] stamps a watermark onto a base image according to a [`Placement`].
//! This is the single rendering function used by *both* the interactive preview and
//! the final full-resolution export, guaranteeing what-you-see-is-what-you-get.

use image::RgbaImage;

use crate::placement::{resolve, Placement};
use crate::watermark::WatermarkSource;

/// Composite `wm` onto a clone of `base` using `p`, returning the result.
pub fn composite(base: &RgbaImage, wm: &WatermarkSource, p: &Placement) -> RgbaImage {
    let (iw, ih) = base.dimensions();
    let rect = resolve(p, iw, ih, wm.aspect());

    let dw = rect.w.round().max(1.0) as u32;
    let dh = rect.h.round().max(1.0) as u32;
    let stamp = wm.rasterize(dw, dh);

    let mut out = base.clone();
    blend_stamp(&mut out, &stamp, rect.x, rect.y, p.opacity, p.rot_deg);
    out
}

/// Alpha-blend `stamp` onto `base` at top-left `(x, y)`, rotated `rot_deg` degrees
/// about the stamp center, scaled by `opacity`.
fn blend_stamp(
    base: &mut RgbaImage,
    stamp: &RgbaImage,
    x: f32,
    y: f32,
    opacity: f32,
    rot_deg: f32,
) {
    let opacity = opacity.clamp(0.0, 1.0);
    if opacity <= 0.0 {
        return;
    }

    if rot_deg == 0.0 {
        blend_axis_aligned(base, stamp, x.round() as i32, y.round() as i32, opacity);
    } else {
        blend_rotated(base, stamp, x, y, opacity, rot_deg);
    }
}

/// Fast path: no rotation, integer offset, exact 1:1 pixel blend.
fn blend_axis_aligned(base: &mut RgbaImage, stamp: &RgbaImage, ox: i32, oy: i32, opacity: f32) {
    let (bw, bh) = base.dimensions();
    let (sw, sh) = stamp.dimensions();
    for sy in 0..sh {
        for sx in 0..sw {
            let bx = ox + sx as i32;
            let by = oy + sy as i32;
            if bx < 0 || by < 0 || bx >= bw as i32 || by >= bh as i32 {
                continue;
            }
            let s = stamp.get_pixel(sx, sy).0;
            blend_pixel(
                base,
                bx as u32,
                by as u32,
                [s[0], s[1], s[2], s[3]],
                opacity,
            );
        }
    }
}

/// General path: inverse-map each destination pixel through the rotation and
/// bilinearly sample the stamp.
fn blend_rotated(
    base: &mut RgbaImage,
    stamp: &RgbaImage,
    x: f32,
    y: f32,
    opacity: f32,
    rot_deg: f32,
) {
    let (bw, bh) = base.dimensions();
    let (sw, sh) = (stamp.width() as f32, stamp.height() as f32);

    let cx = x + sw * 0.5;
    let cy = y + sh * 0.5;
    let (sin, cos) = rot_deg.to_radians().sin_cos();

    // Destination bounding box of the rotated stamp.
    let corners = [(x, y), (x + sw, y), (x, y + sh), (x + sw, y + sh)];
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for (px, py) in corners {
        let (dx, dy) = (px - cx, py - cy);
        let rx = cx + dx * cos - dy * sin;
        let ry = cy + dx * sin + dy * cos;
        min_x = min_x.min(rx);
        min_y = min_y.min(ry);
        max_x = max_x.max(rx);
        max_y = max_y.max(ry);
    }

    let x0 = min_x.floor().max(0.0) as u32;
    let y0 = min_y.floor().max(0.0) as u32;
    let x1 = (max_x.ceil() as i64).clamp(0, bw as i64) as u32;
    let y1 = (max_y.ceil() as i64).clamp(0, bh as i64) as u32;

    for by in y0..y1 {
        for bx in x0..x1 {
            let px = bx as f32 + 0.5;
            let py = by as f32 + 0.5;
            // Inverse rotation R(-theta) maps a base point back into stamp space.
            let (dx, dy) = (px - cx, py - cy);
            let lx = (cos * dx + sin * dy) + sw * 0.5;
            let ly = (-sin * dx + cos * dy) + sh * 0.5;
            if lx < 0.0 || ly < 0.0 || lx >= sw || ly >= sh {
                continue;
            }
            let s = bilinear(stamp, lx, ly);
            blend_pixel(base, bx, by, s, opacity);
        }
    }
}

/// Source-over blend of a straight-RGBA `src` (scaled by `opacity`) onto `base[x,y]`.
fn blend_pixel(base: &mut RgbaImage, x: u32, y: u32, src: [u8; 4], opacity: f32) {
    let a = (src[3] as f32 / 255.0) * opacity;
    if a <= 0.0 {
        return;
    }
    let p = base.get_pixel_mut(x, y);
    for c in 0..3 {
        let s = src[c] as f32;
        let d = p[c] as f32;
        p[c] = (s * a + d * (1.0 - a)).round().clamp(0.0, 255.0) as u8;
    }
    let da = p[3] as f32 / 255.0;
    let out_a = a + da * (1.0 - a);
    p[3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
}

/// Bilinear sample of `img` at floating coordinate `(fx, fy)` (pixel centers at
/// `n + 0.5`), clamping at the edges. Returns straight RGBA as `u8`.
fn bilinear(img: &RgbaImage, fx: f32, fy: f32) -> [u8; 4] {
    let (w, h) = img.dimensions();
    let x = fx - 0.5;
    let y = fy - 0.5;
    let x0 = x.floor();
    let y0 = y.floor();
    let tx = x - x0;
    let ty = y - y0;
    let xi = x0 as i64;
    let yi = y0 as i64;

    let get = |ix: i64, iy: i64| -> [f32; 4] {
        let cx = ix.clamp(0, w as i64 - 1) as u32;
        let cy = iy.clamp(0, h as i64 - 1) as u32;
        let p = img.get_pixel(cx, cy).0;
        [p[0] as f32, p[1] as f32, p[2] as f32, p[3] as f32]
    };

    let p00 = get(xi, yi);
    let p10 = get(xi + 1, yi);
    let p01 = get(xi, yi + 1);
    let p11 = get(xi + 1, yi + 1);

    let mut out = [0u8; 4];
    for c in 0..4 {
        let top = p00[c] * (1.0 - tx) + p10[c] * tx;
        let bot = p01[c] * (1.0 - tx) + p11[c] * tx;
        out[c] = (top * (1.0 - ty) + bot * ty).round().clamp(0.0, 255.0) as u8;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::placement::Anchor;
    use image::Rgba;
    use std::sync::Arc;

    fn placement(anchor: Anchor, width_frac: f32, opacity: f32, rot_deg: f32) -> Placement {
        Placement {
            anchor,
            margin_x_frac: 0.0,
            margin_y_frac: 0.0,
            width_frac,
            rot_deg,
            opacity,
        }
    }

    #[test]
    fn opacity_blends_halfway() {
        // White base, red 10x10 watermark at top-left, 50% opacity, no rotation.
        let base = RgbaImage::from_pixel(100, 100, Rgba([255, 255, 255, 255]));
        let wm = WatermarkSource::Raster(Arc::new(RgbaImage::from_pixel(
            10,
            10,
            Rgba([255, 0, 0, 255]),
        )));
        let p = placement(Anchor::TopLeft, 0.10, 0.5, 0.0);

        let out = composite(&base, &wm, &p);

        // Inside the stamp: red*0.5 + white*0.5 = (255, 128, 128).
        let inside = out.get_pixel(0, 0).0;
        assert_eq!(inside[0], 255);
        assert_eq!(inside[1], 128);
        assert_eq!(inside[2], 128);
        assert_eq!(inside[3], 255);

        // Outside the stamp: untouched white.
        assert_eq!(out.get_pixel(50, 50).0, [255, 255, 255, 255]);
    }

    #[test]
    fn full_opacity_replaces_on_transparent_base() {
        let base = RgbaImage::from_pixel(100, 100, Rgba([0, 0, 0, 0]));
        let wm = WatermarkSource::Raster(Arc::new(RgbaImage::from_pixel(
            20,
            20,
            Rgba([0, 200, 0, 255]),
        )));
        // Centered 20x20 square -> occupies [40,60) in both axes.
        let p = placement(Anchor::Center, 0.20, 1.0, 0.0);

        let out = composite(&base, &wm, &p);

        assert_eq!(out.get_pixel(50, 50).0, [0, 200, 0, 255]);
        // Far outside stays fully transparent.
        assert_eq!(out.get_pixel(5, 5).0, [0, 0, 0, 0]);
    }

    #[test]
    fn rotation_keeps_symmetric_stamp_covering_center() {
        // A solid square rotated 90 degrees about its center still covers the center.
        let base = RgbaImage::from_pixel(100, 100, Rgba([0, 0, 0, 0]));
        let wm = WatermarkSource::Raster(Arc::new(RgbaImage::from_pixel(
            20,
            20,
            Rgba([0, 200, 0, 255]),
        )));
        let p = placement(Anchor::Center, 0.20, 1.0, 90.0);

        let out = composite(&base, &wm, &p);

        let center = out.get_pixel(50, 50).0;
        assert!(center[1] > 150 && center[3] > 200, "center {:?}", center);
        // A corner well outside the rotated square remains transparent.
        assert_eq!(out.get_pixel(2, 2).0, [0, 0, 0, 0]);
    }
}
