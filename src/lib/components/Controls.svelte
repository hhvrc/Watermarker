<script lang="ts">
  import AnchorGrid from './AnchorGrid.svelte';
  import RotationControl from './RotationControl.svelte';
  import Slider from './ui/Slider.svelte';
  import { maxFitWidthFrac } from '$lib/placement';
  import type { Anchor, Placement } from '$lib/types';

  interface Props {
    placement: Placement;
    onchange: (p: Placement) => void;
    /** Current image dimensions and watermark aspect, for best-fit sizing. */
    imgW?: number;
    imgH?: number;
    wmAspect?: number;
    disabled?: boolean;
  }

  let { placement, onchange, imgW, imgH, wmAspect = 1, disabled = false }: Props = $props();

  function set(patch: Partial<Placement>) {
    onchange({ ...placement, ...patch });
  }

  // Largest width_frac that still fits the (rotated) watermark in the image, so
  // "Size" reads as a percentage of best fit rather than of raw image width.
  const maxFit = $derived(maxFitWidthFrac(imgW ?? 0, imgH ?? 0, wmAspect, placement));
  // The watermark's current fill of that best fit, in 0..1.
  const fitFrac = $derived(maxFit > 0 ? placement.width_frac / maxFit : 0);

  /** Rotate while preserving the fill ratio, so rotation never overflows. */
  function setRotation(deg: number) {
    const keep = Math.min(1, fitFrac);
    const newMax = maxFitWidthFrac(imgW ?? 0, imgH ?? 0, wmAspect, { ...placement, rot_deg: deg });
    onchange({ ...placement, rot_deg: deg, width_frac: keep * newMax });
  }

  /** Switch anchor, shrinking the watermark only if the new anchor affords less
   * room (a margin-bearing edge vs a centered band) so it can't clip the edge. */
  function setAnchor(anchor: Anchor) {
    const next = { ...placement, anchor };
    const newMax = maxFitWidthFrac(imgW ?? 0, imgH ?? 0, wmAspect, next);
    onchange({ ...next, width_frac: Math.min(placement.width_frac, newMax) });
  }

  /** Set the uniform margin, shrinking the watermark only if it would overflow
   * the reduced best fit (a larger margin leaves less room to grow into). */
  function setMargin(frac: number) {
    const newMax = maxFitWidthFrac(imgW ?? 0, imgH ?? 0, wmAspect, {
      ...placement,
      margin_x_frac: frac,
      margin_y_frac: frac,
    });
    onchange({
      ...placement,
      margin_x_frac: frac,
      margin_y_frac: frac,
      width_frac: Math.min(placement.width_frac, newMax),
    });
  }

  // v1 exposes a single uniform margin controlling both axes.
  const marginPct = $derived(Math.round(placement.margin_x_frac * 100));
  const sizePct = $derived(Math.min(100, Math.round(fitFrac * 100)));
  const opacityPct = $derived(Math.round(placement.opacity * 100));
</script>

<div class="space-y-4 text-[13px] text-neutral-300">
  <div class="flex items-start gap-5">
    <div class="space-y-1.5">
      <span class="block text-neutral-400">Position</span>
      <AnchorGrid value={placement.anchor} {disabled} onchange={setAnchor} />
    </div>
    <div class="space-y-1.5">
      <span class="block text-neutral-400">Rotation</span>
      <RotationControl value={placement.rot_deg} {disabled} onchange={setRotation} />
    </div>
  </div>

  <Slider
    label="Margin"
    bind:value={() => marginPct, (v) => setMargin(v / 100)}
    min={0}
    max={25}
    {disabled}
  />

  <Slider
    label="Size"
    bind:value={() => sizePct, (v) => set({ width_frac: (v / 100) * maxFit })}
    min={2}
    max={100}
    suffix="% of fit"
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
