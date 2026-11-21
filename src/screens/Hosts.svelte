<script lang="ts">
  // The SSH hosts the daemon knows, with a badge for each one's alerts,
  // and the shells the phone keeps on them.
  import Shells from "../components/Shells.svelte";
  import { alertsFor, app, hosts } from "../lib/state.svelte";
  import { clean } from "../lib/text";
  import type { Node, SessionId } from "../lib/types";

  let {
    onopen,
    onshell,
  }: { onopen: (node: Node) => void; onshell: (node: Node, session: SessionId | null) => void } =
    $props();
</script>

<Shells onopen={(node, t) => onshell(node, t.session)} />
<ul>
  {#each hosts() as node (node.id)}
    {@const alerts = alertsFor(node.id).length}
    <li>
      <button onclick={() => onopen(node)}>
        <span class="name">{clean(node.name)}</span>
        {#if app.monitoring.includes(node.id)}<span class="tag">Monitored</span>{/if}
        {#if alerts}<span class="badge" aria-label="{alerts} alerts">{alerts}</span>{/if}
      </button>
      <button class="plain shell" onclick={() => onshell(node, null)}>New shell</button>
    </li>
  {:else}
    <li class="empty">No SSH hosts in Burrow on this computer yet.</li>
  {/each}
</ul>

<style>
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
  li button:not(.shell) {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 8px;
    padding: 16px;
    border: 0;
    border-radius: 0;
    background: none;
    text-align: left;
  }
  .shell {
    padding: 16px;
  }
  .name {
    flex: 1;
    font-size: 17px;
  }
  .tag {
    color: var(--muted);
    font-size: 13px;
  }
  .badge {
    min-width: 22px;
    padding: 2px 7px;
    border-radius: 11px;
    background: var(--danger);
    color: white;
    font-size: 13px;
    text-align: center;
  }
  .empty {
    padding: 24px 16px;
    color: var(--muted);
  }
</style>
