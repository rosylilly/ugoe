<script lang="ts">
// ライセンスの表示。スタジオ (GPL-3.0-only) と、同梱したものの表記 (public/third-party-notices.txt)。
import { Dialog } from "@ugoe/ui";
import { t } from "./lib/i18n";

let { open = $bindable(false) }: { open?: boolean } = $props();

let text = $state("");

// 開いたときに 1 回だけ読む。
$effect(() => {
  if (open && text === "") void load();
});

async function load() {
  try {
    const res = await fetch(new URL("third-party-notices.txt", document.baseURI));
    text = res.ok ? await res.text() : t("license.loadFailed", { reason: String(res.status) });
  } catch (e) {
    text = t("license.loadFailed", { reason: e instanceof Error ? e.message : String(e) });
  }
}
</script>

<Dialog bind:open title={t("license.title")} description={t("license.description")} size="lg">
  <pre>{text}</pre>
</Dialog>

<style>
  pre {
    max-height: 60vh;
    overflow: auto;
    margin: 0;
    font-family: var(--ugoe-font-mono);
    font-size: var(--ugoe-font-size-xs);
    white-space: pre-wrap;
  }
</style>
