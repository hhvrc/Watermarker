<script lang="ts">
  import Button from './ui/Button.svelte';
  import type { Preset } from '$lib/types';

  interface Props {
    presets: Preset[];
    canSave: boolean;
    onapply: (p: Preset) => void;
    onsave: (name: string) => void;
    ondelete: (name: string) => void;
  }

  let { presets, canSave, onapply, onsave, ondelete }: Props = $props();

  let name = $state('');

  function save() {
    const n = name.trim();
    if (!n) return;
    onsave(n);
    name = '';
  }
</script>

<div class="select-none space-y-2 text-[13px] text-neutral-300">
  <span class="text-neutral-400">Presets</span>

  {#if presets.length === 0}
    <p class="text-xs text-neutral-500">No presets saved yet.</p>
  {:else}
    <div class="flex flex-col gap-1">
      {#each presets as p (p.name)}
        <div class="group flex items-center gap-1">
          <button
            type="button"
            title="Apply preset"
            class="flex flex-1 items-center gap-2 truncate rounded border border-neutral-700 bg-neutral-800 px-2 py-1 text-left text-xs hover:border-neutral-500"
            onclick={() => onapply(p)}
          >
            {#if p.watermark_path}
              <span class="h-1.5 w-1.5 shrink-0 rounded-full bg-neutral-400"></span>
            {/if}
            <span class="truncate">{p.name}</span>
          </button>
          <button
            type="button"
            aria-label={`Delete preset ${p.name}`}
            title="Delete preset"
            class="flex h-6 w-6 shrink-0 items-center justify-center rounded text-neutral-500 hover:bg-rose-600/80 hover:text-white"
            onclick={() => ondelete(p.name)}
          >
            ×
          </button>
        </div>
      {/each}
    </div>
  {/if}

  <div class="flex items-center gap-1">
    <input
      class="min-w-0 flex-1 select-text rounded border border-neutral-700 bg-neutral-800 px-2 py-1 text-xs text-neutral-100 placeholder:text-neutral-500 disabled:opacity-40"
      placeholder="Preset name…"
      disabled={!canSave}
      bind:value={name}
      onkeydown={(e) => {
        if (e.key === 'Enter') {
          e.preventDefault();
          save();
        }
      }}
    />
    <Button size="xs" class="shrink-0" disabled={!canSave || name.trim() === ''} onclick={save}>
      Save
    </Button>
  </div>
  {#if !canSave}
    <p class="text-[11px] text-neutral-500">Load a watermark to save a preset.</p>
  {/if}
</div>
