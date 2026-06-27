// Transient UI feedback shared across the app: a status line and a global busy
// flag that disables actions while a long operation runs.

class StatusStore {
  text = $state('');
  busy = $state(false);
}

export const status = new StatusStore();
