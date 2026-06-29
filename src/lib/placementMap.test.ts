import { describe, it, expect } from 'vitest';
import { resolveRect, DEFAULT_PLACEMENT, maxFitWidthFrac } from './placement';
import {
  canvasPointToImage,
  pointInRect,
  snapToAnchor,
  dragToPlacement,
  nearCorner,
  resizeWidthFrac,
  rotatePoint,
  anchorPoint,
} from './placementMap';
import type { Placement } from './types';

const base: Placement = {
  anchor: 'BottomRight',
  margin_x_frac: 0.05,
  margin_y_frac: 0.05,
  width_frac: 0.2,
  rot_deg: 0,
  opacity: 1,
};

describe('rotatePoint', () => {
  it('rotates 90 degrees about a center', () => {
    // A point directly right of center maps to directly below it (+y is down).
    const p = rotatePoint(20, 10, 10, 10, 90);
    expect(p.x).toBeCloseTo(10);
    expect(p.y).toBeCloseTo(20);
  });

  it('round-trips with the inverse angle', () => {
    const fwd = rotatePoint(35, 12, 20, 20, 37);
    const back = rotatePoint(fwd.x, fwd.y, 20, 20, -37);
    expect(back.x).toBeCloseTo(35);
    expect(back.y).toBeCloseTo(12);
  });

  it('inverse-rotating a rotated corner lands back inside the axis-aligned box', () => {
    // Box [0,100]^2 rotated 45° about its center; a drawn corner inverse-rotated
    // returns to the original axis-aligned corner, so hit-tests still match.
    const r = { x: 0, y: 0, w: 100, h: 100 };
    const cx = 50;
    const cy = 50;
    const drawn = rotatePoint(r.x, r.y, cx, cy, 45); // where the TL handle renders
    const local = rotatePoint(drawn.x, drawn.y, cx, cy, -45);
    expect(local.x).toBeCloseTo(0);
    expect(local.y).toBeCloseTo(0);
  });
});

describe('maxFitWidthFrac', () => {
  // A corner-anchored placement with no margin and the given aspect-driving
  // rotation, so the older width/height/rotation cases keep their expectations.
  const noMargin = (rotDeg: number): Placement => ({
    ...base,
    margin_x_frac: 0,
    margin_y_frac: 0,
    rot_deg: rotDeg,
  });

  it('is bounded by width for a wide watermark on a square image', () => {
    // aspect 2 (wide): width binds first, so best fit is full image width.
    expect(maxFitWidthFrac(1000, 1000, 2, noMargin(0))).toBeCloseTo(1);
  });

  it('is bounded by height for a tall watermark', () => {
    // aspect 0.5 (tall): at width_frac f, height = 2*f*imgW; fills height at f=0.5.
    expect(maxFitWidthFrac(1000, 1000, 0.5, noMargin(0))).toBeCloseTo(0.5);
  });

  it('accounts for rotation (90° swaps the binding dimension)', () => {
    // A wide mark rotated 90° becomes tall; height now binds at f=1 on a square.
    expect(maxFitWidthFrac(1000, 1000, 2, noMargin(90))).toBeCloseTo(1);
    // A tall mark (aspect 0.5) rotated 90° becomes wide (bbox width = 2× drawn
    // width), so width binds first at f=0.5.
    expect(maxFitWidthFrac(1000, 1000, 0.5, noMargin(90))).toBeCloseTo(0.5);
  });

  it('subtracts the anchored-side margin so best fit never overflows', () => {
    // Corner anchor, margin 0.1 (→100px on a 1000² image): the wide mark binds on
    // width, but only 900px is available, so best fit is 0.9 (not 1).
    const p: Placement = { ...base, margin_x_frac: 0.1, margin_y_frac: 0.1, rot_deg: 0 };
    expect(maxFitWidthFrac(1000, 1000, 2, p)).toBeCloseTo(0.9);

    // At that best fit the resolved box sits flush inside the margin on both the
    // anchored edge and the opposite edge — no overflow.
    const r = resolveRect({ ...p, width_frac: 0.9 }, 1000, 1000, 2);
    expect(r.x).toBeCloseTo(0); // opposite (left) edge flush at the image border
    expect(r.x + r.w).toBeCloseTo(900); // anchored (right) edge keeps its margin
  });

  it('ignores margin for centered bands', () => {
    // A Center anchor carries no effective margin, so the full extent is usable.
    const p: Placement = { ...base, anchor: 'Center', margin_x_frac: 0.1, margin_y_frac: 0.1 };
    expect(maxFitWidthFrac(1000, 1000, 2, p)).toBeCloseTo(1);
  });
});

describe('anchorPoint', () => {
  // img 1000x800 -> shorter side 800, margin 0.05 -> 40px gap.
  it('pins to the anchored corner with margin', () => {
    expect(anchorPoint({ ...base, anchor: 'TopLeft' }, 1000, 800)).toEqual({ x: 40, y: 40 });
    expect(anchorPoint({ ...base, anchor: 'BottomRight' }, 1000, 800)).toEqual({
      x: 1000 - 40,
      y: 800 - 40,
    });
  });

  it('uses the image center for centered anchors', () => {
    expect(anchorPoint({ ...base, anchor: 'Center' }, 1000, 800)).toEqual({ x: 500, y: 400 });
    // A centered axis (e.g. TopCenter) is centered on that axis only.
    expect(anchorPoint({ ...base, anchor: 'TopCenter' }, 1000, 800)).toEqual({ x: 500, y: 40 });
  });
});

