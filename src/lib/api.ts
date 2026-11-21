// The Rust side's commands and events. Daemon commands go through
// `execute`, which the bridge checks against its allowlist.

import { Channel, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type {
  Alert, Event, LinkState, Node, NodeId, Paired, PromptAnswer, Response, Sample,
  SessionId, SessionInfo, TrendRange,
} from "./types";

async function execute<T extends Response["type"]>(
  command: Record<string, unknown>,
  expect: T,
): Promise<Extract<Response, { type: T }>> {
  const response = await invoke<Response>("execute", { command });
  if (response.type !== expect) {
    throw new Error(`burrowd answered ${response.type}, not ${expect}`);
  }
  return response as Extract<Response, { type: T }>;
}

export const daemon = {
  nodes: async (): Promise<Node[]> => (await execute({ type: "list_nodes" }, "nodes")).nodes,
  alerts: async (): Promise<Alert[]> => (await execute({ type: "list_alerts" }, "alerts")).alerts,
  monitoring: async (): Promise<NodeId[]> =>
    (await execute({ type: "list_monitoring" }, "monitoring")).nodes,
  sessions: async (): Promise<SessionInfo[]> =>
    (await execute({ type: "list_sessions" }, "sessions")).sessions,
  openMonitor: async (node: NodeId): Promise<SessionId> =>
    (await execute({ type: "open_monitor", node }, "session")).session,
  closeSession: async (session: SessionId): Promise<void> => {
    await execute({ type: "close_session", session }, "ok");
  },
  trend: async (node: NodeId, range: TrendRange): Promise<Sample[]> =>
    (await execute({ type: "monitor_trend", node, range }, "trend")).points,
  answer: async (session: SessionId, prompt: number, answer: PromptAnswer): Promise<void> => {
    await execute({ type: "answer_prompt", session, prompt, answer }, "ok");
  },
  openTerminal: async (node: NodeId, cols: number, rows: number): Promise<SessionId> =>
    (await execute({ type: "open_terminal", node, cols, rows }, "session")).session,
  keepTerminal: async (session: SessionId): Promise<void> => {
    await execute({ type: "keep_terminal", session }, "ok");
  },
  /// `data` is base64, as the API takes bytes.
  write: async (session: SessionId, data: string): Promise<void> => {
    await execute({ type: "write_terminal", session, data }, "ok");
  },
  resize: async (session: SessionId, cols: number, rows: number): Promise<void> => {
    await execute({ type: "resize_terminal", session, cols, rows }, "ok");
  },
};

/// What a kept terminal's channel carries (src-tauri's shells::Output).
export type TerminalOutput =
  | { kind: "replay"; data: string }
  | { kind: "data"; data: string }
  | { kind: "ended" };

export const shells = {
  unlocked: () => invoke<boolean>("shells_unlocked"),
  /// Shows the system's fingerprint or PIN prompt. Rejects if cancelled.
  unlock: () => invoke<void>("unlock_shells"),
  attach: (session: SessionId, onOutput: (output: TerminalOutput) => void) => {
    const channel = new Channel<TerminalOutput>();
    channel.onmessage = onOutput;
    return invoke<void>("attach_terminal", { session, onOutput: channel });
  },
  detach: (session: SessionId) => invoke<void>("detach_terminal", { session }),
};

export const link = {
  state: () => invoke<LinkState>("link_state"),
  paired: () => invoke<Paired | null>("paired"),
  keyLine: (user: string) => invoke<string>("key_line", { user }),
  pair: (address: string, port: number, user: string) =>
    invoke<void>("pair", { address, port, user }),
  pin: (key: string) => invoke<void>("pin", { key }),
  forget: () => invoke<void>("forget"),
  reconnect: () => invoke<void>("reconnect"),
  allowLocalNetwork: () => invoke<boolean>("allow_local_network"),
};

export function onLink(handler: (state: LinkState) => void): Promise<UnlistenFn> {
  return listen<LinkState>("burrow://link", (e) => handler(e.payload));
}

export function onEvent(handler: (event: Event) => void): Promise<UnlistenFn> {
  return listen<Event>("burrow://event", (e) => handler(e.payload));
}

export function onRelist(handler: () => void): Promise<UnlistenFn> {
  return listen("burrow://relist", () => handler());
}
