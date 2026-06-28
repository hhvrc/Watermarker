import { describe, it, expect } from 'vitest';
import { toNotice, bannerVisible, NO_UPDATE, type UpdateInfo } from './updater';

describe('toNotice', () => {
  it('reports "up to date" when check() returns null', () => {
    expect(toNotice(null)).toEqual(NO_UPDATE);
  });

  it('marks available and carries version + notes from the update', () => {
    const update: UpdateInfo = { version: '1.2.3', currentVersion: '1.2.0', body: 'Fixes things' };
    expect(toNotice(update)).toEqual({
      available: true,
      version: '1.2.3',
      current: '1.2.0',
      notes: 'Fixes things',
    });
  });

  it('treats a missing release body as empty notes', () => {
    expect(toNotice({ version: '2.0.0' }).notes).toBe('');
    expect(toNotice({ version: '2.0.0', body: null }).notes).toBe('');
  });
});

describe('bannerVisible', () => {
  it('shows when an update is available and not dismissed', () => {
    expect(
      bannerVisible({ available: true, version: '1.0.0', current: '0.9.0', notes: '' }, false),
    ).toBe(true);
  });

  it('hides once dismissed even if an update is available', () => {
    expect(
      bannerVisible({ available: true, version: '1.0.0', current: '0.9.0', notes: '' }, true),
    ).toBe(false);
  });

  it('never shows when no update is available', () => {
    expect(bannerVisible(NO_UPDATE, false)).toBe(false);
    expect(bannerVisible(NO_UPDATE, true)).toBe(false);
  });
});
