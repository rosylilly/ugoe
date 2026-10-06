<!--
  着せ替えのモジュールのプロジェクトの新規作成 (File > New Module…、docs/editor.md#モジュールのプロジェクト)。
  本体 (.ugoeproj のフォルダーか .ugoe) を選ぶとスロットの一覧を読み、スロットとモジュールの名前を決めて作る。
  スロットのない本体は、本体のプロジェクトの「Slots」タブでスロットを足してから選ぶ。Esc で閉じる。名前の欄の Enter で作る
  (既定のボタン。docs/editor.md#ダイアログ)。
-->
<script lang="ts">
import { Button, Dialog, TextField } from "@ugoe/ui";
import { untrack } from "svelte";
import { errorText } from "./errors";
import { fileDialogs } from "./fileDialogs";
import { BUNDLE_FILTER } from "./filePaths";
import { t } from "./i18n";
import { type BaseSummary, ipc } from "./ipc";
import {
  isValidModuleName,
  moduleCan,
  moduleCannot,
  slotSummary,
  suggestModuleName,
} from "./moduleProject";

interface Props {
  open: boolean;
  /** 最初に開く場所。 */
  startDir: string | null;
  /** 作る (失敗すると例外。メッセージを出してダイアログは開いたまま)。 */
  oncreate: (base: string, slot: string, name: string) => Promise<void>;
  onclose: () => void;
}

let { open, startDir, oncreate, onclose }: Props = $props();

let basePath = $state<string | null>(null);
let base = $state<BaseSummary | null>(null);
let slot = $state<string | null>(null);
let name = $state("");
/** 名前を手で変えたか (変えていなければスロットを選ぶたびに候補を入れる)。 */
let nameEdited = false;
let error = $state<string | null>(null);
let busy = $state(false);
const id = $props.id();

// 開くたびに、前に選んだ本体・名前とエラーを消す。
$effect(() => {
  if (open) untrack(reset);
});

function reset(): void {
  basePath = null;
  base = null;
  slot = null;
  name = "";
  nameEdited = false;
  error = null;
}

async function chooseBase(kind: "project" | "bundle") {
  const dialogs = fileDialogs();
  const defaultPath = startDir ?? undefined;
  const path =
    kind === "project"
      ? await dialogs.openDirectory({ title: t("newModule.chooseProject.title"), defaultPath })
      : await dialogs.openFile({
          title: t("newModule.chooseBundle.title"),
          defaultPath,
          filters: [BUNDLE_FILTER],
        });
  if (!path) return;
  busy = true;
  try {
    base = await ipc.readModuleBase(path);
    basePath = path;
    error = null;
    selectSlot(base.slots[0]?.id ?? null);
  } catch (e) {
    error = errorText(e);
  } finally {
    busy = false;
  }
}

function selectSlot(id: string | null) {
  slot = id;
  if (id && !nameEdited) name = suggestModuleName(id);
}

async function create() {
  if (!ready || !basePath || !slot) return;
  busy = true;
  try {
    await oncreate(basePath, slot, name.trim());
    error = null;
  } catch (e) {
    error = errorText(e);
  } finally {
    busy = false;
  }
}

const ready = $derived(basePath !== null && slot !== null && isValidModuleName(name) && !busy);
</script>

<Dialog {open} title={t("newModule.title")} description={t("newModule.description")} {onclose}>
  <form class="form" onsubmit={(e) => (e.preventDefault(), void create())}>
  <details class="features">
    <summary>{t("newModule.features.summary")}</summary>
    <div class="columns">
      <div>
        <strong>{t("newModule.features.can")}</strong>
        <ul>{#each moduleCan() as line (line)}<li>{line}</li>{/each}</ul>
      </div>
      <div>
        <strong>{t("newModule.features.cannot")}</strong>
        <ul>{#each moduleCannot() as line (line)}<li>{line}</li>{/each}</ul>
      </div>
    </div>
    <p class="muted">{t("newModule.features.later")}</p>
  </details>
  <div class="row" role="group" aria-labelledby={`${id}-base`}>
    <span class="label" id={`${id}-base`}>{t("newModule.base.label")}</span>
    <Button onclick={() => chooseBase("project")} disabled={busy}>
      {t("newModule.base.project")}
    </Button>
    <Button onclick={() => chooseBase("bundle")} disabled={busy}>
      {t("newModule.base.bundle")}
    </Button>
  </div>
  {#if base && basePath}
    <p class="base"><strong>{base.name}</strong> <code>{basePath}</code></p>
    {#if base.slots.length === 0}
      <p class="warn">{t("newModule.noSlots")}</p>
    {:else}
      <fieldset>
        <legend>{t("newModule.slot.label")}</legend>
        {#each base.slots as s (s.id)}
          <label class="slot">
            <input
              type="radio"
              name="slot"
              value={s.id}
              checked={slot === s.id}
              onchange={() => selectSlot(s.id)}
            />
            <strong>{s.id}</strong>
            <span class="muted">{slotSummary(s)}</span>
          </label>
        {/each}
      </fieldset>
      <TextField
        label={t("newModule.name.label")}
        value={name}
        error={name.length > 0 && !isValidModuleName(name) ? t("newModule.name.invalid") : undefined}
        oninput={(value) => {
          nameEdited = true;
          name = value;
        }}
      />
    {/if}
  {/if}
  {#if error}<p class="error ugoe-selectable" role="alert">{error}</p>{/if}
  </form>
  {#snippet actions()}
    <Button onclick={onclose}>{t("newModule.cancel.label")}</Button>
    <Button variant="primary" onclick={create} disabled={!ready}>{t("newModule.create.label")}</Button>
  {/snippet}
</Dialog>

<style>
  /* 中身の並びは Dialog の本文のまま。form は文字の欄の Enter で決定するためだけに置く
     (送信のボタンの無い form は、文字の欄が 1 つだけなら Enter で送られる: HTML の implicit submission) */
  .form {
    display: contents;
  }
  p {
    margin: 0;
  }
  .features ul {
    margin: var(--ugoe-space-2) 0;
    padding-left: var(--ugoe-space-5);
  }
  .columns {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--ugoe-space-4);
  }
  .row {
    display: flex;
    align-items: center;
    gap: var(--ugoe-space-3);
  }
  .label {
    display: block;
    min-width: 96px;
  }
  .base code {
    overflow-wrap: anywhere;
    color: var(--ugoe-color-text-muted);
  }
  fieldset {
    margin: 0;
    padding: var(--ugoe-space-2) var(--ugoe-space-3) var(--ugoe-space-3);
    border: 1px solid var(--ugoe-color-border);
    border-radius: var(--ugoe-radius-md);
  }
  .slot {
    display: flex;
    align-items: baseline;
    gap: var(--ugoe-space-2);
    padding: var(--ugoe-space-1) 0;
  }
  .muted {
    color: var(--ugoe-color-text-muted);
  }
  .warn {
    color: var(--ugoe-color-warning);
  }
  .error {
    color: var(--ugoe-color-danger);
    overflow-wrap: anywhere;
  }
</style>
