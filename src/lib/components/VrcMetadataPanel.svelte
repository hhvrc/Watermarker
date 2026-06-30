<script lang="ts">
  import type { VrcMetadata } from '$lib/types';

  interface Props {
    meta: VrcMetadata;
  }

  let { meta }: Props = $props();

  // The fields to show, in order, filtering out any that are absent.
  const rows = $derived(
    (
      [
        ['World', meta.world_name],
        ['Photographer', meta.author_name],
        ['Captured', meta.captured_at],
      ] as const
    ).filter(([, value]) => value),
  );
</script>

{#if rows.length > 0}
  <div class="select-none space-y-2 text-[13px]">
    <div class="flex items-center gap-1.5 text-neutral-400">
      <span>VRChat</span>
      <span class="rounded bg-sky-500/20 px-1 text-[9px] font-medium text-sky-300 uppercase">
        {meta.source_format}
      </span>
    </div>
    <dl class="space-y-1">
      {#each rows as [label, value] (label)}
        <div class="flex items-baseline gap-2">
          <dt class="w-20 shrink-0 text-[11px] text-neutral-500">{label}</dt>
          <dd class="min-w-0 break-words text-neutral-200" title={value}>{value}</dd>
        </div>
      {/each}
    </dl>
  </div>
{/if}
