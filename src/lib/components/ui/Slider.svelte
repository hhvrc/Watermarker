<script lang="ts">
  interface Props {
    label: string;
    value: number;
    min: number;
    max: number;
    step?: number;
    disabled?: boolean;
    /** Unit shown after the value in the label, e.g. `%` or `% of width`. */
    suffix?: string;
  }

  let {
    label,
    value = $bindable(),
    min,
    max,
    step = 1,
    disabled = false,
    suffix = '%',
  }: Props = $props();
</script>

<label class="flex flex-col gap-1">
  <span class="select-none text-neutral-400">{label} ({value}{suffix})</span>
  <input type="range" {min} {max} {step} {disabled} bind:value />
</label>
