//! Resolution-independent watermark placement.
//!
//! A [`Placement`] describes *where* and *how big* a watermark sits on an image
//! using fractions of the image's own dimensions, never absolute pixels. The same
//! `Placement` therefore produces a visually consistent result across a batch of
//! differently-sized images and at any resolution (preview or full-res export).
//!
//! [`resolve`] is the single source of truth that turns a `Placement` plus a
//! concrete image size into a pixel-space [`Rect`].

use serde::{Deserialize, Serialize};

/// The 9-grid of placement presets. Row (vertical) is Top/Middle/Bottom,
/// column (horizontal) is Left/Center/Right; the dead-center cell is [`Anchor::Center`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Anchor {
    TopLeft,
    TopCenter,
    TopRight,
    MiddleLeft,
    Center,
    MiddleRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

/// Horizontal band of an [`Anchor`].
enum H {
    Left,
    Center,
    Right,
}

/// Vertical band of an [`Anchor`].
enum V {
    Top,
    Middle,
    Bottom,
}

impl Anchor {
    fn horizontal(self) -> H {
        use Anchor::*;
        match self {
            TopLeft | MiddleLeft | BottomLeft => H::Left,
            TopCenter | Center | BottomCenter => H::Center,
            TopRight | MiddleRight | BottomRight => H::Right,
        }
    }

    fn vertical(self) -> V {
        use Anchor::*;
        match self {
            TopLeft | TopCenter | TopRight => V::Top,
            MiddleLeft | Center | MiddleRight => V::Middle,
            BottomLeft | BottomCenter | BottomRight => V::Bottom,
        }
    }
}

/// A resolution-independent watermark placement.
///
/// `width_frac` is a fraction of image **width**. Both margins are fractions of
/// the image's **shorter side** so a single margin value yields the same pixel
/// gap on every edge regardless of the image's aspect ratio (uniform margins).
///
/// `rot_deg` carries a fixed default and can be wired to a UI control without a
/// schema change.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Placement {
    pub anchor: Anchor,
    /// Distance from the anchored vertical edge, as a fraction of the shorter side.
    pub margin_x_frac: f32,
    /// Distance from the anchored horizontal edge, as a fraction of the shorter side.
    pub margin_y_frac: f32,
    /// Drawn watermark width, as a fraction of image width.
    pub width_frac: f32,
    /// Rotation about the watermark center, in degrees (clockwise).
    pub rot_deg: f32,
    /// Watermark opacity in `0.0..=1.0`.
    pub opacity: f32,
}

impl Default for Placement {
    fn default() -> Self {
        Self {
            anchor: Anchor::BottomRight,
            margin_x_frac: 0.03,
            margin_y_frac: 0.03,
            width_frac: 0.20,
            rot_deg: 0.0,
            opacity: 0.35,
        }
    }
}

