// Placement helpers. `resolveRect` mirrors Rust `placement::resolve` so the
// Canvas2D live preview matches the Rust compositor exactly.

import type { Anchor, Placement } from './types';

export const ANCHORS: Anchor[] = [
  'TopLeft',
  'TopCenter',
  'TopRight',
  'MiddleLeft',
  'Center',
  'MiddleRight',
  'BottomLeft',
  'BottomCenter',
  'BottomRight',
];

export const ANCHOR_LABELS: Record<Anchor, string> = {
  TopLeft: 'Top left',
  TopCenter: 'Top center',
  TopRight: 'Top right',
  MiddleLeft: 'Middle left',
  Center: 'Center',
  MiddleRight: 'Middle right',
  BottomLeft: 'Bottom left',
  BottomCenter: 'Bottom center',
  BottomRight: 'Bottom right',
};

/** Default placement applied to the first image; carried forward thereafter. */
export const DEFAULT_PLACEMENT: Placement = {
  anchor: 'BottomRight',
  margin_x_frac: 0.03,
  margin_y_frac: 0.03,
  width_frac: 0.2,
  rot_deg: 0,
  opacity: 0.35,
};

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

/**
 * The largest `width_frac` at which the watermark's bounding box — after its
 * rotation — still fits inside the image, bounded by whichever dimension it
 * reaches first. Lets the Size control mean "% of best fit" (1.0 = touching the
 * binding edge) instead of "% of image width", so a tall or rotated watermark
 * can't be scaled past the top/bottom. Returns 1 when inputs are unknown.
 */
export function maxFitWidthFrac(
  imgW: number,
  imgH: number,
  wmAspect: number,
  rotDeg: number,
): number {
  if (!imgW || !imgH || !wmAspect) return 1;
  const rad = (rotDeg * Math.PI) / 180;
  const c = Math.abs(Math.cos(rad));
  const s = Math.abs(Math.sin(rad));
  // Rotated bbox width/height per unit width_frac, divided out by image size.
  const fitByWidth = 1 / (c + s / wmAspect);
  const fitByHeight = imgH / imgW / (s + c / wmAspect);
  return Math.min(fitByWidth, fitByHeight);
}

type H = 'Left' | 'Center' | 'Right';
type V = 'Top' | 'Middle' | 'Bottom';

export function horizontalOf(anchor: Anchor): H {
  if (anchor === 'TopLeft' || anchor === 'MiddleLeft' || anchor === 'BottomLeft') return 'Left';
  if (anchor === 'TopRight' || anchor === 'MiddleRight' || anchor === 'BottomRight') return 'Right';
  return 'Center';
}

export function verticalOf(anchor: Anchor): V {
  if (anchor === 'TopLeft' || anchor === 'TopCenter' || anchor === 'TopRight') return 'Top';
  if (anchor === 'BottomLeft' || anchor === 'BottomCenter' || anchor === 'BottomRight')
    return 'Bottom';
  return 'Middle';
}

export function anchorFrom(h: H, v: V): Anchor {
  const map: Record<string, Anchor> = {
    'Top|Left': 'TopLeft',
    'Top|Center': 'TopCenter',
    'Top|Right': 'TopRight',
    'Middle|Left': 'MiddleLeft',
    'Middle|Center': 'Center',
    'Middle|Right': 'MiddleRight',
    'Bottom|Left': 'BottomLeft',
    'Bottom|Center': 'BottomCenter',
    'Bottom|Right': 'BottomRight',
  };
  return map[`${v}|${h}`];
}

/**
 * Resolve a placement against an image size and watermark aspect ratio.
 * Returns the unrotated watermark box in image-pixel space.
 */
export function resolveRect(p: Placement, imgW: number, imgH: number, wmAspect: number): Rect {
  const w = p.width_frac * imgW;
  const h = w / wmAspect;
  // Margins resolve against the shorter side so the gap is uniform on every
  // edge, independent of the image's aspect ratio. Mirrors Rust `resolve`.
  const mRef = Math.min(imgW, imgH);
  const mx = p.margin_x_frac * mRef;
  const my = p.margin_y_frac * mRef;

  // Half-extents of the watermark's bounding box *after* rotation. Anchoring
  // against these keeps the rotated watermark pinned in its corner at any angle,
  // so scaling grows it from the anchored corner. Mirrors Rust `resolve`.
  const rad = (p.rot_deg * Math.PI) / 180;
  const cos = Math.abs(Math.cos(rad));
  const sin = Math.abs(Math.sin(rad));
  const hw = (w * cos + h * sin) * 0.5;
  const hh = (w * sin + h * cos) * 0.5;

  // Center of the (unrotated) box; the watermark rotates about this point.
  let cx: number;
  switch (horizontalOf(p.anchor)) {
    case 'Left':
      cx = mx + hw;
      break;
    case 'Right':
      cx = imgW - mx - hw;
      break;
    default:
      cx = imgW * 0.5;
  }

  let cy: number;
  switch (verticalOf(p.anchor)) {
    case 'Top':
      cy = my + hh;
      break;
    case 'Bottom':
      cy = imgH - my - hh;
      break;
    default:
      cy = imgH * 0.5;
  }

  return { x: cx - w / 2, y: cy - h / 2, w, h };
}
