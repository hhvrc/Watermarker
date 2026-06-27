<script lang="ts">
  import { ANCHORS, ANCHOR_LABELS } from '$lib/placement';
  import type { Anchor } from '$lib/types';

  interface Props {
    value: Anchor;
    onchange: (a: Anchor) => void;
    disabled?: boolean;
  }

  let { value, onchange, disabled = false }: Props = $props();
</script>

<div class="grid w-max grid-cols-3 gap-1" role="radiogroup" aria-label="Watermark position">
  {#each ANCHORS as a (a)}
    <button
      type="button"
      {disabled}
      title={ANCHOR_LABELS[a]}
      aria-label={ANCHOR_LABELS[a]}
      aria-checked={value === a}
      role="radio"
      class={[
        'flex h-7 w-7 items-center justify-center rounded border transition-colors disabled:opacity-40',
        value === a
          ? 'border-neutral-400 bg-neutral-700'
          : 'border-neutral-700 bg-neutral-800 hover:border-neutral-500',
      ]}
      onclick={() => onchange(a)}
    >
      <span
        class={[
          'block h-1.5 w-1.5 rounded-full',
          value === a ? 'bg-neutral-100' : 'bg-neutral-500',
        ]}
      ></span>
    </button>
  {/each}
</div>