/// A pixel-space rectangle (top-left origin) describing the drawn watermark box,
/// before any rotation is applied.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Resolve a [`Placement`] against a concrete image size and watermark aspect ratio.
///
/// `wm_aspect` is the watermark's native `width / height`. The returned [`Rect`]
/// preserves that aspect: width comes from `width_frac`, height is derived so the
/// watermark is never stretched.
pub fn resolve(p: &Placement, img_w: u32, img_h: u32, wm_aspect: f32) -> Rect {
    let iw = img_w as f32;
    let ih = img_h as f32;

    // Clamp width to a sane range so a malformed placement (e.g. a hostile or
    // buggy frontend sending an enormous fraction) can't blow up the rasterized
    // watermark size. The UI keeps this in 0.02..=1.0, so legit values are intact.
    let w = p.width_frac.clamp(0.0, 4.0) * iw;
    let h = w / wm_aspect;

    // Margins resolve against the shorter side so the gap is uniform (the same
    // pixel distance on every edge), independent of the image's aspect ratio.
    let m_ref = iw.min(ih);
    let mx = p.margin_x_frac * m_ref;
    let my = p.margin_y_frac * m_ref;

    let x = match p.anchor.horizontal() {
        H::Left => mx,
        H::Center => (iw - w) * 0.5,
        H::Right => iw - w - mx,
    };
    let y = match p.anchor.vertical() {
        V::Top => my,
        V::Middle => (ih - h) * 0.5,
        V::Bottom => ih - h - my,
    };

    Rect { x, y, w, h }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a placement with the common test parameters and a given anchor.
    fn p(anchor: Anchor) -> Placement {
        Placement {
            anchor,
            margin_x_frac: 0.05,
            margin_y_frac: 0.05,
            width_frac: 0.20,
            rot_deg: 0.0,
            opacity: 1.0,
        }
    }

    // img 1000x800, width_frac 0.20 -> w=200; aspect 2.0 -> h=100.
    // margins use the shorter side (800): mx = my = 0.05*800 = 40.
    const IW: u32 = 1000;
    const IH: u32 = 800;
    const ASPECT: f32 = 2.0;

    fn approx(a: Rect, x: f32, y: f32) {
        assert!((a.w - 200.0).abs() < 1e-3, "w={}", a.w);
        assert!((a.h - 100.0).abs() < 1e-3, "h={}", a.h);
        assert!((a.x - x).abs() < 1e-3, "x={} expected {}", a.x, x);
        assert!((a.y - y).abs() < 1e-3, "y={} expected {}", a.y, y);
    }

    #[test]
    fn anchors_place_box_correctly() {
        // Horizontal: left=40, center=(1000-200)/2=400, right=1000-200-40=760.
        // Vertical:   top=40,  middle=(800-100)/2=350, bottom=800-100-40=660.
        approx(resolve(&p(Anchor::TopLeft), IW, IH, ASPECT), 40.0, 40.0);
        approx(resolve(&p(Anchor::TopCenter), IW, IH, ASPECT), 400.0, 40.0);
        approx(resolve(&p(Anchor::TopRight), IW, IH, ASPECT), 760.0, 40.0);
        approx(resolve(&p(Anchor::MiddleLeft), IW, IH, ASPECT), 40.0, 350.0);
        approx(resolve(&p(Anchor::Center), IW, IH, ASPECT), 400.0, 350.0);
        approx(
            resolve(&p(Anchor::MiddleRight), IW, IH, ASPECT),
            760.0,
            350.0,
        );
        approx(resolve(&p(Anchor::BottomLeft), IW, IH, ASPECT), 40.0, 660.0);
        approx(
            resolve(&p(Anchor::BottomCenter), IW, IH, ASPECT),
            400.0,
            660.0,
        );
        approx(
            resolve(&p(Anchor::BottomRight), IW, IH, ASPECT),
            760.0,
            660.0,
        );
    }

    #[test]
    fn aspect_is_preserved() {
        // Tall watermark (aspect 0.5) at fixed width_frac keeps height = w / aspect.
        let r = resolve(&p(Anchor::TopLeft), 1000, 1000, 0.5);
        assert!((r.w - 200.0).abs() < 1e-3);
        assert!((r.h - 400.0).abs() < 1e-3);
    }

    #[test]
    fn placement_is_resolution_independent() {
        // Same placement on a 2x larger image yields a 2x larger, same-position box.
        let small = resolve(&p(Anchor::BottomRight), 1000, 800, ASPECT);
        let large = resolve(&p(Anchor::BottomRight), 2000, 1600, ASPECT);
        assert!((large.x - small.x * 2.0).abs() < 1e-3);
        assert!((large.y - small.y * 2.0).abs() < 1e-3);
        assert!((large.w - small.w * 2.0).abs() < 1e-3);
        assert!((large.h - small.h * 2.0).abs() < 1e-3);
    }

    #[test]
    fn serde_roundtrips() {
        let original = Placement::default();
        let json = serde_json::to_string(&original).unwrap();
        let back: Placement = serde_json::from_str(&json).unwrap();
        assert_eq!(original, back);
    }
}
