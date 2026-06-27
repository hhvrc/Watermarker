<script lang="ts">
  import { dnd } from '$lib/services/dnd.svelte';
</script>

{#if dnd.showOverlay}
  <div
    class="pointer-events-none absolute inset-0 z-50 flex items-center justify-center bg-white/5 ring-4 ring-inset ring-neutral-400/60"
  >
    <div class="rounded-xl bg-neutral-900/90 px-4 py-2 text-sm text-neutral-100 shadow-lg">
      Drop images to add them
    </div>
  </div>
{/if}
