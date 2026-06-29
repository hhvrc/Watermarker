// Pure geometry for the editor canvas: coordinate mapping, hit-testing, and
// turning a freely-dragged watermark box back into an anchor + margins.
// No DOM dependencies, so it's unit-tested directly in placementMap.test.ts.

import type { Anchor, Placement } from './types';
import { anchorFrom, horizontalOf, verticalOf, type Rect } from './placement';

/** Minimal rectangle shape used for mapping (subset of DOMRect). */
export interface RectLike {
  left: number;
  top: number;
  width: number;
  height: number;
}

/**
 * Map a pointer's client coordinates to image-pixel coordinates, accounting for
 * the canvas being CSS-scaled to fit its container.
 */
export function canvasPointToImage(
  clientX: number,
  clientY: number,
  rect: RectLike,
  canvasW: number,
  canvasH: number,
): { x: number; y: number } {
  const x = ((clientX - rect.left) / rect.width) * canvasW;
  const y = ((clientY - rect.top) / rect.height) * canvasH;
  return { x, y };
}

export function pointInRect(px: number, py: number, r: Rect): boolean {
  return px >= r.x && px <= r.x + r.w && py >= r.y && py <= r.y + r.h;
}

function clamp01(v: number): number {
  return Math.max(0, Math.min(1, v));
}

/** Which third of `[0, size)` does `pos` fall in: 0, 1, or 2. */
function third(pos: number, size: number): 0 | 1 | 2 {
  const f = pos / size;
  if (f < 1 / 3) return 0;
  if (f < 2 / 3) return 1;
  return 2;
}

/**
 * Snap a freely-positioned watermark box back to the placement model: pick the
 * anchor whose region contains the box center, then express the offset from the
 * anchored edge(s) as margin fractions. Centered bands carry zero margin.
 */
export function snapToAnchor(
  imgW: number,
  imgH: number,
  box: Rect,
): { anchor: Anchor; margin_x_frac: number; margin_y_frac: number } {
  const cx = box.x + box.w / 2;
  const cy = box.y + box.h / 2;

  const col = third(cx, imgW); // 0 left, 1 center, 2 right
  const row = third(cy, imgH); // 0 top, 1 middle, 2 bottom

  const h = col === 0 ? 'Left' : col === 2 ? 'Right' : 'Center';
  const v = row === 0 ? 'Top' : row === 2 ? 'Bottom' : 'Middle';
  const anchor = anchorFrom(h, v);

  // Express margins relative to the shorter side so a dragged box round-trips
  // through `resolveRect` (which also uses the shorter side as the reference).
  const mRef = Math.min(imgW, imgH);
  let margin_x_frac = 0;
  if (h === 'Left') margin_x_frac = clamp01(box.x / mRef);
  else if (h === 'Right') margin_x_frac = clamp01((imgW - (box.x + box.w)) / mRef);

  let margin_y_frac = 0;
  if (v === 'Top') margin_y_frac = clamp01(box.y / mRef);
  else if (v === 'Bottom') margin_y_frac = clamp01((imgH - (box.y + box.h)) / mRef);

  return { anchor, margin_x_frac, margin_y_frac };
}

/**
 * Apply a pointer drag (in image pixels) to a placement: move the box by
 * (dx, dy) from `startBox`, then snap to the nearest of the 9 anchors. Only the
 * anchor changes; margins stay fixed at the slider values, so the watermark
 * jumps between discrete snap positions rather than landing anywhere in between.
 * Width/rotation/opacity are untouched.
 */
export function dragToPlacement(
  start: Placement,
  startBox: Rect,
  dx: number,
  dy: number,
  imgW: number,
  imgH: number,
): Placement {
  const moved: Rect = { x: startBox.x + dx, y: startBox.y + dy, w: startBox.w, h: startBox.h };
  const { anchor } = snapToAnchor(imgW, imgH, moved);
  return { ...start, anchor };
}

export function dist(ax: number, ay: number, bx: number, by: number): number {
  return Math.hypot(ax - bx, ay - by);
}

/**
 * The fixed image-space point a placement is anchored to: the corner/edge the
 * watermark is pinned against (with its margin). Scaling pivots about this point
 * so a resize drag is always measured relative to the anchor. Margins use the
 * shorter side, matching `resolveRect`.
 */
export function anchorPoint(
  p: Placement,
  imgW: number,
  imgH: number,
): { x: number; y: number } {
  const mRef = Math.min(imgW, imgH);
  const mx = p.margin_x_frac * mRef;
  const my = p.margin_y_frac * mRef;
  const h = horizontalOf(p.anchor);
  const v = verticalOf(p.anchor);
  const x = h === 'Left' ? mx : h === 'Right' ? imgW - mx : imgW / 2;
  const y = v === 'Top' ? my : v === 'Bottom' ? imgH - my : imgH / 2;
  return { x, y };
}

/**
 * Rotate `(px, py)` by `deg` degrees about center `(cx, cy)`. Used to map a
 * pointer into the watermark's local (unrotated) frame for hit-testing: pass
 * `-rot_deg` so a rotated box can be tested with the axis-aligned helpers.
 */
export function rotatePoint(
  px: number,
  py: number,
  cx: number,
  cy: number,
  deg: number,
): { x: number; y: number } {
  const rad = (deg * Math.PI) / 180;
  const cos = Math.cos(rad);
  const sin = Math.sin(rad);
  const dx = px - cx;
  const dy = py - cy;
  return { x: cx + dx * cos - dy * sin, y: cy + dx * sin + dy * cos };
}

/** The four corner points of a rect. */
export function corners(r: Rect): { x: number; y: number }[] {
  return [
    { x: r.x, y: r.y },
    { x: r.x + r.w, y: r.y },
    { x: r.x, y: r.y + r.h },
    { x: r.x + r.w, y: r.y + r.h },
  ];
}

/** Whether `(px, py)` is within `radius` of any corner of `r`. */
export function nearCorner(px: number, py: number, r: Rect, radius: number): boolean {
  return corners(r).some((c) => dist(px, py, c.x, c.y) <= radius);
}

export const MIN_WIDTH_FRAC = 0.02;
export const MAX_WIDTH_FRAC = 1.0;

/**
 * Scale a starting `width_frac` by the ratio of pointer distances from the box
 * center (`d1 / d0`), clamped to sane bounds. Keeps the watermark aspect (height
 * is always derived from width).
 */
export function resizeWidthFrac(
  startWidthFrac: number,
  d0: number,
  d1: number,
  min = MIN_WIDTH_FRAC,
  max = MAX_WIDTH_FRAC,
): number {
  if (d0 <= 0) return startWidthFrac;
  return Math.max(min, Math.min(max, startWidthFrac * (d1 / d0)));
}

/** Re-export so consumers can resolve without importing two modules. */
export { horizontalOf, verticalOf };
