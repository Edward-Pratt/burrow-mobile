// The parts of Burrow's API the phone's screens use, written to match
// burrow-api's serde shapes: tagged enums carry `type`, `kind` or `state`.

export type NodeId = string;
export type SessionId = number;

export interface Node {
  id: NodeId;
  parent: NodeId | null;
  kind: string;
  name: string;
  props: Record<string, unknown>;
}

export type AlertKind =
  | { kind: "unit_failed"; unit: string }
  | { kind: "disk_full"; mount: string; percent: number }
  | { kind: "memory_low"; percent: number }
  | { kind: "unreachable"; error: string }
  | { kind: "container_failed"; container: string; status: string }
  | { kind: "app_failed"; app: string };

export type Alert = { id: number; node: NodeId; since: number } & AlertKind;

export interface Sample {
  time: number;
  uptime: number;
  load: [number, number, number];
  cpu: { percent: number; cores: number };
  memory: { total: number; available: number; swap_total: number; swap_free: number };
  disks: { mount: string; fs: string; total: number; available: number }[];
  net: { name: string; rx: number; tx: number }[];
}

export type SessionState =
  | { state: "connecting"; step: string }
  | { state: "connected" }
  | { state: "closed"; error: string | null };

export type SessionInfo = {
  session: SessionId;
  node: NodeId;
  kind: string;
  background?: boolean;
  /// A terminal that outlives its connection: the phone's shells.
  kept?: boolean;
  /// A kept terminal someone is reading now, maybe this phone.
  attached?: boolean;
} & SessionState;

export interface Prompt {
  id: number;
  kind: { type: "confirm" } | { type: "secret"; can_save: boolean } | { type: "text" };
  title: string;
  message: string;
  action?: string;
}

export type PromptAnswer =
  | { type: "cancel" }
  | { type: "confirm" }
  | { type: "text"; value: string; save: boolean };

export type TrendRange = "hour" | "day" | "week";

export type Response =
  | { type: "nodes"; nodes: Node[] }
  | { type: "alerts"; alerts: Alert[] }
  | { type: "monitoring"; nodes: NodeId[] }
  | { type: "sessions"; sessions: SessionInfo[] }
  | { type: "session"; session: SessionId }
  | { type: "trend"; points: Sample[] }
  | { type: "ok" };

export type Event =
  | { type: "node_created"; node: Node }
  | { type: "node_updated"; node: Node }
  | { type: "node_deleted"; id: NodeId }
  | { type: "session_changed"; session: SessionId; node: NodeId; state: SessionState }
  | { type: "monitor_sample"; session: SessionId; node: NodeId; sample: Sample }
  | { type: "monitoring_changed"; nodes: NodeId[] }
  | { type: "alert_raised"; alert: Alert }
  | { type: "alert_cleared"; alert: Alert }
  | { type: "prompt_requested"; session: SessionId; prompt: Prompt }
  | { type: "prompt_closed"; session: SessionId; prompt: number }
  | { type: string };

/// Where the phone's connection to burrowd stands (burrow-link's LinkState).
export type LinkState =
  | { state: "unpaired" }
  | { state: "connecting" }
  | { state: "connected"; build: string }
  | { state: "new_host_key"; fingerprint: string; key: string }
  | { state: "host_key_changed"; pinned: string; offered: string }
  | { state: "key_refused" }
  | { state: "failed"; error: string; retry_in: number };

export interface Paired {
  address: string;
  port: number;
  user: string;
}
