<!--
  フェイストラッキングで同梱する MediaPipe (wasm とモデル、Apache-2.0) のライセンスの表示 (T0636)。
  本文は `pnpm mediapipe` が public/mediapipe/ に写した third-party-notices.txt (@ugoe/face-tracking のもの)。
-->
<script lang="ts">
import { Dialog } from "@ugoe/ui";
import { errorText } from "../errors";
import { t } from "../i18n";

let shown = $state(false);
let text = $state("");

export async function open(): Promise<void> {
  shown = true;
  if (text !== "") return;
  try {
    const res = await fetch(new URL("mediapipe/third-party-notices.txt", document.baseURI));
    text = res.ok ? await res.text() : t("preview.licenses.loadStatus", { status: res.status });
  } catch (e) {
    text = t("preview.licenses.loadError", { message: errorText(e) });
  }
}
</script>

<Dialog
  bind:open={shown}
  title={t("preview.licenses.title")}
  description={t("preview.licenses.description")}
  size="lg"
  onclose={() => (shown = false)}
>
  <pre>{text}</pre>
</Dialog>

<style>
  pre {
    max-height: 60vh;
    margin: 0;
    overflow: auto;
    font-family: var(--ugoe-font-mono);
    font-size: var(--ugoe-font-size-xs);
    white-space: pre-wrap;
  }
</style>
