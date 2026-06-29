<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    /** Primary line shown in the zone. */
    titleText: string;
    /** Optional secondary/hint line under the title. */
    underText?: string;
    /** Highlight the zone (e.g. while a file is dragged over it). */
    active?: boolean;
    /** Visual size: compact (`sm`) or a large centered area (`lg`). */
    size?: 'sm' | 'lg';
    /** Click handler (e.g. open a file picker). */
    onclick?: () => void;
    /** Bound to the root element so callers can hit-test drop positions. */
    element?: HTMLElement | null;
    /** Optional content rendered above the text (e.g. a thumbnail preview). */
    children?: Snippet;
  }

  let {
    titleText,
    underText,
    active = false,
    size = 'sm',
    onclick,
    element = $bindable(null),
    children,
  }: Props = $props();

  const sizeClass = $derived(
    size === 'lg'
      ? 'min-h-48 gap-3 rounded-2xl p-10 text-sm'
      : 'min-h-20 gap-1 rounded-lg p-3 text-xs',
  );
</script>

<button
  type="button"
  bind:this={element}
  aria-label={titleText}
  class={[
    'flex w-full cursor-pointer select-none flex-col items-center justify-center border border-dashed text-center transition-colors',
    sizeClass,
    active
      ? 'border-neutral-300 bg-white/10'
      : 'border-neutral-700 bg-neutral-800/40 hover:border-neutral-500',
  ]}
  {onclick}
>
  {#if children}{@render children()}{/if}
  <span class="text-neutral-300">{titleText}</span>
  {#if underText}<span class="text-neutral-500">{underText}</span>{/if}
</button>
