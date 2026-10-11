<script lang="ts">
// ヘッダーのファイルの操作: 「File」のメニュー (新規・PSD から新規・新しいモジュール・開く・見本・保存・別名で保存・
// PSD の取り込み) と、よく使う保存・書き出しのボタン。プロジェクト名・未保存の印・読み書きしている操作の文
// (docs/editor.md#画面の配置)。パスは OS のファイルダイアログで選ぶ (docs/editor.md#ファイルダイアログ)。
// 外でのプロジェクトの書き換えも、ウィンドウに戻ったとき・一定の間隔・保存の前に確かめる (docs/editor.md#外での書き換え)。
// モジュールのプロジェクトでは、本体の変化も同じ機会に確かめ (BaseChangeDialog。「Reload automatically」なら
// 尋ねずに読み直す)、本体の問題で開けない・読み直せないときは直し方を出す (ModuleProblemBar)。開いている
// モジュールの本体・スロットの付け替えは、開き直さずに reload_base で行う (保存していない編集を保つ)。
// `Module:` の印を押すとモジュールの設定 (ModuleSettingsDialog)。「Open base project」で本体を開いている間は、
// モジュールに戻る帯 (ModuleReturnBar) を出す。
// 保存・書き出しの結果は Toast、失敗はヘッダーの下の知らせの帯 (Notice)、保存していない変更を捨てる操作は確かめのダイアログ。
// 流れ (操作を重ねないことも) は fileMenuFlow.svelte.ts の FileMenuFlow が持ち、ここは表示と呼び出しだけ。
import {
  AlertDialog,
  Button,
  Menu,
  type MenuEntry,
  Notice,
  Popover,
  showToast,
  Tooltip,
} from "@ugoe/ui";
import type { Snippet } from "svelte";
import BaseChangeDialog from "./BaseChangeDialog.svelte";
import type { EditorDocument } from "./document.svelte";
import ExportPanel from "./ExportPanel.svelte";
import ExternalChangeDialog from "./ExternalChangeDialog.svelte";
import { messageText } from "./errors";
import { CHECK_INTERVAL_MS } from "./externalChange";
import { type FileActivity, FileMenuFlow, type PathAction } from "./fileMenuFlow.svelte";
import { parentDir } from "./filePaths";
import { t } from "./i18n";
import ModuleProblemBar from "./ModuleProblemBar.svelte";
import ModuleReturnBar from "./ModuleReturnBar.svelte";
import ModuleSettingsDialog from "./ModuleSettingsDialog.svelte";
import { moduleLabel, moduleTitle } from "./moduleProject";
import NewModuleDialog from "./NewModuleDialog.svelte";
import TipButton from "./ui/TipButton.svelte";

interface Props {
  /** 編集を通すドキュメント (モジュールの設定の適用)。 */
  doc: EditorDocument;
  /**
   * 新規作成・開く・PSD の取り込みでモデルが変わったときに呼ぶ。`replaced` は別のモデルに替わったこと
   * (新規・PSD から新規・開く。パラメータのスライダーを既定値に戻す)。PSD の取り込みは `false`。
   * `reloaded` は同じプロジェクトを開き直したこと (外での書き換えの後。表示の位置を保つ)。
   */
  onchange: (replaced: boolean, reloaded?: boolean) => void;
  /** モジュールの名前を変えた (作業用のモデルの ID の接頭辞が変わる。読み直す前に選択を付け替える)。 */
  onrenamemodule?: (from: string, to: string) => void;
  /** ファイルの操作の後、プロジェクトの名前の前に並べるもの (Undo・Redo)。 */
  children?: Snippet;
}

let { doc, onchange, onrenamemodule, children }: Props = $props();

const flow = new FileMenuFlow({
  onchange: (replaced, reloaded) => onchange(replaced, reloaded),
  onrenamemodule: (from, to) => onrenamemodule?.(from, to),
  applyWith: (action, options) => doc.applyWith(action, options),
  inform: (text, long) => showToast(text, { tone: "success", duration: long ? 10000 : 5000 }),
});

