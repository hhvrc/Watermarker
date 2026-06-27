// The edit/review workflow: which mode the UI is in, the staged exact previews,
// and the batch export run.

import { listen } from '@tauri-apps/api/event';
import { renderExactPreviewUrl, processBatch } from '$lib/ipc';
import type { ProgressEvent } from '$lib/types';
import { queue } from './queue.svelte';
import { placement } from './placement.svelte';
import { watermark } from './watermark.svelte';
import { output } from './output.svelte';
import { status } from './status.svelte';

class WorkspaceStore {
  mode = $state<'edit' | 'review'>('edit');
  reviewUrls = $state<Record<string, string>>({});
  processing = $state(false);
  progress = $state({ completed: 0, total: 0 });

  canStage = $derived(queue.images.length > 0 && watermark.ref !== null);
  canCommit = $derived(this.canStage && !this.processing);

  toEdit = () => {
    this.mode = 'edit';
  };

  editImage = (i: number) => {
    placement.commitCurrent();
    queue.select(i);
    this.mode = 'edit';
  };

  #resetReview = () => {
    for (const url of Object.values(this.reviewUrls)) URL.revokeObjectURL(url);
    this.reviewUrls = {};
  };

  /** Empty the queue and return to a clean editing state. */
  clearQueue = async () => {
    await queue.clear();
    this.#resetReview();
    this.mode = 'edit';
  };

  /** Render an exact Rust preview for every image and switch to review mode. */
  stage = async () => {
    if (!this.canStage) return;
    status.busy = true;
    this.mode = 'review';
    this.#resetReview();
    try {
      for (const img of queue.images) {
        try {
          this.reviewUrls[img.id] = await renderExactPreviewUrl(
            img.id,
            placement.effectiveFor(img),
          );
        } catch (e) {
          console.error('exact preview failed', e);
        }
      }
    } finally {
      status.busy = false;
    }
  };

  commit = async () => {
    if (!this.canCommit) return;
    if (!output.settings.dir) {
      const picked = await output.pickDir();
      if (!picked || !output.settings.dir) {
        status.text = 'Select an output folder to save into.';
        return;
      }
    }
    this.processing = true;
    status.text = '';
    this.progress = { completed: 0, total: queue.images.length };
    for (const img of queue.images) img.status = 'processing';

    const unlisten = await listen<ProgressEvent>('wm://progress', (e) => {
      const ev = e.payload;
      this.progress = { completed: ev.completed, total: ev.total };
      const img = queue.images.find((i) => i.id === ev.image_id);
      if (img) {
        img.status = ev.status;
        img.error = ev.error ?? undefined;
      }
    });

    try {
      const items = queue.images.map((i) => ({
        image_id: i.id,
        placement: placement.effectiveFor(i),
      }));
      const summary = await processBatch(items, output.settings);
      status.text =
        `Saved ${summary.succeeded}/${summary.total}` +
        (summary.failed ? `, ${summary.failed} failed` : '');
    } catch (e) {
      status.text = `Batch failed: ${e}`;
    } finally {
      unlisten();
      this.processing = false;
    }
  };
}

export const workspace = new WorkspaceStore();
