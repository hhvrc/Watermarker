<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    checked: boolean;
    children: Snippet;
  }

  let { checked = $bindable(), children }: Props = $props();
</script>

<label class="flex items-center gap-2 text-xs text-neutral-400">
  <input type="checkbox" bind:checked />
  {@render children()}
</label>