/** 状態 (名前・未保存の印) を取り直す。編集の後に親から呼ぶ。 */
export function refresh(): Promise<void> {
  return flow.refresh();
}

/** パスを選んで行う操作 (空の状態の導線・ショートカットから)。 */
export function pick(kind: PathAction): Promise<void> {
  return flow.pick(kind);
}

/** 保存する (ショートカットから)。 */
export function save(): Promise<void> {
  return flow.save();
}

/** 「File」のメニュー。よく使う保存・書き出しはヘッダーにも並べる。 */
const entries = $derived<MenuEntry[]>([
  { label: t("fileMenu.new.label"), onselect: () => void flow.newProject() },
  { label: t("fileMenu.newFromPsd.label"), onselect: () => void flow.pick("newFromPsd") },
  {
    label: t("fileMenu.newModule.label"),
    description: t("fileMenu.newModule.tooltip"),
    onselect: () => void flow.newModule(),
  },
  { kind: "separator" },
  { label: t("fileMenu.open.label"), shortcut: "Mod+O", onselect: () => void flow.pick("open") },
  {
    label: t("fileMenu.openSample.label"),
    description: t("fileMenu.openSample.tooltip"),
    onselect: () => void flow.pick("openSample"),
  },
  { kind: "separator" },
  { label: t("fileMenu.save.label"), shortcut: "Mod+S", onselect: () => void flow.save() },
  {
    label: t("fileMenu.saveAs.label"),
    shortcut: "Mod+Shift+S",
    onselect: () => void flow.pick("saveAs"),
  },
  { kind: "separator" },
  { label: t("fileMenu.importPsd.label"), onselect: () => void flow.pick("importPsd") },
]);

/** 読み書きしている操作の文。 */
function activityText(activity: FileActivity): string {
  switch (activity) {
    case "save":
    case "saveAs":
      return t("fileMenu.activity.saving");
    case "export":
    case "exportModule":
      return t("fileMenu.activity.exporting");
    case "newFromPsd":
    case "importPsd":
      return t("fileMenu.activity.importing");
    case "open":
    case "openSample":
    case "locateBaseProject":
    case "locateBaseBundle":
      return t("fileMenu.activity.opening");
  }
}

/** 開いているモジュールの本体のプロジェクトを開く (適用のエラーの案内から。T1609)。 */
export function openModuleBase(): Promise<void> {
  return flow.openModuleBase();
}

$effect(() => {
  const timer = setInterval(() => {
    if (document.visibilityState === "visible") void flow.checkExternal();
  }, CHECK_INTERVAL_MS);
  return () => clearInterval(timer);
});

flow.refresh();
</script>

<svelte:window onfocus={() => void flow.checkExternal()} />

