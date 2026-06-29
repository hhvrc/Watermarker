<script lang="ts">
  import { getVersion } from '@tauri-apps/api/app';
  import Button from '$lib/components/ui/Button.svelte';
  import { queue } from '$lib/state/queue.svelte';
  import { workspace } from '$lib/state/workspace.svelte';
  import { status } from '$lib/state/status.svelte';

  let version = $state('');
  void getVersion().then((v) => (version = v));
</script>

<header
  class="flex items-center justify-between gap-3 border-b border-neutral-800 bg-neutral-900/60 px-3 py-2"
>
  <div class="flex items-baseline gap-2">
    <h1 class="text-sm font-semibold">Batch Watermarker</h1>
    {#if version}
      <span class="text-xs text-neutral-500">v{version}</span>
    {/if}
  </div>

  <div class="flex items-center gap-2 text-xs">
    {#if workspace.mode === 'edit'}
      <Button
        variant="accent"
        onclick={workspace.stage}
        disabled={!workspace.canStage || status.busy || workspace.processing}
      >
        Stage ({queue.images.length})
      </Button>
    {:else}
      <Button onclick={workspace.toEdit} disabled={workspace.processing}>Back to editing</Button>
      <Button variant="success" onclick={workspace.commit} disabled={!workspace.canCommit}>
        {workspace.processing
          ? `Saving ${workspace.progress.completed}/${workspace.progress.total}…`
          : 'Commit batch'}
      </Button>
    {/if}
  </div>
</header>
