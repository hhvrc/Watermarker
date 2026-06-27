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

  /** Apply a placement (e.g. from a preset) to the default and current image. */
  apply = (p: Placement) => {
    this.last = { ...p };
    const img = queue.current;
    if (img) img.placement = { ...p };
  };

  effectiveFor = (img: ImageRef): Placement => img.placement ?? this.last;
}

export const placement = new PlacementStore();
