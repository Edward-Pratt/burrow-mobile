<script lang="ts">
  // The shells the phone keeps running, above the hosts: open one again,
  // or end it.
  import { daemon } from "../lib/api";
  import { app } from "../lib/state.svelte";
  import { clean } from "../lib/text";
  import type { Node, SessionInfo } from "../lib/types";

  let { onopen }: { onopen: (node: Node, terminal: SessionInfo) => void } = $props();

  const named = $derived(
    app.terminals.flatMap((t) => {
      const node = app.nodes.find((n) => n.id === t.node);
      return node ? [{ t, node }] : [];
    }),
  );

  function end(t: SessionInfo, node: Node) {
    if (confirm(`End this shell on ${clean(node.name)}?`)) daemon.closeSession(t.session).catch(() => {});
  }
</script>

{#if named.length}
  <h2>Shells</h2>
  <ul>
    {#each named as { t, node } (t.session)}
      <li>
        <button class="open" onclick={() => onopen(node, t)}>
          <span class="name">{clean(node.name)}</span>
          <span class="tag">{t.attached ? "Open" : "Running"}</span>
        </button>
        <button class="plain" onclick={() => end(t, node)}>End…</button>
      </li>
    {/each}
  </ul>
{/if}

<style>
  h2 {
    margin: 0;
    padding: 12px 16px 4px;
    color: var(--muted);
    font-size: 13px;
    font-weight: 600;
  }
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: flex;
    align-items: center;
    border-bottom: 1px solid var(--line);
  }
  .open {
    display: flex;
    flex: 1;
    gap: 8px;
    padding: 14px 16px;
    border: 0;
    border-radius: 0;
    background: none;
    text-align: left;
  }
  .name {
    flex: 1;
    font-size: 17px;
  }
  .tag {
    color: var(--muted);
    font-size: 13px;
  }
  li .plain {
    padding: 14px 16px;
  }
</style>
