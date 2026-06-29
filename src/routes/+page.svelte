<script lang="ts">
  import PreviewCanvas from '$lib/components/PreviewCanvas.svelte';
  import ReviewGrid from '$lib/components/ReviewGrid.svelte';
  import DropZone from '$lib/components/DropZone.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import { queue } from '$lib/state/queue.svelte';
  import { watermark } from '$lib/state/watermark.svelte';
  import { placement } from '$lib/state/placement.svelte';
  import { workspace } from '$lib/state/workspace.svelte';
  import { dnd } from '$lib/services/dnd.svelte';

  // Local alias so the {#if current} block narrows away the null case.
  const current = $derived(queue.current);

  // Toggle the on-image editing overlay (selection box, resize handles, margin
  // guides) so the watermark can be previewed clean. Persists across image
  // switches since this page isn't remounted.
  let showGuides = $state(true);

  // Freeze the current image's placement before moving, so an image still
  // inheriting the sticky default doesn't drift when a later image is edited.
  function prev() {
    placement.commitCurrent();
    queue.prev();
  }
  function next() {
    placement.commitCurrent();
    queue.next();
  }
</script>

<section class="flex min-h-0 select-none flex-col overflow-hidden p-3">
  {#if workspace.mode === 'edit'}
    {#if current}
      <div class="mb-2 flex items-center justify-between gap-2 text-xs text-neutral-300">
        <span class="truncate" title={current.name}>
          Image {queue.index + 1} / {queue.images.length}: {current.name}
        </span>
        <div class="flex items-center gap-1.5">
          <Button
            onclick={() => (showGuides = !showGuides)}
            aria-pressed={showGuides}
            title="Show or hide the editing overlay: selection box, resize handles, and margin guides"
          >
            <span class="inline-flex items-center gap-1.5">
              <span
                class="h-1.5 w-1.5 rounded-full {showGuides ? 'bg-sky-400' : 'bg-neutral-600'}"
              ></span>
              Guides
            </span>
          </Button>
          <div class="mx-1 h-4 w-px bg-neutral-700"></div>
          <Button onclick={prev} disabled={queue.index <= 0}>← Prev</Button>
          <Button onclick={next} disabled={queue.index >= queue.images.length - 1}>Next →</Button>
        </div>
      </div>
      <div class="min-h-0 flex-1">
        {#key current.id}
          <PreviewCanvas
            image={current}
            watermark={watermark.ref}
            placement={placement.current}
            onchange={placement.update}
            {showGuides}
          />
        {/key}
      </div>
    {:else}
      <div class="flex h-full items-center justify-center p-4">
        <div class="w-full max-w-md">
          <DropZone
            size="lg"
            active={dnd.showOverlay}
            onclick={queue.loadFromPicker}
            titleText="Drag & drop images here"
            underText="or click to choose images"
          />
        </div>
      </div>
    {/if}
  {:else}
    <ReviewGrid images={queue.images} urls={workspace.reviewUrls} onedit={workspace.editImage} />
  {/if}
</section>
