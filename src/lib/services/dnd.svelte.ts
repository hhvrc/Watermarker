// Native drag-drop routing: tracks hover state and sends a dropped file to the
// watermark zone or the image queue based on where (and what) it is.

import { getCurrentWebview } from '@tauri-apps/api/webview';
import { isWatermarkPath, physPointInEl } from '$lib/dnd';
import { watermark } from '$lib/state/watermark.svelte';
import { queue } from '$lib/state/queue.svelte';
import { workspace } from '$lib/state/workspace.svelte';

class DndController {
  dragOver = $state(false);
  wmDragOver = $state(false);
  // Bound to the watermark drop zone so drops can be routed by position.
  zoneEl = $state<HTMLElement | null>(null);
  // The full-window overlay shows for an images drop, not a watermark drop.
  showOverlay = $derived(this.dragOver && !this.wmDragOver);

  #unlisten: (() => void) | null = null;

  register = async () => {
    this.#unlisten = await getCurrentWebview().onDragDropEvent((event) => {
      const p = event.payload;
      if (p.type === 'enter') {
        this.dragOver = true;
      } else if (p.type === 'over') {
        this.dragOver = true;
        this.wmDragOver = physPointInEl(this.zoneEl, p.position.x, p.position.y);
      } else if (p.type === 'leave') {
        this.dragOver = false;
        this.wmDragOver = false;
      } else if (p.type === 'drop') {
        const overWm = physPointInEl(this.zoneEl, p.position.x, p.position.y);
        this.dragOver = false;
        this.wmDragOver = false;
        if (workspace.processing || p.paths.length === 0) return;
        if (overWm && isWatermarkPath(p.paths[0])) {
          void watermark.setFromPath(p.paths[0]);
        } else {
          void queue.importPaths(p.paths);
        }
      }
    });
  };

  dispose = () => {
    this.#unlisten?.();
  };
}

export const dnd = new DndController();
