// Settings persistence: restore on launch, then autosave (debounced) whenever a
// persisted field changes. Owns all reading/writing of the Rust settings store so
// the domain stores stay unaware of persistence.

import { getSettings, setSettings, type AppSettings } from '$lib/ipc';
import { watermark } from './watermark.svelte';
import { placement } from './placement.svelte';
import { presets } from './presets.svelte';
import { output } from './output.svelte';

class Persistence {
  #timer: ReturnType<typeof setTimeout> | null = null;
  #pending: AppSettings | null = null;
  #stop: (() => void) | null = null;

  #snapshot = (): AppSettings => ({
    watermark_path: watermark.path,
    output_dir: output.settings.dir || null,
    strip_metadata: output.settings.strip_metadata,
    placement: placement.last,
    presets: presets.list,
  });

  /** Restore persisted settings into the stores. Rust has already pruned any
   * paths that no longer exist on disk. */
  load = async () => {
    let saved: AppSettings;
    try {
      saved = await getSettings();
    } catch (e) {
      console.error('load settings failed', e);
      return;
    }

    if (saved.output_dir) output.settings.dir = saved.output_dir;
    output.settings.strip_metadata = saved.strip_metadata;
    if (saved.placement) placement.last = saved.placement;
    if (saved.presets) presets.list = saved.presets;

    if (saved.watermark_path) {
      try {
        await watermark.applyManaged(saved.watermark_path);
      } catch {
        // File exists but couldn't be loaded, so forget it.
        watermark.path = null;
        this.#save(this.#snapshot());
      }
    }
  };

  /** Begin autosaving whenever a persisted field changes. Call after {@link load}. */
  start = () => {
    this.#stop = $effect.root(() => {
      let first = true;
      $effect(() => {
        // Reading the snapshot subscribes to exactly the persisted fields. A slider
        // drag fires many changes, so the actual write is debounced below.
        const snap = this.#snapshot();
        if (first) {
          first = false;
          return;
        }
        this.#schedule(snap);
      });
    });
  };

  #schedule = (snap: AppSettings) => {
    this.#pending = snap;
    if (this.#timer) clearTimeout(this.#timer);
    this.#timer = setTimeout(() => {
      this.#timer = null;
      if (this.#pending) this.#save(this.#pending);
    }, 300);
  };

  #save = (snap: AppSettings) => {
    setSettings(snap).catch((e) => console.error('persist failed', e));
  };

  /** Flush a pending save and stop autosaving. */
  flush = () => {
    if (this.#timer) {
      clearTimeout(this.#timer);
      this.#timer = null;
      if (this.#pending) this.#save(this.#pending);
    }
    this.#stop?.();
    this.#stop = null;
  };
}

export const persistence = new Persistence();
