<script lang="ts">
// ファイル操作 (新規・PSD から新規・開く・保存・別名で保存・書き出し・PSD の取り込み) と
// プロジェクト名・未保存の印。パスは OS のファイルダイアログで選ぶ (docs/editor.md#ファイルダイアログ)。
// 外でのプロジェクトの書き換えも、ウィンドウに戻ったとき・一定の間隔・保存の前に確かめる (docs/editor.md#外での書き換え)。
// モジュールのプロジェクトでは、本体の変化も同じ機会に確かめ (BaseChangeDialog。「Reload automatically」なら
// 尋ねずに読み直す)、本体の問題で開けない・読み直せないときは直し方を出す (ModuleProblemBar)。開いている
// モジュールの本体・スロットの付け替えは、開き直さずに reload_base で行う (保存していない編集を保つ)。
// `Module:` の印を押すとモジュールの設定 (ModuleSettingsDialog)。「Open base project」で本体を開いている間は、
// モジュールに戻る帯 (ModuleReturnBar) を出す。
// 保存・書き出しの結果は Toast、失敗はヘッダーの下の知らせの帯 (Notice)、保存していない変更を捨てる操作は確かめのダイアログ。
// 流れ (操作を重ねないことも) は fileMenu.svelte.ts の FileMenuFlow が持ち、ここは表示と呼び出しだけ。
import { AlertDialog, Button, Notice, Popover, showToast, Tooltip } from "@ugoe/ui";
import BaseChangeDialog from "./BaseChangeDialog.svelte";
import type { EditorDocument } from "./document.svelte";
import ExportPanel from "./ExportPanel.svelte";
import ExternalChangeDialog from "./ExternalChangeDialog.svelte";
import { messageText } from "./errors";
import { CHECK_INTERVAL_MS } from "./externalChange";
import { FileMenuFlow } from "./fileMenu.svelte";
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
}

let { doc, onchange, onrenamemodule }: Props = $props();

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
  <Button variant="ghost" onclick={() => flow.newProject()} disabled={flow.busy}>{t("fileMenu.new.label")}</Button>
  <Button variant="ghost" onclick={() => flow.pick("newFromPsd")} disabled={flow.busy}>
    {t("fileMenu.newFromPsd.label")}
  </Button>
  <TipButton variant="ghost" onclick={() => flow.newModule()} disabled={flow.busy} tooltip={t("fileMenu.newModule.tooltip")}>
    {t("fileMenu.newModule.label")}
  </TipButton>
  <Button variant="ghost" onclick={() => flow.pick("open")} disabled={flow.busy}>{t("fileMenu.open.label")}</Button>
  <TipButton
    variant="ghost"
    onclick={() => flow.pick("openSample")}
    disabled={flow.busy}
    tooltip={t("fileMenu.openSample.tooltip")}>{t("fileMenu.openSample.label")}</TipButton
  >
  <Button variant="ghost" onclick={() => flow.save()} disabled={flow.busy}>{t("fileMenu.save.label")}</Button>
  <Button variant="ghost" onclick={() => flow.pick("saveAs")} disabled={flow.busy}>{t("fileMenu.saveAs.label")}</Button>
  <!-- 書き出しの設定と結果は Popover (Esc・外のクリックで閉じ、開いたボタンへフォーカスを戻す。T0907) -->
  <Popover label={t("export.aria")} bind:open={flow.exportOpen} disabled={flow.busy}>
    {#snippet trigger(attrs)}
      {#if flow.moduleInfo}
        <TipButton {...attrs} variant="ghost" disabled={flow.busy} tooltip={t("fileMenu.exportModule.tooltip")}
          >{t("fileMenu.exportModule.label")}</TipButton
        >
      {:else}
        <Button {...attrs} variant="ghost" disabled={flow.busy}>{t("fileMenu.export.label")}</Button>
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
  <Button variant="ghost" onclick={() => flow.pick("importPsd")} disabled={flow.busy}>{t("fileMenu.importPsd.label")}</Button>
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
</nav>
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
  /* 幅が足りないときはボタンを次の行へ折り返す (ヘッダーの高さが伸びる)。 */
  /* ヘッダーの残りの幅を使い、プロジェクトの名前を省略して 1 行に収める。名前の欄も最小の幅に届かないほど狭い
     ときだけ次の行へ折り返す (ヘッダーの高さが伸びる)。 */
  .file-menu {
    display: flex;
    flex: 1 1 0;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--ugoe-space-1);
    min-width: 0;
  }
  .project {
    display: inline-flex;
    /* 幅の基準を 0 にして、名前が長くてもボタンを折り返させない (余った幅だけを使う) */
    flex: 1 1 0;
    min-width: 4em;
    margin-left: var(--ugoe-space-2);
    font-weight: var(--ugoe-font-weight-strong);
    white-space: nowrap;
  }
  .project:focus-visible {
    outline: var(--ugoe-focus-ring);
    outline-offset: 2px;
  }
  .project .name {
    min-width: 0;
    max-width: 16em;
    overflow: hidden;
    text-overflow: ellipsis;
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
    padding: 0 0 0 var(--ugoe-space-5);
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
