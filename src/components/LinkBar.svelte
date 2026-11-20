<script lang="ts">
  // Where the connection stands, while it isn't simply up.
  import { link } from "../lib/api";
  import { app } from "../lib/state.svelte";
  import { clean } from "../lib/text";
</script>

{#if app.link.state === "connecting"}
  <div class="bar">Connecting…</div>
{:else if app.link.state === "failed"}
  <div class="bar failed">
    <span>{clean(app.link.error)}. Trying again in {app.link.retry_in} s.</span>
    <button onclick={() => link.reconnect()}>Try now</button>
  </div>
{/if}
{#if app.error}
  <div class="bar failed">{clean(app.error)}</div>
{/if}

<style>
  .bar {
    display: flex;
    gap: 12px;
    align-items: center;
    justify-content: space-between;
    padding: 10px 16px;
    background: var(--surface);
    font-size: 14px;
  }
  .failed {
    color: var(--danger);
  }
</style>
