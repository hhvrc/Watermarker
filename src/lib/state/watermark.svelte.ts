// The current watermark and its managed (durable) file path.

import { open } from '@tauri-apps/plugin-dialog';
import { setWatermark, setWatermarkPersisted, getWatermarkPreviewUrl } from '$lib/ipc';
import type { WatermarkRef, WatermarkMeta } from '$lib/types';
import { WATERMARK_EXTS } from '$lib/dnd';
import { status } from './status.svelte';

class WatermarkStore {
  ref = $state<WatermarkRef | null>(null);
  // Managed path the logo was copied to; persisted so it survives the original
  // file moving. Native drag-drop and the picker both yield a path.
  path = $state<string | null>(null);

  #applyPreview = async (meta: WatermarkMeta) => {
    const previewUrl = await getWatermarkPreviewUrl();
    if (this.ref?.previewUrl) URL.revokeObjectURL(this.ref.previewUrl);
    this.ref = { ...meta, previewUrl };
  };

  /**
   * Set the watermark from a file path (picker or native drop). The logo is copied
   * into managed storage so it survives the original file moving; the durable
   * path is kept, not the user's original location.
   */
  setFromPath = async (path: string) => {
    status.busy = true;
    try {
      const { stored_path, ...meta } = await setWatermarkPersisted(path);
      await this.#applyPreview(meta);
      this.path = stored_path;
    } catch (e) {
      status.text = `Failed to set watermark: ${e}`;
    } finally {
      status.busy = false;
    }
  };

  /**
   * Load an already-managed path (a preset's logo or restored settings) without
   * re-copying it. Throws on failure so the caller can report context.
   */
  applyManaged = async (path: string) => {
    const meta = await setWatermark(path);
    await this.#applyPreview(meta);
    this.path = path;
  };

  loadFromPicker = async () => {
    const sel = await open({
      multiple: false,
      filters: [{ name: 'Watermark', extensions: WATERMARK_EXTS }],
    });
    if (!sel || Array.isArray(sel)) return;
    await this.setFromPath(sel);
  };

  clear = () => {
    if (this.ref?.previewUrl) URL.revokeObjectURL(this.ref.previewUrl);
    this.ref = null;
    this.path = null;
  };
}

export const watermark = new WatermarkStore();
