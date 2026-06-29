<script lang="ts">
  import { onMount } from 'svelte';
  import type { ImageRef, Placement, WatermarkRef } from '$lib/types';
  import { resolveRect, maxFitWidthFrac } from '$lib/placement';
  import {
    canvasPointToImage,
    pointInRect,
    dragToPlacement,
    resizeHandles,
    horizontalOf,
    verticalOf,
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
    /** Show the selection box, resize handles, and margin guides. */
    showGuides?: boolean;
  }

  let { image, watermark, placement, onchange, showGuides = true }: Props = $props();

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
  // measured relative to the anchor (the image center for centered anchors).
  let resizeRef = { x: 0, y: 0 };
  let resizeStartDist = 0;
  // Unit vector from the anchor toward the grabbed handle at drag start. The
  // pointer is *projected* onto this axis (a signed distance), so dragging past
  // the anchor keeps shrinking toward the minimum instead of growing again.
  let resizeDir = { x: 0, y: 0 };

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

  // Redraw whenever placement (slider or drag), base, watermark, or the guide
  // toggle changes.
  $effect(() => {
    void placement;
    void baseImg;
    void wmImg;
    void showGuides;
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
    if (showGuides) {
      drawMargin();
      drawSelection();
    }
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
    ctx.lineWidth = Math.max(0.5, canvas.width / 1500);
    ctx.setLineDash([8, 5]);
    ctx.strokeRect(r.x, r.y, r.w, r.h);

    // Resize handles on the corners that actually move: the ones not pinned to an
    // anchored edge (one for a corner anchor, two for an edge-centre, four for
    // dead-centre).
    ctx.setLineDash([]);
    const s = handleRadius();
    ctx.fillStyle = accent;
    for (const hd of resizeHandles(placement, r)) {
      ctx.fillRect(hd.x - s / 2, hd.y - s / 2, s, s);
    }
    ctx.restore();
  }

  // A minimal margin guide: one very thin dotted line per anchored dimension,
  // spanning the margin gap from the image's outer edge to the watermark's
  // anchored edge. No labels. Centered axes carry no margin, so they draw
  // nothing. Drawn in image space (outside the rotation).
  //
  // The gap is always exactly mx/my: `resolveRect` pins the watermark by its
  // rotated bounding box (half-extents that grow with rotation), so the rotated
  // box's anchored edge sits at mx/(iw-mx)/my/(ih-my) regardless of angle. Using
  // the unrotated box edge (r.x + r.w) here would overshoot once rotated, since
  // that edge lies inside the gap.
  function drawMargin() {
    if (!ctx || !watermark) return;
    const iw = canvas.width;
    const ih = canvas.height;
    const mRef = Math.min(iw, ih);
    const mx = placement.margin_x_frac * mRef;
    const my = placement.margin_y_frac * mRef;
    const h = horizontalOf(placement.anchor);
    const v = verticalOf(placement.anchor);
    const r = resolveRect(placement, iw, ih, wmAspect);
    const wmCx = r.x + r.w / 2;
    const wmCy = r.y + r.h / 2;

    ctx.save();
    ctx.strokeStyle = 'rgba(125,211,252,0.9)';
    ctx.lineWidth = Math.max(0.5, iw / 1500);
    ctx.setLineDash([4, 5]);
    if (h !== 'Center' && mx > 1) {
      line(h === 'Right' ? iw : 0, wmCy, h === 'Right' ? iw - mx : mx, wmCy);
    }
    if (v !== 'Middle' && my > 1) {
      line(wmCx, v === 'Bottom' ? ih : 0, wmCx, v === 'Bottom' ? ih - my : my);
    }
    ctx.restore();
  }

  function line(x0: number, y0: number, x1: number, y1: number) {
    if (!ctx) return;
    ctx.beginPath();
    ctx.moveTo(x0, y0);
    ctx.lineTo(x1, y1);
    ctx.stroke();
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

  // The resize handle under a local-frame pointer, or null. With several handles
  // (edge-centre / centre anchors) the nearest within grab range wins.
  function handleAt(lp: { x: number; y: number }, r: { x: number; y: number; w: number; h: number }) {
    const radius = handleRadius();
    return (
      resizeHandles(placement, r).find((hd) => dist(lp.x, lp.y, hd.x, hd.y) <= radius) ?? null
    );
  }

  function onPointerDown(e: PointerEvent) {
    if (!watermark) return;
    const p = toImage(e);
    const r = resolveRect(placement, canvas.width, canvas.height, wmAspect);
    const lp = toLocal(p, r);

    if (handleAt(lp, r)) {
      mode = 'resize';
      startWidthFrac = placement.width_frac;
      resizeRef = anchorPoint(placement, canvas.width, canvas.height);
      const dx = p.x - resizeRef.x;
      const dy = p.y - resizeRef.y;
      resizeStartDist = Math.hypot(dx, dy);
      resizeDir =
        resizeStartDist > 0
          ? { x: dx / resizeStartDist, y: dy / resizeStartDist }
          : { x: 0, y: 0 };
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
      const hovered = handleAt(lp, r);
      if (hovered) {
        // A handle sits on a corner; its diagonal cursor depends on which one.
        const isLeft = hovered.x < r.x + r.w / 2;
        const isTop = hovered.y < r.y + r.h / 2;
        canvas.style.cursor = isLeft === isTop ? 'nwse-resize' : 'nesw-resize';
      } else {
        canvas.style.cursor = pointInRect(lp.x, lp.y, r) ? 'move' : 'default';
      }
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
      // Signed distance along the drag axis: projecting onto resizeDir means
      // crossing the anchor flips the sign and clamps to the minimum, rather than
      // the watermark growing again on the far side.
      const d1 = (p.x - resizeRef.x) * resizeDir.x + (p.y - resizeRef.y) * resizeDir.y;
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
