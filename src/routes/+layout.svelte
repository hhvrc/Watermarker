<script lang="ts">
  import '../app.css';
  import { onMount, onDestroy, type Snippet } from 'svelte';
  import Header from './Header.svelte';
  import Sidebar from './Sidebar.svelte';
  import DropOverlay from '$lib/components/DropOverlay.svelte';
  import { dnd } from '$lib/services/dnd.svelte';
  import { persistence } from '$lib/state/persistence.svelte';

  interface Props {
    children: Snippet;
  }

  let { children }: Props = $props();

  onMount(() => {
    void dnd.register();
    // Restore settings, then autosave on any later change.
    void persistence.load().then(() => persistence.start());
  });

  onDestroy(() => {
    dnd.dispose();
    persistence.flush();
  });
</script>

<div class="relative flex h-dvh flex-col bg-neutral-950 text-neutral-100">
  <Header />

  <main class="grid min-h-0 flex-1 [grid-template-columns:clamp(14rem,24vw,20rem)_1fr]">
    <Sidebar />
    {@render children()}
  </main>

  <DropOverlay />
</div>
