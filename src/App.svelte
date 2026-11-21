<script lang="ts">
  // The app's frame: pairing until the link can connect, then hosts and
  // alerts, with a host's monitor on top. The prompt dialog is app-wide,
  // since a background monitor's prompts come whichever page is open.
  import { onMount } from "svelte";

  import LinkBar from "./components/LinkBar.svelte";
  import PromptDialog from "./components/PromptDialog.svelte";
  import { link } from "./lib/api";
  import { app, start } from "./lib/state.svelte";
  import type { Node, SessionId } from "./lib/types";
  import Alerts from "./screens/Alerts.svelte";
  import Hosts from "./screens/Hosts.svelte";
  import Monitor from "./screens/Monitor.svelte";
  import Pairing from "./screens/Pairing.svelte";
  import Terminal from "./screens/Terminal.svelte";

  const PAIRING = ["unpaired", "new_host_key", "key_refused", "host_key_changed"];

  let tab = $state<"hosts" | "alerts">("hosts");
  /// A host's monitor or a shell, over the lists.
  type Open = { node: Node; shell: false } | { node: Node; shell: true; session: SessionId | null };
  let open = $state<Open | null>(null);
  const pairing = $derived(PAIRING.includes(app.link.state));

  onMount(() => {
    start();
    // Android's back button and gesture leave the monitor for the list.
    const back = () => (open = null);
    window.addEventListener("popstate", back);
    return () => window.removeEventListener("popstate", back);
  });

  function show(next: Open) {
    history.pushState({}, "");
    open = next;
  }

  function forget() {
    if (confirm("Pair with another computer? This one will be forgotten.")) link.forget();
  }
</script>

{#if pairing}
  <Pairing />
{:else}
  <header>
    {#if open}
      <button class="plain" onclick={() => history.back()} aria-label="Back">‹ Hosts</button>
    {:else}
      <h1>Burrow</h1>
      <button class="plain" onclick={forget}>Pair again…</button>
    {/if}
  </header>
  <LinkBar />
  <main>
    {#if open?.shell}
      {#key open}<Terminal node={open.node} session={open.session} onclosed={() => history.back()} />{/key}
    {:else if open}
      {#key open.node.id}<Monitor node={open.node} />{/key}
    {:else if tab === "hosts"}
      <Hosts
        onopen={(node) => show({ node, shell: false })}
        onshell={(node, session) => show({ node, shell: true, session })}
      />
    {:else}
      <Alerts />
    {/if}
  </main>
  {#if !open}
    <nav>
      <button class:active={tab === "hosts"} onclick={() => (tab = "hosts")}>Hosts</button>
      <button class:active={tab === "alerts"} onclick={() => (tab = "alerts")}>
        Alerts{app.alerts.length ? ` (${app.alerts.length})` : ""}
      </button>
    </nav>
  {/if}
{/if}
<PromptDialog />

<style>
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: calc(env(safe-area-inset-top) + 12px) 16px 12px;
    border-bottom: 1px solid var(--line);
  }
  h1 {
    margin: 0;
    font-size: 20px;
  }
  main {
    flex: 1;
    overflow-y: auto;
  }
  nav {
    display: grid;
    grid-template-columns: 1fr 1fr;
    padding-bottom: env(safe-area-inset-bottom);
    border-top: 1px solid var(--line);
  }
  nav button {
    padding: 14px;
    border: 0;
    border-radius: 0;
    background: none;
    color: var(--muted);
  }
  nav button.active {
    color: var(--accent);
    font-weight: 600;
  }
</style>
