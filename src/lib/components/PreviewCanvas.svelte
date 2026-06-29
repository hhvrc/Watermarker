<script lang="ts">
  import { onMount } from 'svelte';
  import type { ImageRef, Placement, WatermarkRef } from '$lib/types';
  import { resolveRect, maxFitWidthFrac } from '$lib/placement';
  import {
    canvasPointToImage,
    pointInRect,
    dragToPlacement,
    corners,
    nearCorner,
    dist,
    resizeWidthFrac,
    rotatePoint,
    anchorPoint,
    MIN_WIDTH_FRAC,
  } from '$lib/placementMap';

  interface Props {
    image: ImageRef;
    watermark: WatermarkRef | null;
    placement: Placement;
    onchange: (p: Placement) => void;
  }

  let { image, watermark, placement, onchange }: Props = $props();

  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D | null = null;

  let baseImg: HTMLImageElement | null = $state(null);
  let wmImg: HTMLImageElement | null = $state(null);

  type Mode = 'none' | 'move' | 'resize';
  let mode: Mode = 'none';
  let startBox = { x: 0, y: 0, w: 0, h: 0 };
  let startPointer = { x: 0, y: 0 };
  let startWidthFrac = 0;
  // Fixed reference for a resize drag: the placement's anchor point. Scaling is
  // always measured relative to the anchor (the image center for centered
  // anchors), so it grows monotonically from there.
  let resizeRef = { x: 0, y: 0 };
  let resizeStartDist = 0;

  const wmAspect = $derived(watermark ? watermark.width / watermark.height : 1);

  function handleRadius() {
    return Math.max(8, canvas.width / 90);
  }

  function loadImg(url: string): Promise<HTMLImageElement> {
    return new Promise((resolve, reject) => {
      const i = new Image();
      i.onload = () => resolve(i);
      i.onerror = reject;
      i.src = url;
    });
  }

  // (Re)load the base preview when the selected image changes.
  $effect(() => {
    const url = image.previewUrl;
    if (!url) {
      baseImg = null;
      return;
    }
    let alive = true;
    loadImg(url).then((i) => {
      if (!alive) return;
      baseImg = i;
      sizeCanvas();
      draw();
    });
    return () => {
      alive = false;
    };
  });

  // (Re)load the watermark preview when it changes.
  $effect(() => {
    const url = watermark?.previewUrl;
    if (!url) {
      wmImg = null;
      draw();
      return;
    }
    let alive = true;
    loadImg(url).then((i) => {
      if (!alive) return;
      wmImg = i;
      draw();
    });
    return () => {
      alive = false;
    };
  });

  // Redraw whenever placement (slider or drag), base, or watermark changes.
  $effect(() => {
    void placement;
    void baseImg;
    void wmImg;
    draw();
  });

  function sizeCanvas() {
    if (!canvas || !baseImg) return;
    canvas.width = baseImg.naturalWidth;
    canvas.height = baseImg.naturalHeight;
  }

  function draw() {
    if (!ctx || !canvas) return;
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    if (!baseImg) return;
    ctx.drawImage(baseImg, 0, 0, canvas.width, canvas.height);
    drawWatermark();
    drawSelection();
  }

  function drawWatermark() {
    if (!wmImg || !ctx) return;
    const r = resolveRect(placement, canvas.width, canvas.height, wmAspect);
    ctx.save();
    ctx.globalAlpha = placement.opacity;
    ctx.translate(r.x + r.w / 2, r.y + r.h / 2);
    ctx.rotate((placement.rot_deg * Math.PI) / 180);
    ctx.drawImage(wmImg, -r.w / 2, -r.h / 2, r.w, r.h);
    ctx.restore();
  }

  function drawSelection() {
    if (!watermark || !ctx) return;
    const r = resolveRect(placement, canvas.width, canvas.height, wmAspect);
    const accent = 'rgba(229,229,229,0.95)';
    ctx.save();
    // Rotate the outline and handles about the box center so the selection
    // tracks the (rotated) watermark.
    ctx.translate(r.x + r.w / 2, r.y + r.h / 2);
    ctx.rotate((placement.rot_deg * Math.PI) / 180);
    ctx.translate(-(r.x + r.w / 2), -(r.y + r.h / 2));

    ctx.strokeStyle = accent;
    ctx.lineWidth = Math.max(1, canvas.width / 600);
    ctx.setLineDash([8, 5]);
    ctx.strokeRect(r.x, r.y, r.w, r.h);

    // Corner resize handles.
    ctx.setLineDash([]);
    const s = handleRadius();
    ctx.fillStyle = accent;
    for (const c of corners(r)) {
      ctx.fillRect(c.x - s / 2, c.y - s / 2, s, s);
    }
    ctx.restore();
  }

  // Map an image-space pointer into the watermark's local (unrotated) frame so
  // the axis-aligned hit-tests work when the box is rotated.
  function toLocal(p: { x: number; y: number }, r: { x: number; y: number; w: number; h: number }) {
    if (!placement.rot_deg) return p;
    return rotatePoint(p.x, p.y, r.x + r.w / 2, r.y + r.h / 2, -placement.rot_deg);
  }

  function toImage(e: PointerEvent) {
    const rect = canvas.getBoundingClientRect();
    return canvasPointToImage(e.clientX, e.clientY, rect, canvas.width, canvas.height);
  }

  function onPointerDown(e: PointerEvent) {
    if (!watermark) return;
    const p = toImage(e);
    const r = resolveRect(placement, canvas.width, canvas.height, wmAspect);
    const lp = toLocal(p, r);

    if (nearCorner(lp.x, lp.y, r, handleRadius())) {
      mode = 'resize';
      startWidthFrac = placement.width_frac;
      resizeRef = anchorPoint(placement, canvas.width, canvas.height);
      resizeStartDist = dist(p.x, p.y, resizeRef.x, resizeRef.y);
    } else if (pointInRect(lp.x, lp.y, r)) {
      mode = 'move';
      startBox = r;
      startPointer = p;
    } else {
      return;
    }
    canvas.setPointerCapture(e.pointerId);
  }

  function onPointerMove(e: PointerEvent) {
    const p = toImage(e);
    if (mode === 'none') {
      if (!watermark) return;
      const r = resolveRect(placement, canvas.width, canvas.height, wmAspect);
      const lp = toLocal(p, r);
      canvas.style.cursor = nearCorner(lp.x, lp.y, r, handleRadius())
        ? 'nwse-resize'
        : pointInRect(lp.x, lp.y, r)
          ? 'move'
          : 'default';
      return;
    }
    if (mode === 'move') {
      const next = dragToPlacement(
        placement,
        startBox,
        p.x - startPointer.x,
        p.y - startPointer.y,
        canvas.width,
        canvas.height,
      );
      // The new anchor may afford less room (a margin-bearing edge vs a centered
      // band), so re-clamp width to the new best fit so it can't clip the edge.
      const maxFit = maxFitWidthFrac(canvas.width, canvas.height, wmAspect, next);
      onchange({ ...next, width_frac: Math.min(next.width_frac, maxFit) });
    } else {
      const d1 = dist(p.x, p.y, resizeRef.x, resizeRef.y);
      // Cap growth at best fit so the (rotated) watermark can't be dragged past
      // the binding image edge.
      const maxFit = maxFitWidthFrac(canvas.width, canvas.height, wmAspect, placement);
      onchange({
        ...placement,
        width_frac: resizeWidthFrac(startWidthFrac, resizeStartDist, d1, MIN_WIDTH_FRAC, maxFit),
      });
    }
  }

  function onPointerUp(e: PointerEvent) {
    if (mode === 'none') return;
    mode = 'none';
    try {
      canvas.releasePointerCapture(e.pointerId);
    } catch {
      /* already released */
    }
  }

  onMount(() => {
    ctx = canvas.getContext('2d');
    sizeCanvas();
    draw();
  });
</script>

<div class="flex h-full w-full items-center justify-center">
  <canvas
    bind:this={canvas}
    class="block max-h-full max-w-full touch-none rounded-lg bg-black object-contain"
    style="cursor: {watermark ? 'move' : 'default'}"
    onpointerdown={onPointerDown}
    onpointermove={onPointerMove}
    onpointerup={onPointerUp}
    onpointercancel={onPointerUp}
  ></canvas>
</div>
