<script lang="ts">
  import Button from './ui/Button.svelte';
  import Checkbox from './ui/Checkbox.svelte';
  import { output } from '$lib/state/output.svelte';
</script>

<div class="select-none space-y-2 text-[13px] text-neutral-300">
  <span class="text-neutral-400">Output</span>
  <div class="flex items-center gap-2">
    <Button size="sm" onclick={output.pickDir}>Choose folder…</Button>
  </div>
  <p class="truncate text-[11px] text-neutral-500" title={output.settings.dir}>
    {output.settings.dir || 'No folder selected'}
  </p>

  <label class="flex items-center gap-2 text-xs text-neutral-400">
    <span class="w-16">Suffix</span>
    <input
      class="flex-1 select-text rounded border border-neutral-700 bg-neutral-800 px-2 py-1 text-neutral-100"
      bind:value={output.settings.suffix}
    />
  </label>
  <Checkbox bind:checked={output.settings.overwrite}>Overwrite existing files</Checkbox>
  <Checkbox bind:checked={output.settings.strip_metadata}>Remove all metadata (privacy)</Checkbox>
  <Checkbox bind:checked={output.settings.export_clean}>Also export a non-watermarked copy</Checkbox
  >
</div>
