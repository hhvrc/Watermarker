import { defineConfig } from 'vitest/config';

// Unit tests for pure TS logic (placement math, mapping). No DOM/Svelte needed.
export default defineConfig({
  test: {
    environment: 'node',
    include: ['src/**/*.test.ts'],
  },
  // Bypass the project tsconfig (it `extends` the SvelteKit-generated tsconfig,
  // which may not exist yet) so esbuild can transform the pure-TS tests standalone.
  esbuild: {
    tsconfigRaw: '{}',
  },
});
