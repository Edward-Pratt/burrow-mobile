<script lang="ts">
  // A line of values from 0 to `max`, oldest on the left, so the app needs
  // no chart library.
  let { values, max = 100, label }: { values: number[]; max?: number; label: string } = $props();

  const points = $derived(
    values
      .map((v, i) => {
        const x = values.length < 2 ? 100 : (i / (values.length - 1)) * 100;
        const y = 30 - Math.min(Math.max(v, 0) / (max || 1), 1) * 30;
        return `${x.toFixed(2)},${y.toFixed(2)}`;
      })
      .join(" "),
  );
</script>

<svg viewBox="0 0 100 30" preserveAspectRatio="none" role="img" aria-label={label}>
  <polyline {points} fill="none" stroke="currentColor" stroke-width="2" vector-effect="non-scaling-stroke" />
</svg>

<style>
  svg {
    display: block;
    width: 100%;
    height: 56px;
    color: var(--accent);
    background: var(--surface);
    border-radius: 8px;
  }
</style>
