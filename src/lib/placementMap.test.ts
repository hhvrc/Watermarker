import { describe, it, expect } from 'vitest';
import { resolveRect, DEFAULT_PLACEMENT } from './placement';
import {
  canvasPointToImage,
  pointInRect,
  snapToAnchor,
  dragToPlacement,
  nearCorner,
  resizeWidthFrac,
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
