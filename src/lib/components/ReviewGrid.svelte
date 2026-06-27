<script lang="ts">
  import type { ImageRef } from '$lib/types';

  interface Props {
    images: ImageRef[];
    urls: Record<string, string>;
    onedit: (i: number) => void;
  }

  let { images, urls, onedit }: Props = $props();
</script>

<div class="h-full overflow-auto p-4">
  <div class="grid grid-cols-2 gap-4 lg:grid-cols-3 xl:grid-cols-4">
    {#each images as item, i (item.id)}
      <button
        type="button"
        class="group flex flex-col overflow-hidden rounded-xl border border-neutral-800 bg-neutral-900 text-left transition-colors hover:border-neutral-500"
        onclick={() => onedit(i)}
        title="Click to adjust this image"
      >
        <div class="flex aspect-video items-center justify-center bg-black">
          {#if urls[item.id]}
            <img src={urls[item.id]} alt={item.name} class="max-h-full max-w-full object-contain" />
          {:else}
            <span class="text-xs text-neutral-500">rendering…</span>
          {/if}
        </div>
        <div class="flex items-center justify-between gap-2 px-2 py-1.5">
          <span class="truncate text-xs text-neutral-300" title={item.name}>{item.name}</span>
          {#if item.status === 'done'}
            <span class="rounded bg-emerald-600/90 px-1 text-[10px] text-white">done</span>
          {:else if item.status === 'error'}
            <span class="rounded bg-rose-600/90 px-1 text-[10px] text-white" title={item.error}
              >error</span
            >
          {/if}
        </div>
      </button>
    {/each}
  </div>
</div>
