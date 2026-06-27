<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  type Variant = 'neutral' | 'accent' | 'success' | 'danger';
  type Size = 'md' | 'sm' | 'xs' | 'xxs';

  interface Props extends HTMLButtonAttributes {
    variant?: Variant;
    size?: Size;
    children: Snippet;
  }

  let {
    variant = 'neutral',
    size = 'md',
    type = 'button',
    class: className = '',
    children,
    ...rest
  }: Props = $props();

  const VARIANTS: Record<Variant, string> = {
    neutral: 'border-neutral-700 bg-neutral-800 hover:border-neutral-500',
    accent: 'border-neutral-300 bg-neutral-100 font-medium text-neutral-900 hover:bg-white',
    success: 'border-emerald-600 bg-emerald-600/20 text-emerald-200 hover:bg-emerald-600/30',
    danger: 'border-neutral-700 bg-neutral-800 hover:border-rose-500 hover:text-rose-300',
  };

  const SIZES: Record<Size, string> = {
    md: 'rounded-lg px-2.5 py-1.5',
    sm: 'rounded-lg px-2 py-1 text-xs',
    xs: 'rounded px-2 py-1 text-xs',
    xxs: 'rounded px-1.5 py-0.5 text-[11px]',
  };
</script>

<button
  {type}
  class="border disabled:opacity-40 {VARIANTS[variant]} {SIZES[size]} {className}"
  {...rest}
>
  {@render children()}
</button>
