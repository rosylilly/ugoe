<!--
  フェイストラッキングで同梱する MediaPipe (wasm とモデル、Apache-2.0) のライセンスの表示 (T0636)。
  本文は `pnpm mediapipe` が public/mediapipe/ に写した third-party-notices.txt (@ugoe/face-tracking のもの)。
-->
<script lang="ts">
let dialog = $state<HTMLDialogElement | null>(null);
let text = $state("");

export async function open(): Promise<void> {
  dialog?.showModal();
  if (text !== "") return;
  try {
    const res = await fetch(new URL("mediapipe/third-party-notices.txt", document.baseURI));
    text = res.ok ? await res.text() : `Cannot load the notices (${res.status})`;
  } catch (e) {
    text = `Cannot load the notices: ${e instanceof Error ? e.message : String(e)}`;
  }
}
</script>

<dialog bind:this={dialog} onclick={(e) => e.target === dialog && dialog?.close()}>
  <header>
    <h2>Face tracking licenses</h2>
    <button type="button" onclick={() => dialog?.close()}>Close</button>
  </header>
  <p>
    The ugoe editor is distributed under GPL-3.0-only. Face tracking uses MediaPipe, bundled with the
    editor under the following licenses.
  </p>
  <pre>{text}</pre>
</dialog>

<style>
  dialog {
    width: min(760px, 90vw);
    max-height: 85vh;
    padding: 16px;
    color: var(--text);
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 6px;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  h2 {
    margin: 0;
    font-size: 14px;
  }
  pre {
    max-height: 60vh;
    overflow: auto;
    font-size: 11px;
    white-space: pre-wrap;
  }
</style>
