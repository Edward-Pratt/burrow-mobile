<script lang="ts">
  // A session's question, such as whether to install burrow-agent on a
  // host. Shown once per prompt, for the phone's own sessions only; the
  // daemon says when another client answered it first.
  import { daemon } from "../lib/api";
  import { app, visiblePrompts } from "../lib/state.svelte";
  import { clean } from "../lib/text";
  import type { PromptAnswer } from "../lib/types";

  const current = $derived(visiblePrompts()[0]);
  let value = $state("");
  let error = $state<string | null>(null);

  async function reply(answer: PromptAnswer) {
    if (!current) return;
    const { session, prompt } = current;
    try {
      await daemon.answer(session, prompt.id, answer);
      app.prompts = app.prompts.filter((p) => !(p.session === session && p.prompt.id === prompt.id));
      value = "";
      error = null;
    } catch (e) {
      error = String(e);
    }
  }
</script>

{#if current}
  <div class="scrim">
    <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="prompt-title">
      <h2 id="prompt-title">{clean(current.prompt.title)}</h2>
      <p>{clean(current.prompt.message)}</p>
      {#if current.prompt.kind.type !== "confirm"}
        <!-- A secret answer is sent once and never kept on the phone. -->
        <input
          type={current.prompt.kind.type === "secret" ? "password" : "text"}
          bind:value
          autocomplete="off"
        />
      {/if}
      {#if error}<p class="error">{clean(error)}</p>{/if}
      <div class="actions">
        <button onclick={() => reply({ type: "cancel" })}>Cancel</button>
        {#if current.prompt.kind.type === "confirm"}
          <button class="primary" onclick={() => reply({ type: "confirm" })}>
            {clean(current.prompt.action ?? "Yes")}
          </button>
        {:else}
          <button class="primary" onclick={() => reply({ type: "text", value, save: false })}>OK</button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 16px;
    background: rgb(0 0 0 / 0.45);
    z-index: 10;
  }
  .dialog {
    width: 100%;
    max-width: 420px;
    padding: 20px;
    border-radius: 16px;
    background: var(--bg);
  }
  h2 {
    margin: 0 0 8px;
    font-size: 18px;
  }
  p {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  input {
    width: 100%;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 16px;
  }
  .error {
    color: var(--danger);
  }
</style>
