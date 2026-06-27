<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  type Variant = 'neutral' | 'success' | 'error' | 'warning';

  interface Props extends HTMLAttributes<HTMLDivElement> {
    variant?: Variant;
    children: Snippet;
  }

  let { variant = 'neutral', class: className = '', children, ...rest }: Props = $props();

  const VARIANTS: Record<Variant, string> = {
    neutral: 'bg-neutral-200/90 font-medium text-neutral-900',
    success: 'bg-emerald-600/90 text-white',
    error: 'bg-rose-600/90 text-white',
    warning: 'bg-amber-600/90 text-white',
  };
</script>

<div class="rounded px-1 text-[9px] {VARIANTS[variant]} {className}" {...rest}>
  {@render children()}
</div>
