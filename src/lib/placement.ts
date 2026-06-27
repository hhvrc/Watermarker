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

  let x: number;
  switch (horizontalOf(p.anchor)) {
    case 'Left':
      x = mx;
      break;
    case 'Right':
      x = imgW - w - mx;
      break;
    default:
      x = (imgW - w) * 0.5;
  }

  let y: number;
  switch (verticalOf(p.anchor)) {
    case 'Top':
      y = my;
      break;
    case 'Bottom':
      y = imgH - h - my;
      break;
    default:
      y = (imgH - h) * 0.5;
  }

  return { x, y, w, h };
}
