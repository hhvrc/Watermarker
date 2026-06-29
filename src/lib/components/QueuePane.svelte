<script lang="ts">
  import Button from './ui/Button.svelte';
  import Badge from './ui/Badge.svelte';
  import type { ImageRef } from '$lib/types';

  interface Props {
    images: ImageRef[];
    index: number;
    onselect: (i: number) => void;
    onremove: (i: number) => void;
    onclear: () => void;
  }

  let { images, index, onselect, onremove, onclear }: Props = $props();

  function onKey(e: KeyboardEvent, i: number) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onselect(i);
    }
  }
</script>

<div class="select-none space-y-2">
  <div class="flex items-center justify-between text-neutral-400">
    <span>Queue ({images.length})</span>
    {#if images.length > 0}
      <Button variant="danger" size="xxs" onclick={onclear}>Clear</Button>
    {/if}
  </div>

  {#if images.length === 0}
    <p class="text-xs text-neutral-500">No images loaded.</p>
  {:else}
    <div class="grid grid-cols-2 gap-2 sm:grid-cols-3">
      {#each images as item, i (item.id)}
        <div
          role="option"
          tabindex="0"
          aria-selected={i === index}
          aria-label={`Select ${item.name}`}
          class={[
            'group relative cursor-pointer overflow-hidden rounded-lg border focus:outline-none focus-visible:ring-2 focus-visible:ring-neutral-500',
            i === index ? 'border-neutral-300' : 'border-neutral-800 hover:border-neutral-600',
          ]}
          onclick={() => onselect(i)}
          onkeydown={(e) => onKey(e, i)}
        >
          {#if item.previewUrl}
            <img
              src={item.previewUrl}
              alt={item.name}
              class="aspect-video w-full object-cover"
              loading="lazy"
            />
          {:else}
            <div
              class="flex aspect-video w-full items-center justify-center bg-neutral-800 text-[10px] text-neutral-500"
            >
              …
            </div>
          {/if}

          <div
            class="absolute inset-x-0 bottom-0 truncate bg-neutral-950/70 px-1 py-0.5 text-[10px] text-neutral-200"
            title={item.name}
          >
            {item.name}
          </div>

          {#if item.placement}
            <Badge class="absolute left-1 top-1">set</Badge>
          {/if}

          {#if item.status === 'done'}
            <Badge variant="success" class="absolute left-1 bottom-4">done</Badge>
          {:else if item.status === 'error'}
            <Badge variant="error" class="absolute left-1 bottom-4" title={item.error}>error</Badge>
          {:else if item.status === 'processing'}
            <Badge variant="warning" class="absolute left-1 bottom-4">…</Badge>
          {/if}

          <button
            type="button"
            aria-label={`Remove ${item.name}`}
            title="Remove"
            class="absolute right-1 top-1 flex h-5 w-5 items-center justify-center rounded bg-neutral-950/70 text-xs text-neutral-200 opacity-0 transition-opacity hover:bg-rose-600/90 group-hover:opacity-100 focus:opacity-100"
            onclick={(e) => {
              e.stopPropagation();
              onremove(i);
            }}
          >
            ×
          </button>
        </div>
      {/each}
    </div>
  {/if}
</div>
