import { defineConfig } from 'vitest/config';

// Unit tests for pure TS logic (placement math, mapping). No DOM/Svelte needed.
export default defineConfig({
  test: {
    environment: 'node',
    include: ['src/**/*.test.ts'],
  },
});
