<script lang="ts">
  import Controls from '$lib/components/Controls.svelte';
  import Presets from '$lib/components/Presets.svelte';
  import QueuePane from '$lib/components/QueuePane.svelte';
  import VrcMetadataPanel from '$lib/components/VrcMetadataPanel.svelte';
  import WatermarkPanel from '$lib/components/WatermarkPanel.svelte';
  import OutputPanel from '$lib/components/OutputPanel.svelte';
  import { queue } from '$lib/state/queue.svelte';
  import { watermark } from '$lib/state/watermark.svelte';
  import { placement } from '$lib/state/placement.svelte';
  import { presets } from '$lib/state/presets.svelte';
  import { workspace } from '$lib/state/workspace.svelte';
  import { status } from '$lib/state/status.svelte';
</script>

<aside class="min-h-0 space-y-5 overflow-auto border-r border-neutral-800 bg-neutral-900 p-3">
  <WatermarkPanel />

  <hr class="border-neutral-800" />

  {#if workspace.mode === 'edit'}
    <Controls
      placement={placement.current}
      onchange={placement.update}
      imgW={queue.current?.width}
      imgH={queue.current?.height}
      wmAspect={watermark.ref ? watermark.ref.width / watermark.ref.height : 1}
      disabled={!queue.current || !watermark.ref}
    />
    <hr class="border-neutral-800" />
    <Presets
      presets={presets.list}
      canSave={presets.canSave}
      onapply={presets.apply}
      onsave={presets.save}
      ondelete={presets.remove}
    />
    <hr class="border-neutral-800" />
  {/if}

  <QueuePane
    images={queue.images}
    index={queue.index}
    onselect={workspace.editImage}
    onremove={queue.removeAt}
    onclear={workspace.clearQueue}
  />

  {#if queue.current?.vrchat}
    <hr class="border-neutral-800" />
    <VrcMetadataPanel meta={queue.current.vrchat} />
  {/if}

  {#if workspace.mode === 'review'}
    <hr class="border-neutral-800" />
    <OutputPanel />
  {/if}

  {#if status.text}
    <p class="select-none text-xs text-neutral-400">{status.text}</p>
  {/if}
</aside>
