// The image queue: the imported images and which one is selected.

import { open } from '@tauri-apps/plugin-dialog';
import { importImages, removeImage, getImagePreviewUrl } from '$lib/ipc';
import type { ImageMeta, ImageRef } from '$lib/types';
import { IMAGE_EXTS } from '$lib/dnd';
import { status } from './status.svelte';

class QueueStore {
  images = $state<ImageRef[]>([]);
  index = $state(0);
  current = $derived(this.images[this.index] ?? null);

  #push = async (meta: ImageMeta) => {
    let previewUrl: string | null = null;
    try {
      previewUrl = await getImagePreviewUrl(meta.id);
    } catch (e) {
      console.error('preview failed', e);
    }
    this.images = [...this.images, { ...meta, placement: null, previewUrl, status: 'pending' }];
  };

  /** Import images by path (native drag-drop or the picker). */
  importPaths = async (paths: string[]) => {
    status.busy = true;
    try {
      const metas = await importImages(paths);
      for (const m of metas) await this.#push(m);
    } catch (e) {
      status.text = `Failed to load images: ${e}`;
    } finally {
      status.busy = false;
    }
  };

  loadFromPicker = async () => {
    // Stay busy across the whole picker + import, matching the original flow.
    status.busy = true;
    try {
      const sel = await open({
        multiple: true,
        filters: [{ name: 'Images', extensions: IMAGE_EXTS }],
      });
      if (!sel) return;
      await this.importPaths(Array.isArray(sel) ? sel : [sel]);
    } finally {
      status.busy = false;
    }
  };

  removeAt = async (i: number) => {
    const img = this.images[i];
    if (!img) return;
    if (img.previewUrl) URL.revokeObjectURL(img.previewUrl);
    try {
      await removeImage(img.id);
    } catch (e) {
      console.error('remove failed', e);
    }
    this.images = this.images.filter((_, k) => k !== i);
    if (this.index >= this.images.length) this.index = Math.max(0, this.images.length - 1);
    else if (i < this.index) this.index--;
  };

  /** Empty the queue and free every preview. */
  clear = async () => {
    for (const img of this.images) {
      if (img.previewUrl) URL.revokeObjectURL(img.previewUrl);
      try {
        await removeImage(img.id);
      } catch (e) {
        console.error('remove failed', e);
      }
    }
    this.images = [];
    this.index = 0;
  };

  select = (i: number) => {
    this.index = i;
  };

  prev = () => {
    if (this.index > 0) this.index--;
  };

  next = () => {
    if (this.index < this.images.length - 1) this.index++;
  };
}

export const queue = new QueueStore();
