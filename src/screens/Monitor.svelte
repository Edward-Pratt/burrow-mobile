<script lang="ts">
  // One host: live samples every 2 s from a monitor, and the trend Burrow
  // keeps on disk for hosts monitored in the background. For such a host
  // open_monitor joins the background session, which only turning
  // monitoring off stops, so leaving closes only a monitor of the phone's.
  import Sparkline from "../components/Sparkline.svelte";
  import { daemon } from "../lib/api";
  import { alertsFor, app, claim, listenEvents } from "../lib/state.svelte";
  import { describe } from "../lib/alerts";
  import { bytes, clean, percent } from "../lib/text";
  import type { Node, Sample, SessionId, TrendRange } from "../lib/types";

  let { node }: { node: Node } = $props();

  const LIVE = 150;
  let session = $state<SessionId | null>(null);
  let samples = $state<Sample[]>([]);
  let step = $state("Connecting…");
  let range = $state<TrendRange>("hour");
  let trend = $state<Sample[]>([]);
  let error = $state<string | null>(null);

  const background = $derived(app.monitoring.includes(node.id));
  const latest = $derived(samples.at(-1));
  const used = (s: Sample) => (s.memory.total ? 100 - (s.memory.available / s.memory.total) * 100 : 0);

  $effect(() => {
    // Again on every new connection: the old one's sessions are gone.
    void app.connection;
    if (app.link.state !== "connected") return;
    const id = node.id;
    let mine: SessionId | null = null;
    let gone = false;
    samples = [];
    const stop = listenEvents((e) => {
      if (e.type === "monitor_sample" && "session" in e && e.session === session) {
        samples = [...samples.slice(-(LIVE - 1)), (e as { sample: Sample }).sample];
        step = "";
      } else if (e.type === "session_changed" && "session" in e && e.session === session) {
        const s = (e as { state: { state: string; step?: string; error?: string | null } }).state;
        step = s.state === "connecting" ? clean(s.step ?? "") : s.state === "closed" ? clean(s.error ?? "Closed") : "";
      }
    });
    daemon.openMonitor(id).then(
      (s) => {
        claim(s);
        if (gone) return void close(s, id);
        session = mine = s;
      },
      (e) => (error = String(e)),
    );
    return () => {
      gone = true;
      stop();
      if (mine !== null) close(mine, id);
      session = null;
    };
  });

  function close(s: SessionId, id: string) {
    // A background monitor isn't the phone's to stop, and refuses anyway.
    if (!app.monitoring.includes(id)) daemon.closeSession(s).catch(() => {});
  }

  $effect(() => {
    void app.connection;
    const r = range;
    if (app.link.state !== "connected") return;
    daemon.trend(node.id, r).then(
      (points) => (trend = points),
      (e) => (error = String(e)),
    );
  });
</script>

<section>
  <h1>{clean(node.name)}</h1>
  {#each alertsFor(node.id) as alert (alert.id)}
    <p class="alert">{describe(alert)}</p>
  {/each}
  {#if error}<p class="alert">{clean(error)}</p>{/if}

  {#if latest}
    <div class="stats">
      <div><span>CPU</span><strong>{percent(latest.cpu.percent)}</strong></div>
      <div><span>Memory</span><strong>{percent(used(latest))}</strong></div>
      <div><span>Load</span><strong>{latest.load[0].toFixed(2)}</strong></div>
    </div>
    <Sparkline values={samples.map((s) => s.cpu.percent)} label="CPU, live" />
    <Sparkline values={samples.map(used)} label="Memory, live" />
    <h2>Disks</h2>
    {#each latest.disks as disk (disk.mount)}
      {@const full = disk.total ? 100 - (disk.available / disk.total) * 100 : 0}
      <div class="disk">
        <span>{clean(disk.mount)}</span>
        <span>{bytes(disk.total - disk.available)} of {bytes(disk.total)}</span>
        <meter min="0" max="100" high="90" value={full}></meter>
      </div>
    {/each}
    <h2>Network</h2>
    {#each latest.net as net (net.name)}
      <p class="net">{clean(net.name)}: ↓ {bytes(net.rx)}/s ↑ {bytes(net.tx)}/s</p>
    {/each}
  {:else}
    <p class="muted">{step || "Waiting for the first sample…"}</p>
  {/if}

  <h2>Trend</h2>
  <div class="ranges" role="group" aria-label="Trend range">
    {#each ["hour", "day", "week"] as const as r (r)}
      <button class:primary={range === r} onclick={() => (range = r)}>
        {r === "hour" ? "Hour" : r === "day" ? "Day" : "Week"}
      </button>
    {/each}
  </div>
  {#if trend.length}
    <span class="muted">CPU</span>
    <Sparkline values={trend.map((s) => s.cpu.percent)} label="CPU, {range}" />
    <span class="muted">Memory</span>
    <Sparkline values={trend.map(used)} label="Memory, {range}" />
  {:else}
    <p class="muted">
      {background ? "No history yet." : "Burrow keeps history for hosts monitored in the background."}
    </p>
  {/if}
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 16px;
  }
  h1 {
    margin: 0;
    font-size: 22px;
  }
  h2 {
    margin: 12px 0 0;
    font-size: 16px;
  }
  .stats {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
  }
  .stats div {
    display: flex;
    flex-direction: column;
    padding: 10px;
    border-radius: 8px;
    background: var(--surface);
  }
  .stats span,
  .muted {
    color: var(--muted);
    font-size: 13px;
  }
  .stats strong {
    font-size: 20px;
  }
  .disk {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 4px 8px;
    font-size: 14px;
  }
  .disk meter {
    grid-column: 1 / -1;
    width: 100%;
  }
  .net {
    margin: 0;
    font-size: 14px;
  }
  .alert {
    margin: 0;
    color: var(--danger);
  }
  .ranges {
    display: flex;
    gap: 8px;
  }
</style>
