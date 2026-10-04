<!--
  同梱する第三者のもの (MediaPipe の wasm とモデル・バンドルに入る JS のライブラリ) のライセンスの表示 (T0636・T0926)。
  本文は public/third-party-notices.txt (scripts/third-party-notices.mjs が生成する。docs/release.md#同梱物のライセンスの表記)。
-->
<script lang="ts">
import { Dialog } from "@ugoe/ui";
import { errorText } from "../errors";
import { t } from "../i18n";

let shown = $state(false);
let text = $state("");
/** 本文を読めたか (読めなかったときは、次に開いたときに読み直す)。 */
let loaded = false;

export async function open(): Promise<void> {
  shown = true;
  if (loaded) return;
  try {
    const res = await fetch(new URL("third-party-notices.txt", document.baseURI));
    if (res.ok) {
      text = await res.text();
      loaded = true;
    } else {
      text = t("preview.licenses.loadStatus", { status: res.status });
    }
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
  <pre class="ugoe-selectable">{text}</pre>
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