<nav class="file-menu" aria-label={t("fileMenu.aria")}>
  <Menu label={t("fileMenu.menu.label")} variant="ghost" entries={entries} disabled={flow.busy} />
  <TipButton variant="ghost" onclick={() => flow.save()} disabled={flow.busy} tooltip={t("fileMenu.save.tooltip")}
    shortcut="Mod+S">{t("fileMenu.save.label")}</TipButton
  >
  <!-- 書き出しの設定と結果は Popover (Esc・外のクリックで閉じ、開いたボタンへフォーカスを戻す。T0907) -->
  <Popover label={t("export.aria")} bind:open={flow.exportOpen} disabled={flow.busy}>
    {#snippet trigger(attrs)}
      {#if flow.moduleInfo}
        <TipButton {...attrs} variant="ghost" disabled={flow.busy} tooltip={t("fileMenu.exportModule.tooltip")}
          >{t("fileMenu.exportModule.label")}</TipButton
        >
      {:else}
        <TipButton {...attrs} variant="ghost" disabled={flow.busy} tooltip={t("fileMenu.export.tooltip")}
          >{t("fileMenu.export.label")}</TipButton
        >
      {/if}
    {/snippet}
    <ExportPanel
      busy={flow.busy}
      summary={flow.exportLines}
      label={flow.moduleInfo ? t("fileMenu.export.module") : t("fileMenu.export.bundle")}
      onexport={() => flow.pick(flow.moduleInfo ? "exportModule" : "export")}
      onstatus={(s) => flow.setStatus(s)}
    />
  </Popover>
</nav>
{@render children?.()}
<div class="project-info">
  <!-- 長い名前は省略し、ツールチップに全体と保存先のパスを出す。未保存の印は省略されない。 -->
  <Tooltip
    text={flow.status?.path
      ? t("fileMenu.project.tooltip", { name: flow.status.name, path: flow.status.path })
      : t("fileMenu.project.tooltipUnsaved", { name: flow.status?.name ?? "…" })}
  >
    {#snippet trigger(attrs)}
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <span {...attrs} class="project" tabindex="0">
        <span class="name">{flow.status?.name ?? "…"}</span>{#if flow.status?.modified}<span
            class="dirty"
            role="img"
            aria-label={t("fileMenu.project.modified")}>●</span
          >{/if}
      </span>
    {/snippet}
  </Tooltip>
  {#if flow.moduleInfo}
    <TipButton small class="module" tooltip={moduleTitle(flow.moduleInfo)} onclick={() => flow.openSettings()} disabled={flow.busy}
      >{moduleLabel(flow.moduleInfo)}</TipButton
    >
  {/if}
  {#if flow.warnings.length > 0}
    <Popover label={t("fileMenu.warnings.aria")}>
      {#snippet trigger(attrs)}
        <Button {...attrs} variant="ghost" class="warnings">
          {t("fileMenu.warnings.label", { count: flow.warnings.length })}
        </Button>
      {/snippet}
      <ul class="warning-list">
        {#each flow.warnings as w, i (i)}
          <li>
            {w.path.length > 0
              ? t("fileMenu.warnings.item", { layer: w.path.join(" / "), message: messageText(w) })
              : messageText(w)}
          </li>
        {/each}
      </ul>
    </Popover>
  {/if}
  <!-- 読み書きしている操作 (「保存しています…」)。読み上げにも知らせる。 -->
  <span class="activity" role="status">{flow.activity ? activityText(flow.activity) : ""}</span>
</div>
<!-- 知らせの帯: ヘッダーの下に重ねる (モジュールに戻る道・本体の問題・失敗の文)。 -->
{#if flow.returnTo || flow.problem || flow.message}
<div class="bands">
{#if flow.returnTo}
  <ModuleReturnBar
    back={flow.returnTo}
    modified={flow.status?.modified ?? false}
    busy={flow.busy}
    onreturn={() => flow.returnToModule()}
    ondismiss={() => flow.dismissReturn()}
  />
{/if}
{#if flow.problem}
  <ModuleProblemBar
    problem={flow.problem.problem}
    canReopen={!flow.problem.opening || flow.problem.project !== null}
    busy={flow.busy}
    onlocate={(kind) => flow.pick(kind === "project" ? "locateBaseProject" : "locateBaseBundle")}
    onchooseslot={(slot) => flow.useSlot(slot)}
    onopenbase={() => flow.openBase()}
    ondropcolliders={() => flow.dropColliders()}
    ondismiss={() => flow.dismissProblem()}
  />
{/if}
{#if flow.message}
  <Notice tone="danger" ondismiss={() => flow.dismissMessage()}><p>{flow.message}</p></Notice>
{/if}
</div>
{/if}
<AlertDialog
  open={flow.pendingDiscard !== null}
  title={t("fileMenu.discard.title")}
  description={t("fileMenu.discard.description")}
  confirmLabel={t("fileMenu.discard.confirm")}
  onconfirm={() => flow.confirmDiscard()}
  oncancel={() => flow.cancelDiscard()}
/>
<ModuleSettingsDialog
  open={flow.settingsOpen}
  module={flow.moduleInfo}
  onapply={(command) => flow.applyModuleSettings(command)}
  onopenbase={(rejected) => flow.openModuleBase(rejected)}
  onclose={() => flow.closeSettings()}
  retry={flow.settingsRetry}
/>
<BaseChangeDialog
  change={flow.baseChange}
  base={flow.moduleInfo?.base ?? ""}
  autoReload={flow.autoReload}
  onautoreload={(enabled) => flow.setAutoReload(enabled)}
  onreload={() => flow.reloadBase()}
  onlocate={(kind) => flow.pick(kind === "project" ? "locateBaseProject" : "locateBaseBundle")}
  ondismiss={() => flow.dismissBase()}
/>
<NewModuleDialog
  open={flow.newModuleOpen}
  startDir={flow.lastDir ?? (flow.status?.path ? parentDir(flow.status.path) : null)}
  oncreate={(base, slot, name) => flow.createModule(base, slot, name)}
  onclose={() => (flow.newModuleOpen = false)}
/>
<ExternalChangeDialog
  change={flow.external}
  modified={flow.status?.modified ?? false}
  saving={flow.externalOnSave}
  autoReload={flow.autoReload}
  onautoreload={(enabled) => flow.setAutoReload(enabled)}
  onreload={() => flow.reloadProject()}
  onoverwrite={() => flow.overwriteExternal()}
  onsaveas={() => flow.saveAsExternal()}
  ondismiss={() => flow.dismissExternal()}
/>

<style>
  .file-menu {
    display: flex;
    align-items: center;
    gap: var(--ugoe-space-1);
  }
  /* ヘッダーの残りの幅を使い、プロジェクトの名前を省略して 1 行に収める (ボタンは縮めない)。 */
  .project-info {
    display: flex;
    flex: 1 1 0;
    align-items: center;
    gap: var(--ugoe-space-2);
    /* 縮めるのは名前 (省略記号) まで。モジュールのボタンなどが入らなければ、重ねずにヘッダーを折り返す (日本語の 200% の拡大。T0918) */
    min-width: min-content;
  }
  .project-info > :global(*) {
    flex-shrink: 0;
  }
  .project-info > .project {
    display: inline-flex;
    flex-shrink: 1;
    min-width: 4em;
    margin-left: var(--ugoe-space-2);
    font-weight: var(--ugoe-font-weight-strong);
    white-space: nowrap;
  }
  .project:focus-visible {
    outline: var(--ugoe-focus-ring);
    outline-offset: var(--ugoe-focus-offset);
  }
  .project .name {
    min-width: 0;
    max-width: 16em;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .activity {
    color: var(--ugoe-color-text-muted);
    white-space: nowrap;
  }
  .file-menu :global(.module) {
    border-color: var(--ugoe-color-accent);
    color: var(--ugoe-color-accent);
  }
  .dirty {
    flex-shrink: 0;
    margin-left: var(--ugoe-space-2);
    color: var(--ugoe-color-accent);
  }
  .file-menu :global(.warnings) {
    color: var(--ugoe-color-warning);
  }
  .warning-list {
    max-width: 480px;
    max-height: 50vh;
    margin: 0;
    padding: 0 var(--ugoe-scrollbar-gap) 0 var(--ugoe-space-5);
    overflow: auto;
    list-style: disc;
  }
  /* ヘッダーの中の最後の行 (App.svelte の header は折り返す)。重ねずに場所を取るので、ビューポートのボタンを隠さない。 */
  .bands {
    display: flex;
    flex: 0 0 100%;
    order: 1;
    flex-direction: column;
    align-items: center;
    gap: var(--ugoe-space-2);
    padding-bottom: var(--ugoe-space-1);
    white-space: normal;
  }
  .bands > :global(*) {
    width: min(720px, 100%);
  }
  .bands p {
    margin: 0;
    overflow-wrap: anywhere;
  }
</style>
