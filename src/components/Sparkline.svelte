<script lang="ts">
  // A line of percentages, oldest on the left, so the app needs no chart
  // library. The box spans the smallest of a few round widths that fits the
  // values, from a multiple of 5 at or under the lowest, and says where it
  // starts and ends. A host idling at 2% CPU, or holding memory at 60%,
  // draws a line that moves instead of a flat one, without looking busy.
  import { percent } from "../lib/text";

  let { values, label }: { values: number[]; label: string } = $props();

  const SPANS = [5, 10, 20, 50, 100];

  const range = $derived.by(() => {
    const clamped = values.map((v) => Math.min(Math.max(v, 0), 100));
    const low = Math.floor(Math.min(100, ...clamped) / 5) * 5;
    const high = Math.max(0, ...clamped);
    const span = SPANS.find((s) => low + s >= high) ?? 100;
    const top = Math.min(low + span, 100);
    return { bottom: Math.max(top - span, 0), top };
  });

  const points = $derived(
    values
      .map((v, i) => {
        const x = values.length < 2 ? 100 : (i / (values.length - 1)) * 100;
        const { bottom, top } = range;
        const y = 30 - (Math.min(Math.max(v, bottom), top) - bottom) / (top - bottom) * 30;
        return `${x.toFixed(2)},${y.toFixed(2)}`;
      })
      .join(" "),
  );
</script>

<div>
  <svg viewBox="0 0 100 30" preserveAspectRatio="none" role="img" aria-label="{label}, {percent(range.bottom)} to {percent(range.top)}">
    <polyline {points} fill="none" stroke="currentColor" stroke-width="2" vector-effect="non-scaling-stroke" />
  </svg>
  <span class="top" aria-hidden="true">{percent(range.top)}</span>
  <span class="bottom" aria-hidden="true">{percent(range.bottom)}</span>
</div>

<style>
  div {
    position: relative;
  }
  svg {
    display: block;
    width: 100%;
    height: 56px;
    color: var(--accent);
    background: var(--surface);
    border-radius: 8px;
  }
  /* On a backing of their own, so a line running under them can't hide them. */
  span {
    position: absolute;
    left: 6px;
    padding: 0 3px;
    border-radius: 4px;
    color: var(--muted);
    background: var(--surface);
    font-size: 11px;
    line-height: 14px;
  }
  .top {
    top: 3px;
  }
  .bottom {
    bottom: 3px;
  }
</style>
