<script lang="ts">
  // Keys a phone keyboard lacks. Ctrl and Alt are sticky: they apply to
  // the next key, from here or the keyboard, then let go.
  let {
    send,
    ctrl = $bindable(false),
    alt = $bindable(false),
  }: { send: (data: string) => void; ctrl?: boolean; alt?: boolean } = $props();

  const KEYS: [string, string][] = [
    ["Esc", "\x1b"],
    ["Tab", "\t"],
    ["↑", "\x1b[A"],
    ["↓", "\x1b[B"],
    ["←", "\x1b[D"],
    ["→", "\x1b[C"],
    ["|", "|"],
    ["~", "~"],
    ["/", "/"],
    ["-", "-"],
  ];

  // Buttons don't take the focus, so the keyboard stays up.
  const keep = (e: PointerEvent) => e.preventDefault();
</script>

<div class="bar" role="toolbar" aria-label="Extra keys">
  <button class:on={ctrl} aria-pressed={ctrl} onpointerdown={keep} onclick={() => (ctrl = !ctrl)}>
    Ctrl
  </button>
  <button class:on={alt} aria-pressed={alt} onpointerdown={keep} onclick={() => (alt = !alt)}>
    Alt
  </button>
  {#each KEYS as [label, data] (label)}
    <button onpointerdown={keep} onclick={() => send(data)}>{label}</button>
  {/each}
</div>

<style>
  .bar {
    display: flex;
    gap: 4px;
    padding: 6px 8px calc(env(safe-area-inset-bottom) + 6px);
    overflow-x: auto;
    border-top: 1px solid var(--line);
    background: var(--surface);
  }
  button {
    flex: none;
    min-width: 44px;
    min-height: 40px;
    padding: 0 10px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--bg);
    color: var(--fg);
    font: 15px ui-monospace, monospace;
  }
  button.on {
    border-color: var(--accent);
    color: var(--accent);
    font-weight: 600;
  }
</style>
