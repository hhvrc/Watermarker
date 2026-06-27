// Native (Tauri) drag-and-drop helpers: accepted file types and physical-pixel
// hit-testing against on-screen zones.

/** Image file extensions accepted for import (lowercase, no dot). */
export const IMAGE_EXTS = ['png', 'jpg', 'jpeg', 'tiff', 'tif', 'webp'];

/** Watermark file extensions accepted (raster formats plus SVG). */
export const WATERMARK_EXTS = ['svg', 'png', 'webp', 'jpg', 'jpeg', 'tiff', 'tif'];

/** Lowercase extension of a path, without the leading dot. */
function extOf(path: string): string {
  return path.slice(path.lastIndexOf('.') + 1).toLowerCase();
}

/** Whether `path` looks like a watermark file we can load. */
export function isWatermarkPath(path: string): boolean {
  return WATERMARK_EXTS.includes(extOf(path));
}

/**
 * Whether a native drag-drop position (in physical pixels) falls inside `el`.
 * Tauri reports physical coordinates, so divide by the device pixel ratio before
 * comparing against the element's CSS-pixel rect.
 */
export function physPointInEl(el: HTMLElement | null, physX: number, physY: number): boolean {
  if (!el) return false;
  const r = el.getBoundingClientRect();
  const dpr = window.devicePixelRatio || 1;
  const x = physX / dpr;
  const y = physY / dpr;
  return x >= r.left && x <= r.right && y >= r.top && y <= r.bottom;
}
