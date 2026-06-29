<script lang="ts">
  import DropZone from '$lib/components/DropZone.svelte';
  import { watermark } from '$lib/state/watermark.svelte';
  import { dnd } from '$lib/services/dnd.svelte';
</script>

<div class="select-none space-y-2 text-[13px]">
  <div class="flex items-center justify-between">
    <span class="text-neutral-400">Watermark</span>
    {#if watermark.ref}
      <button class="text-[11px] text-neutral-400 hover:text-rose-300" onclick={watermark.clear}>
        Remove
      </button>
    {/if}
  </div>
  <DropZone
    bind:element={dnd.zoneEl}
    active={dnd.wmDragOver}
    onclick={watermark.loadFromPicker}
    titleText={watermark.ref ? 'Drop or click to replace' : 'Drop watermark here'}
    underText={watermark.ref ? undefined : 'or click to choose (SVG / PNG / …)'}
  >
    {#if watermark.ref}
      <img
        src={watermark.ref.previewUrl}
        alt="watermark"
        class="max-h-16 max-w-full object-contain"
      />
    {/if}
  </DropZone>
</div>
