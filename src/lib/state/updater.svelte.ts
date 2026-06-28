// Auto-update notification. On startup we ask GitHub Releases (via the Tauri
// updater plugin) whether a newer signed build exists. If so, we surface a
// dismissible dialog — the user decides whether to install. Nothing updates
// automatically.
//
// Three ways to dismiss:
//   • Remind me later  — hide for this session; ask again next launch.
//   • Skip this version — persist `skippedVersion`; stay quiet until a newer
//     version ships.
//   • Never show again  — persist `disabled`; stop checking entirely.
//
// `skippedVersion` / `disabled` are persisted through the settings store
// (see persistence.svelte.ts, which snapshots them). Pure decision logic lives
// in `$lib/updater` (unit-tested); this file owns reactive state + side effects.

import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { NO_UPDATE, toNotice, bannerVisible } from '$lib/updater';

class UpdaterStore {
  notice = $state(NO_UPDATE);
  installing = $state(false);
  error = $state('');
  dismissed = $state(false);

  // Persisted preferences (restored by persistence.load before check runs).
  skippedVersion = $state<string | null>(null);
  disabled = $state(false);

  #update: Update | null = null;

  get available() {
    return this.notice.available;
  }
  get version() {
    return this.notice.version;
  }
  get current() {
    return this.notice.current;
  }
  get notes() {
    return this.notice.notes;
  }

  /** Whether the dialog should be shown. */
  visible = $derived(bannerVisible(this.notice, this.dismissed));

  /** Check once at startup. Stays silent on failure (offline, dev build, …),
   *  when updates are disabled, or when the offered version was skipped. */
  check = async () => {
    if (this.disabled) return;
    try {
      const update = await check();
      if (!update) return;
      if (update.version === this.skippedVersion) return;
      this.#update = update;
      this.notice = toNotice(update);
    } catch (e) {
      // No endpoint in dev, no network, etc. — not worth bothering the user.
      console.error('update check failed', e);
    }
  };

  /** Download + install the pending update, then relaunch into the new version. */
  install = async () => {
    if (!this.#update || this.installing) return;
    this.installing = true;
    this.error = '';
    try {
      await this.#update.downloadAndInstall();
      await relaunch();
    } catch (e) {
      this.error = `Update failed: ${e}`;
      this.installing = false;
    }
  };

  /** Hide for this session only; the prompt returns on the next launch. */
  remindLater = () => {
    this.dismissed = true;
  };

  /** Don't prompt for this version again (a newer one will still prompt). */
  skipVersion = () => {
    this.skippedVersion = this.notice.version;
    this.dismissed = true;
  };

  /** Turn off update checks and prompts entirely. */
  disable = () => {
    this.disabled = true;
    this.dismissed = true;
  };
}

export const updater = new UpdaterStore();
