<script lang="ts">
  import AnchorGrid from './AnchorGrid.svelte';
  import Slider from './ui/Slider.svelte';
  import type { Anchor, Placement } from '$lib/types';

  interface Props {
    placement: Placement;
    onchange: (p: Placement) => void;
    disabled?: boolean;
  }

  let { placement, onchange, disabled = false }: Props = $props();

  function set(patch: Partial<Placement>) {
    onchange({ ...placement, ...patch });
  }

  // v1 exposes a single uniform margin controlling both axes.
  const marginPct = $derived(Math.round(placement.margin_x_frac * 100));
  const sizePct = $derived(Math.round(placement.width_frac * 100));
  const opacityPct = $derived(Math.round(placement.opacity * 100));
</script>

<div class="space-y-4 text-[13px] text-neutral-300">
  <div class="space-y-1.5">
    <span class="text-neutral-400">Position</span>
    <AnchorGrid value={placement.anchor} {disabled} onchange={(a: Anchor) => set({ anchor: a })} />
  </div>

  <Slider
    label="Margin"
    bind:value={() => marginPct, (v) => set({ margin_x_frac: v / 100, margin_y_frac: v / 100 })}
    min={0}
    max={25}
    {disabled}
  />

  <Slider
    label="Size"
    bind:value={() => sizePct, (v) => set({ width_frac: v / 100 })}
    min={2}
    max={100}
    suffix="% of width"
    {disabled}
  />

  <Slider
    label="Opacity"
    bind:value={() => opacityPct, (v) => set({ opacity: v / 100 })}
    min={5}
    max={100}
    {disabled}
  />
</div>