describe('canvasPointToImage', () => {
  it('maps through CSS scaling', () => {
    // Canvas is 1000x500 image pixels shown in a 500x250 box -> 2x scale.
    const rect = { left: 10, top: 20, width: 500, height: 250 };
    const p = canvasPointToImage(10 + 250, 20 + 125, rect, 1000, 500);
    expect(p.x).toBeCloseTo(500);
    expect(p.y).toBeCloseTo(250);
  });
});

describe('resolveRect matches the Rust model', () => {
  it('places a bottom-right box', () => {
    // 1000x800, width_frac 0.2 -> 200 wide; aspect 2 -> 100 tall.
    // Margins use the shorter side (800): 0.05*800 = 40 on both axes.
    const r = resolveRect(base, 1000, 800, 2);
    expect(r).toEqual({ x: 760, y: 660, w: 200, h: 100 });
  });
});

describe('pointInRect', () => {
  it('detects inside and outside', () => {
    const r = { x: 10, y: 10, w: 20, h: 20 };
    expect(pointInRect(15, 15, r)).toBe(true);
    expect(pointInRect(5, 15, r)).toBe(false);
  });
});

describe('snapToAnchor', () => {
  it('snaps a box near the top-left corner', () => {
    // Margins are relative to the shorter side (800): 24/800 = 0.03.
    const s = snapToAnchor(1000, 800, { x: 24, y: 24, w: 100, h: 50 });
    expect(s.anchor).toBe('TopLeft');
    expect(s.margin_x_frac).toBeCloseTo(0.03);
    expect(s.margin_y_frac).toBeCloseTo(0.03);
  });

  it('snaps a centered box to Center with zero margins', () => {
    const s = snapToAnchor(1000, 800, { x: 450, y: 375, w: 100, h: 50 });
    expect(s.anchor).toBe('Center');
    expect(s.margin_x_frac).toBe(0);
    expect(s.margin_y_frac).toBe(0);
  });

  it('snaps a box near the bottom-right corner', () => {
    // Gaps of 24px from each edge -> 24/800 = 0.03 against the shorter side.
    const s = snapToAnchor(1000, 800, { x: 876, y: 726, w: 100, h: 50 });
    expect(s.anchor).toBe('BottomRight');
    expect(s.margin_x_frac).toBeCloseTo(0.03);
    expect(s.margin_y_frac).toBeCloseTo(0.03);
  });
});

describe('dragToPlacement', () => {
  it('snaps to the anchor without taking margins from the drop position', () => {
    // Start bottom-right on a 1000x800 image, drag up-left into the top-left cell.
    const startBox = resolveRect(base, 1000, 800, 2); // x760 y660 w200 h100
    // Drop the box only ~10px from the top-left edges, an "in between" spot that
    // does NOT correspond to base's 0.05 margin.
    const next = dragToPlacement(base, startBox, 10 - 760, 10 - 660, 1000, 800);
    expect(next.anchor).toBe('TopLeft');
    // Margins are unchanged; they stay at the slider values, so the watermark
    // lands on the discrete snap position, not where the pointer dropped it.
    expect(next.margin_x_frac).toBe(base.margin_x_frac);
    expect(next.margin_y_frac).toBe(base.margin_y_frac);
    // Untouched fields carry over.
    expect(next.width_frac).toBe(base.width_frac);
    expect(next.opacity).toBe(base.opacity);
  });

  it('default placement resolves within image bounds', () => {
    const r = resolveRect(DEFAULT_PLACEMENT, 1920, 1080, 3);
    expect(r.x).toBeGreaterThanOrEqual(0);
    expect(r.y).toBeGreaterThanOrEqual(0);
    expect(r.x + r.w).toBeLessThanOrEqual(1920);
    expect(r.y + r.h).toBeLessThanOrEqual(1080);
  });
});

describe('resize', () => {
  it('nearCorner detects grabs at the box corners', () => {
    const r = { x: 100, y: 100, w: 40, h: 20 };
    expect(nearCorner(102, 98, r, 8)).toBe(true); // near top-left
    expect(nearCorner(140, 120, r, 8)).toBe(true); // near bottom-right
    expect(nearCorner(120, 110, r, 8)).toBe(false); // center, not a corner
  });

  it('resizeWidthFrac scales by the distance ratio and clamps', () => {
    expect(resizeWidthFrac(0.2, 100, 150)).toBeCloseTo(0.3); // 1.5x
    expect(resizeWidthFrac(0.2, 100, 50)).toBeCloseTo(0.1); // 0.5x
    expect(resizeWidthFrac(0.2, 100, 100000)).toBe(1.0); // clamped to max
    expect(resizeWidthFrac(0.2, 100, 1)).toBe(0.02); // clamped to min
    expect(resizeWidthFrac(0.2, 0, 50)).toBe(0.2); // degenerate d0
  });
});
