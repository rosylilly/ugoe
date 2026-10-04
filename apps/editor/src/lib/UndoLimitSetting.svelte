<!--
  設定の「General」の Undo の上限 (docs/editor.md#undo)。確定するとすぐに当て (超えた古い履歴は捨てる)、バックエンドが
  アプリの設定のディレクトリに覚える (次の起動でも使う。apps/editor/src-tauri/src/history.rs)。
-->
<script module lang="ts">
/** 選べる範囲 (apps/editor/src-tauri/src/history.rs の UNDO_LIMIT_RANGE と同じ)。 */
export const UNDO_LIMIT_MIN = 1;
export const UNDO_LIMIT_MAX = 1000;
</script>

<script lang="ts">
import { NumberField } from "@ugoe/ui";
import type { EditorDocument } from "./document.svelte";
import { errorText } from "./errors";
import { t } from "./i18n";

interface Props {
  doc: EditorDocument;
}

let { doc }: Props = $props();

const id = $props.id();
let busy = $state(false);
let error = $state<string | null>(null);
/** 変えるたびに増やし、欄を今の上限から作り直す (失敗したときに入力した値を残さない)。 */
let generation = $state(0);

async function change(limit: number): Promise<void> {
  busy = true;
  error = null;
  try {
    await doc.setUndoLimit(limit);
  } catch (e) {
    error = errorText(e);
  } finally {
    busy = false;
    generation += 1;
  }
}
</script>

<div class="undo-limit">
  {#if doc.history}
    {#key generation}
      <NumberField
        label={t("settings.undoLimit.label")}
        unit={t("settings.undoLimit.unit")}
        min={UNDO_LIMIT_MIN}
        max={UNDO_LIMIT_MAX}
        step={1}
        size={5}
        value={doc.history.undo_limit}
        disabled={busy}
        describedby={`${id}-note`}
        onchange={change}
      />
    {/key}
  {/if}
  <p class="hint" id={`${id}-note`}>{t("settings.undoLimit.description")}</p>
  {#if error}<p class="error ugoe-selectable">{error}</p>{/if}
</div>

<style>
  .undo-limit {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-2);
  }
  .hint {
    margin: 0;
    color: var(--ugoe-color-text-muted);
    font-size: var(--ugoe-font-size-sm);
  }
  .error {
    margin: 0;
    color: var(--ugoe-color-danger);
    overflow-wrap: anywhere;
  }
</style>
