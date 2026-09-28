<!--
  ヘッダーの「Help」。操作マニュアルをブラウザで開く (lib/help.ts)。開けなかったときは、
  URL を選んで写せるように出す。
-->
<script lang="ts">
import { MANUAL_URL, openExternal } from "./help";

/** 開けなかったときの理由 (null なら出さない)。 */
let failure = $state<string | null>(null);

async function open() {
  try {
    await openExternal(MANUAL_URL);
    failure = null;
  } catch (e) {
    failure = String(e);
  }
}
</script>

<span class="help">
  <button onclick={open} title="Open the user manual in your browser">Help</button>
  {#if failure !== null}
    <span class="popup" role="status">
      Could not open the browser ({failure}). Manual:
      <a href={MANUAL_URL} target="_blank" rel="noreferrer">{MANUAL_URL}</a>
      <button onclick={() => (failure = null)} aria-label="Close">×</button>
    </span>
  {/if}
</span>

<style>
  .help {
    position: relative;
  }
  .popup {
    position: absolute;
    right: 0;
    top: 100%;
    z-index: 10;
    margin-top: 4px;
    padding: 8px;
    background: var(--panel);
    border: 1px solid var(--border);
    white-space: nowrap;
    user-select: text;
  }
</style>
