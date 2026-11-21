// One shell on the phone: opened and kept in the daemon, so it outlives
// the screen locking, and attached to again whenever its stream ends.
//
// The stream ends when the connection drops, when shells lock, when
// another reader takes over or this one fell behind. Each time, the shell
// lists sessions (which tells the bridge the terminals it kept are the
// phone's on this connection) and attaches again with a backoff, unless
// the terminal is gone or shells are locked.

import { daemon, shells, type TerminalOutput } from "./api";
import { claim, takeTerminals } from "./state.svelte";
import type { NodeId, SessionId } from "./types";

export type ShellStatus =
  | { state: "connecting" }
  | { state: "open" }
  | { state: "locked" }
  | { state: "closed" }
  | { state: "failed"; error: string };

/// The terminal view the shell writes to.
export interface Screen {
  readonly cols: number;
  readonly rows: number;
  write(data: Uint8Array, done?: () => void): void;
  reset(): void;
}

const FIRST_RETRY = 500;
const LAST_RETRY = 8000;

export function toBase64(text: string): string {
  let binary = "";
  for (const byte of new TextEncoder().encode(text)) binary += String.fromCharCode(byte);
  return btoa(binary);
}

export function fromBase64(data: string): Uint8Array {
  return Uint8Array.from(atob(data), (c) => c.charCodeAt(0));
}

const isLocked = (e: unknown) => String(e).includes("unlock the phone");

export class Shell {
  /// Replies the terminal makes to queries in a replay are old: they'd
  /// type junk into the shell, so typing waits until the replay is drawn.
  private muted = true;
  /// Bumped on every attach, so output from an earlier one is ignored.
  private attaches = 0;
  private retry = FIRST_RETRY;
  private timer: ReturnType<typeof setTimeout> | undefined;
  private stopped = false;
  private typed = "";
  private sending = false;

  constructor(
    private readonly node: NodeId,
    public session: SessionId | null,
    private readonly screen: Screen,
    private readonly status: (status: ShellStatus) => void,
  ) {}

  /// Unlocks shells if they aren't, opens the terminal if it's new, and
  /// attaches. Called again by the page's Unlock button.
  async start() {
    try {
      if (!(await shells.unlocked())) await shells.unlock();
    } catch {
      return this.status({ state: "locked" });
    }
    this.status({ state: "connecting" });
    try {
      if (this.session === null) {
        const session = await daemon.openTerminal(this.node, this.screen.cols, this.screen.rows);
        claim(session);
        await daemon.keepTerminal(session);
        this.session = session;
      }
    } catch (e) {
      return this.status({ state: "failed", error: String(e) });
    }
    await this.attach();
  }

  async attach() {
    clearTimeout(this.timer);
    const session = this.session;
    if (session === null || this.stopped) return;
    const attach = ++this.attaches;
    this.muted = true;
    try {
      const sessions = await daemon.sessions();
      takeTerminals(sessions);
      if (!sessions.some((s) => s.session === session)) return this.status({ state: "closed" });
      await shells.attach(session, (output) => this.take(attach, output));
      // A program redraws on a change of size, and only a change: one row
      // fewer and back makes a full-screen one draw itself again whole.
      const { cols, rows } = this.screen;
      await daemon.resize(session, cols, Math.max(rows - 1, 1));
      await daemon.resize(session, cols, rows);
    } catch (e) {
      if (isLocked(e)) return this.status({ state: "locked" });
      this.again();
    }
  }

  private take(attach: number, output: TerminalOutput) {
    if (attach !== this.attaches || this.stopped) return;
    switch (output.kind) {
      case "replay":
        this.screen.reset();
        this.screen.write(fromBase64(output.data), () => {
          if (attach === this.attaches) this.muted = false;
        });
        this.retry = FIRST_RETRY;
        this.status({ state: "open" });
        break;
      case "data":
        this.screen.write(fromBase64(output.data));
        break;
      case "ended":
        this.muted = true;
        this.status({ state: "connecting" });
        this.again();
        break;
    }
  }

  private again() {
    if (this.stopped) return;
    this.timer = setTimeout(() => this.attach(), this.retry);
    this.retry = Math.min(this.retry * 2, LAST_RETRY);
  }

  /// What the user typed, already turned into the bytes a terminal sends.
  /// Commands run side by side in the bridge, so one write at a time is
  /// sent, with whatever was typed meanwhile, or keys could swap places.
  type(data: string) {
    if (this.muted || this.session === null) return;
    this.typed += data;
    if (!this.sending) void this.send(this.session);
  }

  private async send(session: SessionId) {
    this.sending = true;
    while (this.typed) {
      const data = this.typed;
      this.typed = "";
      try {
        await daemon.write(session, toBase64(data));
      } catch (e) {
        if (isLocked(e)) this.status({ state: "locked" });
      }
    }
    this.sending = false;
  }

  resize(cols: number, rows: number) {
    if (this.session === null) return;
    daemon.resize(this.session, cols, rows).catch(() => {});
  }

  /// The page left: its stream goes, and the terminal runs on in the
  /// daemon, unread.
  stop() {
    this.stopped = true;
    this.attaches++;
    clearTimeout(this.timer);
    if (this.session !== null) shells.detach(this.session).catch(() => {});
  }
}
