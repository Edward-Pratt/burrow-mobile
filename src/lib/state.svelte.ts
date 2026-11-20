// The app's view of the daemon: the link, the hosts, their alerts, which
// are monitored in the background, and the prompts waiting for an answer.
// Everything is listed again after a (re)connection and whenever events
// may have been missed, so nothing here outlives a gap.

import { daemon, link as linkApi, onEvent, onLink, onRelist } from "./api";
import { isLocal } from "./local";
import type { Alert, Event, LinkState, Node, NodeId, Prompt, SessionId } from "./types";

export interface OpenPrompt {
  session: SessionId;
  prompt: Prompt;
}

export const app = $state({
  link: { state: "connecting" } as LinkState,
  nodes: [] as Node[],
  alerts: [] as Alert[],
  monitoring: [] as NodeId[],
  /// Every prompt waiting, the desktop's included, since this role hears
  /// all events; only those of `mine` are shown.
  prompts: [] as OpenPrompt[],
  /// Sessions this phone opened, whose prompts it answers.
  mine: [] as SessionId[],
  /// Bumped on every new connection, whose sessions are all new, so pages
  /// holding one open it again. A resync on the same connection isn't one.
  connection: 0,
  error: null as string | null,
});

type Listener = (event: Event) => void;
const listeners = new Set<Listener>();

/// For a page that wants events the app state doesn't keep, such as
/// monitor samples. Returns the function that stops it.
export function listenEvents(listener: Listener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/// Marks a session as the phone's, so its prompts show. A replayed prompt
/// may have arrived before the session's id did.
export function claim(session: SessionId) {
  if (!app.mine.includes(session)) app.mine = [...app.mine, session];
}

export function visiblePrompts(): OpenPrompt[] {
  return app.prompts.filter((p) => app.mine.includes(p.session));
}

export function hosts(): Node[] {
  return app.nodes.filter((n) => n.kind === "ssh-host").sort((a, b) => a.name.localeCompare(b.name));
}

export function alertsFor(node: NodeId): Alert[] {
  return app.alerts.filter((a) => a.node === node);
}

async function relist() {
  try {
    const [nodes, alerts, monitoring] = await Promise.all([
      daemon.nodes(),
      daemon.alerts(),
      daemon.monitoring(),
    ]);
    app.nodes = nodes;
    app.alerts = alerts;
    app.monitoring = monitoring;
    app.error = null;
  } catch (e) {
    app.error = String(e);
  }
}

function apply(event: Event) {
  switch (event.type) {
    case "node_created":
    case "node_updated": {
      const node = (event as { node: Node }).node;
      app.nodes = [...app.nodes.filter((n) => n.id !== node.id), node];
      break;
    }
    case "node_deleted": {
      const id = (event as { id: NodeId }).id;
      app.nodes = app.nodes.filter((n) => n.id !== id);
      break;
    }
    case "monitoring_changed":
      app.monitoring = (event as { nodes: NodeId[] }).nodes;
      break;
    case "alert_raised": {
      const alert = (event as { alert: Alert }).alert;
      app.alerts = [...app.alerts.filter((a) => a.id !== alert.id), alert];
      break;
    }
    case "alert_cleared": {
      const id = (event as { alert: Alert }).alert.id;
      app.alerts = app.alerts.filter((a) => a.id !== id);
      break;
    }
    case "prompt_requested": {
      // Joining a background monitor says its prompts again: show one once.
      const { session, prompt } = event as { session: SessionId; prompt: Prompt };
      if (!app.prompts.some((p) => p.session === session && p.prompt.id === prompt.id)) {
        app.prompts = [...app.prompts, { session, prompt }];
      }
      break;
    }
    case "prompt_closed": {
      const { session, prompt } = event as { session: SessionId; prompt: number };
      app.prompts = app.prompts.filter((p) => !(p.session === session && p.prompt.id === prompt));
      break;
    }
  }
  for (const listener of listeners) listener(event);
}

/// Starts following the bridge. Called once, when the app mounts.
export async function start() {
  await onLink((state) => {
    if (state.state === "connected" && app.link.state !== "connected") {
      // The old connection's sessions and prompts went with it.
      app.prompts = [];
      app.mine = [];
      app.connection++;
    }
    app.link = state;
  });
  await onRelist(relist);
  await onEvent(apply);
  app.link = await linkApi.state();
  if (app.link.state === "connected") await relist();
  // A computer paired by its LAN address needs the permission kept, and a
  // reinstall or the user may have taken it away.
  const paired = await linkApi.paired();
  if (paired && isLocal(paired.address)) {
    await linkApi.allowLocalNetwork();
    linkApi.reconnect();
  }
}
