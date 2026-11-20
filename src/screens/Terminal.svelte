<script lang="ts">
  // A shell on a host, in xterm. Leaving keeps it running in the daemon;
  // only Close ends it. The server's output goes into xterm as it is, so
  // xterm gets no clipboard or link addons: a server can't write to the
  // phone's clipboard or plant a link.
  import "@xterm/xterm/css/xterm.css";
  import { FitAddon } from "@xterm/addon-fit";
  import { Terminal } from "@xterm/xterm";
  import { onMount } from "svelte";

  import KeyBar from "../components/KeyBar.svelte";
  import { daemon } from "../lib/api";
  import { Shell, type ShellStatus } from "../lib/shell";
  import { app } from "../lib/state.svelte";
  import { clean } from "../lib/text";
  import type { Node, SessionId } from "../lib/types";

  let { node, session, onclosed }: { node: Node; session: SessionId | null; onclosed: () => void } =
    $props();

  let element: HTMLDivElement;
  let status = $state<ShellStatus>({ state: "connecting" });
  let ctrl = $state(false);
  let alt = $state(false);
  let shell: Shell | undefined;

  /// Applies the sticky Ctrl and Alt to a key, then lets go of them.
  function modified(data: string): string {
    if (ctrl && data.length === 1) {
      const code = data === " " ? 0 : data.toUpperCase().charCodeAt(0) & 0x1f;
      data = String.fromCharCode(code);
    }
    if (alt) data = "\x1b" + data;
    ctrl = alt = false;
    return data;
  }

  function colour(name: string): string {
    return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  }

  onMount(() => {
    const term = new Terminal({
      fontFamily: "ui-monospace, monospace",
      fontSize: 14,
      scrollback: 5000,
      cursorBlink: true,
      theme: { background: colour("--bg"), foreground: colour("--fg"), cursor: colour("--accent") },
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(element);
    const input = element.querySelector("textarea");
    for (const [name, value] of [["autocorrect", "off"], ["autocapitalize", "none"], ["spellcheck", "false"]]) {
      input?.setAttribute(name, value);
    }
    fit.fit();
    shell = new Shell(node.id, session, {
      get cols() { return term.cols; },
      get rows() { return term.rows; },
      write: (data, done) => term.write(data, done),
      reset: () => term.reset(),
    }, (s) => (status = s));
    term.onData((data) => shell?.type(modified(data)));
    term.onResize(({ cols, rows }) => shell?.resize(cols, rows));
    const observer = new ResizeObserver(() => fit.fit());
    observer.observe(element);
    void shell.start();
    return () => {
      observer.disconnect();
      shell?.stop();
      term.dispose();
    };
  });

  // A new connection: attach to the same shell there at once, rather
  // than when the backoff comes round.
  let seen = app.connection;
  $effect(() => {
    if (app.connection !== seen && app.link.state === "connected") {
      seen = app.connection;
      void shell?.attach();
    }
  });

  async function close() {
    if (shell?.session == null) return onclosed();
    if (!confirm(`End this shell on ${clean(node.name)}?`)) return;
    await daemon.closeSession(shell.session).catch(() => {});
    onclosed();
  }
</script>

<div class="screen">
  <div class="bar">
    <span class="name">{clean(node.name)}</span>
    {#if status.state === "connecting"}<span class="muted">Connecting…</span>{/if}
    <button class="plain" onclick={close}>End shell…</button>
  </div>
  <div class="term" bind:this={element}></div>
  {#if status.state === "locked"}
    <div class="over">
      <p>Shells are locked.</p>
      <button onclick={() => shell?.start()}>Unlock</button>
    </div>
  {:else if status.state === "closed"}
    <div class="over"><p>This shell has ended.</p></div>
  {:else if status.state === "failed"}
    <div class="over"><p>{clean(status.error)}</p></div>
  {/if}
  <KeyBar send={(data) => shell?.type(data)} bind:ctrl bind:alt />
</div>

<style>
  .screen {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    border-bottom: 1px solid var(--line);
  }
  .name {
    flex: 1;
    font-weight: 600;
  }
  .muted {
    color: var(--muted);
    font-size: 13px;
  }
  .term {
    flex: 1;
    min-height: 0;
    padding: 4px;
  }
  .over {
    position: absolute;
    inset: 48px 16px auto;
    padding: 16px;
    border: 1px solid var(--line);
    border-radius: 12px;
    background: var(--surface);
    text-align: center;
  }
</style>
