// Watermark placement: the per-image placement plus the sticky default carried
// forward to the next image the user touches.

import { DEFAULT_PLACEMENT } from '$lib/placement';
import type { ImageRef, Placement } from '$lib/types';
import { queue } from './queue.svelte';

class PlacementStore {
  last = $state<Placement>({ ...DEFAULT_PLACEMENT });
  // Placement shown for the selected image: its own, or the sticky default.
  current = $derived(queue.current?.placement ?? this.last);

  /** Set the selected image's placement and carry it forward as the default. */
  update = (p: Placement) => {
    const img = queue.current;
    if (!img) return;
    img.placement = p;
    this.last = p;
  };

  /**
   * Freeze the selected image's currently-shown placement onto it. Images that
   * only inherit the sticky default (placement === null) otherwise float: editing
   * a later image moves `last` and silently shifts them. Call before navigating
   * away so each image you've viewed keeps the position it had. Touched images
   * already carry an explicit placement, so this leaves them untouched.
   */
  commitCurrent = () => {
    const img = queue.current;
    if (img && !img.placement) img.placement = { ...this.last };
  };

  /** Apply a placement (e.g. from a preset) to the default and current image. */
  apply = (p: Placement) => {
    this.last = { ...p };
    const img = queue.current;
    if (img) img.placement = { ...p };
  };

  effectiveFor = (img: ImageRef): Placement => img.placement ?? this.last;
}

export const placement = new PlacementStore();
