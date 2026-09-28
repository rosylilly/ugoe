<script lang="ts">
// ライセンスの表示。ビューア (GPL-3.0-only) と、同梱したものの表記 (public/third-party-notices.txt)。
let dialog = $state<HTMLDialogElement | null>(null);
let text = $state("");

export async function open(): Promise<void> {
  dialog?.showModal();
  if (text !== "") return;
  try {
    const res = await fetch(new URL("third-party-notices.txt", document.baseURI));
    text = res.ok ? await res.text() : `読み込めません (${res.status})`;
  } catch (e) {
    text = `読み込めません: ${e instanceof Error ? e.message : String(e)}`;
  }
}
</script>

<dialog bind:this={dialog} onclick={(e) => e.target === dialog && dialog?.close()}>
  <header>
    <h2>ライセンス</h2>
    <button type="button" onclick={() => dialog?.close()}>閉じる</button>
  </header>
  <p>
    ugoe viewer は GPL-3.0-only で配布しています。同梱しているソフトウェアとモデルのライセンスは次のとおりです。
  </p>
  <pre>{text}</pre>
</dialog>

<style>
  dialog {
    width: min(760px, 90vw);
    max-height: 85vh;
    padding: 1rem;
    color: var(--text);
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 6px;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  h2 {
    margin: 0;
    font-size: 1rem;
  }
  p {
    font-size: 0.85rem;
  }
  pre {
    max-height: 60vh;
    overflow: auto;
    font-size: 0.75rem;
    white-space: pre-wrap;
    margin: 0;
  }
</style>
