<script lang="ts">
  import Button from '$lib/components/ui/Button.svelte';
  import { updater } from '$lib/state/updater.svelte';

  // Whether the "No thanks" decline options (skip / never) are revealed.
  let declining = $state(false);

  // Reset back to the primary view whenever the dialog closes.
  $effect(() => {
    if (!updater.visible) declining = false;
  });

  function close() {
    if (!updater.installing) updater.remindLater();
  }
</script>

<svelte:window
  onkeydown={(e) => {
    if (e.key === 'Escape') close();
  }}
/>

{#if updater.visible}
  <!-- Backdrop. Click outside the dialog dismisses, unless installing. -->
  <div
    class="absolute inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget) close();
    }}
  >
    <div
      class="w-full max-w-sm select-none rounded-xl border border-neutral-700 bg-neutral-900 p-5 shadow-2xl"
      role="dialog"
      aria-modal="true"
      aria-labelledby="update-title"
      tabindex="-1"
    >
      {#if declining}
        <h2 id="update-title" class="text-sm font-semibold text-neutral-100">
          Dismiss this update?
        </h2>

        <p class="mt-2 text-[13px] text-neutral-300">
          How long should we hide the <span class="font-medium">v{updater.version}</span> update?
        </p>
        <ul class="mt-3 space-y-1.5 text-xs text-neutral-400">
          <li>
            <span class="text-neutral-200">Skip this version</span> — stay quiet about v{updater.version},
            but tell me about newer releases.
          </li>
          <li>
            <span class="text-neutral-200">Never show again</span> — turn off update notifications completely.
          </li>
        </ul>
      {:else}
        <h2 id="update-title" class="text-sm font-semibold text-neutral-100">Update available</h2>

        <p class="mt-2 text-[13px] text-neutral-300">
          Watermarker <span class="font-medium">v{updater.version}</span> is available.
          {#if updater.current}You're on v{updater.current}.{/if}
        </p>

        {#if updater.notes}
          <div
            class="mt-3 max-h-40 select-text overflow-auto whitespace-pre-wrap rounded-lg border border-neutral-800 bg-neutral-950 p-2.5 text-xs text-neutral-400"
          >
            {updater.notes}
          </div>
        {/if}
      {/if}

      {#if updater.error}
        <p class="mt-3 text-xs text-rose-300">{updater.error}</p>
      {/if}

      {#if updater.installing}
        <div class="mt-5 flex items-center justify-end">
          <Button size="sm" variant="accent" disabled>Installing…</Button>
        </div>
      {:else if declining}
        <!-- Revealed by "No thanks": how long to keep this update hidden. -->
        <div class="mt-5 flex items-center justify-end gap-2">
          <Button size="sm" onclick={updater.skipVersion}>Skip this version</Button>
          <Button size="sm" onclick={updater.disable}>Never show again</Button>
        </div>
      {:else}
        <div class="mt-5 flex items-center justify-end gap-2">
          <Button size="sm" onclick={() => (declining = true)}>No thanks</Button>
          <Button size="sm" variant="accent" onclick={updater.install}>Update &amp; restart</Button>
        </div>
      {/if}
    </div>
  </div>
{/if}
