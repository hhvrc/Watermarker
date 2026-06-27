// Saved presets: named bundles of watermark + placement, applied as a unit.

import type { Preset } from '$lib/types';
import { status } from './status.svelte';
import { watermark } from './watermark.svelte';
import { placement } from './placement.svelte';

class PresetStore {
  list = $state<Preset[]>([]);
  canSave = $derived(watermark.ref !== null);

  /** Apply a preset: load its logo (if any), then its placement. */
  apply = async (p: Preset) => {
    if (p.watermark_path) {
      status.busy = true;
      try {
        await watermark.applyManaged(p.watermark_path);
      } catch (e) {
        status.text = `Failed to load preset watermark: ${e}`;
      } finally {
        status.busy = false;
      }
    }
    // Placement covers anchor, margin, size, rotation and opacity in one struct.
    placement.apply(p.placement);
  };

  /** Save the current logo + placement under a name, replacing any same-named one. */
  save = (name: string) => {
    const preset: Preset = {
      name,
      placement: { ...placement.current },
      watermark_path: watermark.path,
    };
    const idx = this.list.findIndex((p) => p.name === name);
    if (idx >= 0) {
      const next = [...this.list];
      next[idx] = preset;
      this.list = next;
    } else {
      this.list = [...this.list, preset];
    }
  };

  remove = (name: string) => {
    this.list = this.list.filter((p) => p.name !== name);
  };
}

export const presets = new PresetStore();
