<script lang="ts">
  // Snap rotation: two buttons that step the watermark by 90° (the primary,
  // intuitive control), sitting beside the position 9-grid. The compositor
  // accepts any angle, so finer rotation can be layered on later.
  interface Props {
    /** Current rotation in degrees. */
    value: number;
    onchange: (deg: number) => void;
    disabled?: boolean;
  }

  let { value, onchange, disabled = false }: Props = $props();

  // Snap to the nearest 90° step, rotate by `delta`, and wrap into [0, 360).
  function rotate(delta: number) {
    const snapped = Math.round(value / 90) * 90;
    onchange((((snapped + delta) % 360) + 360) % 360);
  }

  // Angle shown to the user, normalized onto the 0/90/180/270 grid.
  const display = $derived(((((Math.round(value / 90) * 90) % 360) + 360) % 360));

  const btn =
    'flex h-7 w-7 items-center justify-center rounded border transition-colors ' +
    'border-neutral-700 bg-neutral-800 text-neutral-300 ' +
    'hover:border-neutral-500 hover:text-neutral-100 disabled:opacity-40';
</script>

<div class="flex flex-col items-center gap-1.5">
  <div class="flex gap-1">
    <button
      type="button"
      {disabled}
      title="Rotate left 90°"
      aria-label="Rotate left 90°"
      class={btn}
      onclick={() => rotate(-90)}
    >
      <svg
        class="h-4 w-4"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        aria-hidden="true"
      >
        <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
        <path d="M3 3v5h5" />
      </svg>
    </button>
    <button
      type="button"
      {disabled}
      title="Rotate right 90°"
      aria-label="Rotate right 90°"
      class={btn}
      onclick={() => rotate(90)}
    >
      <svg
        class="h-4 w-4"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        aria-hidden="true"
      >
        <path d="M21 12a9 9 0 1 1-9-9 9.75 9.75 0 0 1 6.74 2.74L21 8" />
        <path d="M21 3v5h-5" />
      </svg>
    </button>
  </div>
  <span class="tabular-nums text-neutral-400">{display}°</span>
</div>
