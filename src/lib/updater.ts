// Pure update-notification logic, split out from the runes store so it can be
// unit-tested without a Tauri runtime or the Svelte compiler. The store
// (`state/updater.svelte.ts`) just holds reactive state and calls these.

/** The minimal shape of a Tauri updater `check()` result we care about. */
export interface UpdateInfo {
  version: string;
  currentVersion?: string;
  body?: string | null;
}

/** Notification state derived from a `check()` result. */
export interface UpdateNotice {
  available: boolean;
  /** The newer version offered by the release. */
  version: string;
  /** The version currently installed. */
  current: string;
  notes: string;
}

export const NO_UPDATE: UpdateNotice = { available: false, version: '', current: '', notes: '' };

/** Map a `check()` result (or null = up to date) to the notification state. */
export function toNotice(update: UpdateInfo | null): UpdateNotice {
  if (!update) return NO_UPDATE;
  return {
    available: true,
    version: update.version,
    current: update.currentVersion ?? '',
    notes: update.body ?? '',
  };
}

/** Whether the update dialog should be shown. */
export function bannerVisible(notice: UpdateNotice, dismissed: boolean): boolean {
  return notice.available && !dismissed;
}
