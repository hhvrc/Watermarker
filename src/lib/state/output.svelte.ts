// Output/export settings: destination folder, naming, and per-export toggles.

import { open } from '@tauri-apps/plugin-dialog';
import type { OutputSettings } from '$lib/types';

class OutputStore {
  settings = $state<OutputSettings>({
    dir: '',
    suffix: '_wm',
    overwrite: false,
    strip_metadata: false,
    export_clean: false,
  });

  /** Prompt for an output folder; returns whether one was chosen. */
  pickDir = async (): Promise<boolean> => {
    const sel = await open({ directory: true, multiple: false });
    if (sel && !Array.isArray(sel)) {
      this.settings.dir = sel;
      return true;
    }
    return false;
  };
}

export const output = new OutputStore();
