<script lang="ts">
  // What is wrong now on the hosts monitored in the background, newest
  // first. Alerts come and go live as the daemon raises and clears them.
  import { describe } from "../lib/alerts";
  import { app } from "../lib/state.svelte";
  import { ago, clean } from "../lib/text";

  const alerts = $derived([...app.alerts].sort((a, b) => b.since - a.since));
  const name = (id: string) => clean(app.nodes.find((n) => n.id === id)?.name ?? "A host");
</script>

<ul>
  {#each alerts as alert (alert.id)}
    <li>
      <span class="host">{name(alert.node)}</span>
      <span>{describe(alert)}</span>
      <span class="when">{ago(alert.since)}</span>
    </li>
  {:else}
    <li class="empty">Nothing is wrong on the hosts Burrow monitors in the background.</li>
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
    flex-direction: column;
    gap: 2px;
    padding: 14px 16px;
    border-bottom: 1px solid var(--line);
  }
  .host {
    font-weight: 600;
  }
  .when {
    color: var(--muted);
    font-size: 13px;
  }
  .empty {
    color: var(--muted);
    border: 0;
  }
</style>
