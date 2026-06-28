// Typed wrappers around the Tauri command surface.
//
// Preview *pixels* come back as raw ArrayBuffers (no base64) and are wrapped in
// Blob URLs here. Callers own the returned URL and must revokeObjectURL when done.

import { invoke } from '@tauri-apps/api/core';
import type {
  BatchSummary,
  ImageMeta,
  OutputSettings,
  Placement,
  Preset,
  WatermarkMeta,
  WatermarkMetaStored,
} from './types';

export async function importImages(paths: string[]): Promise<ImageMeta[]> {
  return invoke('import_images', { paths });
}

/**
 * Import one image from raw bytes (drag-dropped file or bitmap). Bytes go as the
 * raw IPC body (not JSON); the name travels URI-encoded in a header.
 */
export async function importImageBytes(name: string, bytes: Uint8Array): Promise<ImageMeta> {
  return invoke('import_image_bytes', bytes, { headers: { name: encodeURIComponent(name) } });
}

export async function removeImage(id: string): Promise<void> {
  await invoke('remove_image', { id });
}

export async function setWatermark(path: string): Promise<WatermarkMeta> {
  return invoke('set_watermark', { path });
}

/** Load a watermark and copy it into managed storage; returns the durable path. */
export async function setWatermarkPersisted(path: string): Promise<WatermarkMetaStored> {
  return invoke('set_watermark_persisted', { path });
}

/** Set the watermark from raw dropped bytes (file or bitmap). */
export async function setWatermarkBytes(name: string, bytes: Uint8Array): Promise<WatermarkMeta> {
  return invoke('set_watermark_bytes', bytes, { headers: { name: encodeURIComponent(name) } });
}

function toBlobUrl(buf: ArrayBuffer): string {
  return URL.createObjectURL(new Blob([buf], { type: 'image/png' }));
}

export async function getImagePreviewUrl(id: string): Promise<string> {
  const buf = await invoke<ArrayBuffer>('get_image_preview', { id });
  return toBlobUrl(buf);
}

export async function getWatermarkPreviewUrl(): Promise<string> {
  const buf = await invoke<ArrayBuffer>('get_watermark_preview');
  return toBlobUrl(buf);
}

export async function renderExactPreviewUrl(
  imageId: string,
  placement: Placement,
): Promise<string> {
  const buf = await invoke<ArrayBuffer>('render_exact_preview', { imageId, placement });
  return toBlobUrl(buf);
}

export interface ImageJob {
  image_id: string;
  placement: Placement;
}

export async function processBatch(
  items: ImageJob[],
  output: OutputSettings,
): Promise<BatchSummary> {
  return invoke('process_batch', { job: { items, output } });
}

/** Persistent settings (mirror of Rust `Settings`). */
export interface AppSettings {
  watermark_path: string | null;
  output_dir: string | null;
  strip_metadata: boolean;
  /** Last-used placement, restored as the sticky default on next launch. */
  placement: Placement | null;
  /** Saved presets, in user order. */
  presets: Preset[];
  /** A release version the user chose to skip (hidden until a newer one ships). */
  skipped_version: string | null;
  /** When true, the app never checks for or prompts about updates. */
  updates_disabled: boolean;
}

/** Read settings; Rust prunes any persisted paths that no longer exist. */
export async function getSettings(): Promise<AppSettings> {
  return invoke('get_settings');
}

export async function setSettings(value: AppSettings): Promise<void> {
  await invoke('set_settings', { value });
}
