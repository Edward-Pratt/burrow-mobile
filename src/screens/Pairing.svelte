<script lang="ts">
  // Pairing with the computer that runs burrowd: the phone's key line to
  // add there, then the computer's host key to confirm. Also where a
  // refused key or a changed host key lands, since only the user can move
  // those on.
  import { link } from "../lib/api";
  import { app } from "../lib/state.svelte";
  import { clean } from "../lib/text";
  import type { Paired } from "../lib/types";

  let address = $state("");
  let port = $state(22);
  let user = $state("");
  let line = $state("");
  let error = $state<string | null>(null);
  let copied = $state(false);

  // A pairing in place (key refused, host key to check) shows its user's line.
  $effect(() => {
    if (app.link.state === "unpaired") return;
    link.paired().then((p: Paired | null) => {
      if (p) ({ address, port, user } = p);
    });
  });

  $effect(() => {
    const name = user;
    copied = false;
    if (!name) return void (line = "");
    link.keyLine(name).then(
      (l) => ((line = l), (error = null)),
      () => (line = ""),
    );
  });

  async function act(f: () => Promise<void>) {
    try {
      error = null;
      await f();
    } catch (e) {
      error = String(e);
    }
  }

  async function copy() {
    await navigator.clipboard.writeText(line);
    copied = true;
  }

  function forget() {
    if (confirm("Forget this computer? The phone will need pairing again.")) act(link.forget);
  }
</script>

<section>
  {#if app.link.state === "unpaired" || app.link.state === "key_refused"}
    <h1>{app.link.state === "unpaired" ? "Pair with your computer" : "The computer refused this phone"}</h1>
    {#if app.link.state === "unpaired"}
      <p>The computer that runs Burrow, reachable from this phone, for example over Tailscale.</p>
      <label>Address <input bind:value={address} placeholder="warren" autocapitalize="off" /></label>
      <label>Port <input type="number" bind:value={port} min="1" max="65535" /></label>
    {/if}
    <label>Your user there <input bind:value={user} autocapitalize="off" autocomplete="off" /></label>
    {#if line}
      <p>
        Add this line to <code>~/.ssh/authorized_keys</code> for {user} on the computer. It lets this
        phone run Burrow there, and nothing else: no shell and no forwarding.
      </p>
      <pre>{line}</pre>
      <button onclick={copy}>{copied ? "Copied" : "Copy the line"}</button>
    {/if}
    {#if app.link.state === "unpaired"}
      <button class="primary" disabled={!address || !user} onclick={() => act(() => link.pair(address, port, user))}>
        Pair
      </button>
    {:else}
      <button class="primary" onclick={() => act(link.reconnect)}>Try again</button>
      <button onclick={forget}>Pair with another computer…</button>
    {/if}
  {:else if app.link.state === "new_host_key"}
    <h1>Check the computer's host key</h1>
    <p>On {address || "the computer"}, run:</p>
    <pre>ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub</pre>
    <p>It should show this fingerprint. If it doesn't, something may be in between.</p>
    <pre class="fingerprint">{clean(app.link.fingerprint)}</pre>
    {@const key = app.link.key}
    <button class="primary" onclick={() => act(() => link.pin(key))}>It matches</button>
    <button onclick={forget}>Pair with another computer…</button>
  {:else if app.link.state === "host_key_changed"}
    <h1>The computer's host key changed</h1>
    <p class="danger">
      Burrow won't connect. Unless you reinstalled the computer or changed its keys, someone may be
      between this phone and it.
    </p>
    <p>Pinned: <code>{clean(app.link.pinned)}</code></p>
    <p>Now: <code>{clean(app.link.offered)}</code></p>
    <button onclick={forget}>Forget this computer…</button>
  {/if}
  {#if error}<p class="danger">{clean(error)}</p>{/if}
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 24px 16px;
  }
  h1 {
    margin: 0;
    font-size: 22px;
  }
  p {
    margin: 0;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  pre {
    margin: 0;
    padding: 12px;
    border-radius: 8px;
    background: var(--surface);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    font-size: 13px;
  }
  .fingerprint {
    font-size: 15px;
  }
  .danger {
    color: var(--danger);
  }
</style>
