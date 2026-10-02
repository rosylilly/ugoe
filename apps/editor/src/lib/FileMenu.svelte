<script lang="ts">
// ファイル操作 (新規・PSD から新規・開く・保存・別名で保存・書き出し・PSD の取り込み) と
// プロジェクト名・未保存の印。パスは OS のファイルダイアログで選ぶ (docs/editor.md#ファイルダイアログ)。
// 外でのプロジェクトの書き換えも、ウィンドウに戻ったとき・一定の間隔・保存の前に確かめる (docs/editor.md#外での書き換え)。
// モジュールのプロジェクトでは、本体の変化も同じ機会に確かめ (BaseChangeDialog。「Reload automatically」なら
// 尋ねずに読み直す)、本体の問題で開けない・読み直せないときは直し方を出す (ModuleProblemBar)。開いている
// モジュールの本体・スロットの付け替えは、開き直さずに reload_base で行う (保存していない編集を保つ)。
// `Module:` の印を押すとモジュールの設定 (ModuleSettingsDialog)。
// 保存・書き出しの結果は Toast、失敗はヘッダーの下の知らせの帯 (Notice)、保存していない変更を捨てる操作は確かめのダイアログ。
import { AlertDialog, Button, Notice, Popover, showToast, Tooltip } from "@ugoe/ui";
import BaseChangeDialog from "./BaseChangeDialog.svelte";
import ExportPanel from "./ExportPanel.svelte";
import ExternalChangeDialog from "./ExternalChangeDialog.svelte";
import { errorText, messageText } from "./errors";
import { exportSummary, moduleExportSummary } from "./exportSummary";
import {
  baseAction,
  CHECK_INTERVAL_MS,
  externalAction,
  isExternalChangeError,
  loadAutoReload,
  saveAutoReload,
} from "./externalChange";
import { fileDialogs } from "./fileDialogs";
import {
  BUNDLE_FILTER,
  joinPath,
  MODULE_FILTER,
  PROJECT_FILTER,
  PSD_FILTER,
  parentDir,
  safeFileName,
  withExtension,
} from "./filePaths";
import { t } from "./i18n";
import {
  type Command,
  type ExternalChange,
  ipc,
  type ModuleProblem,
  type ModuleStatus,
  type ProjectStatus,
  type PsdImportWarning,
} from "./ipc";
import ModuleProblemBar from "./ModuleProblemBar.svelte";
import ModuleSettingsDialog from "./ModuleSettingsDialog.svelte";
import { droppedCollidersNotice, moduleLabel, moduleTitle } from "./moduleProject";
import NewModuleDialog from "./NewModuleDialog.svelte";
import TipButton from "./ui/TipButton.svelte";

interface Props {
  /**
   * 新規作成・開く・PSD の取り込みでモデルが変わったときに呼ぶ。`replaced` は別のモデルに替わったこと
   * (新規・PSD から新規・開く。パラメータのスライダーを既定値に戻す)。PSD の取り込みは `false`。
   * `reloaded` は同じプロジェクトを開き直したこと (外での書き換えの後。表示の位置を保つ)。
   */
  onchange: (replaced: boolean, reloaded?: boolean) => void;
  /** モジュールの名前を変えた (作業用のモデルの ID の接頭辞が変わる。読み直す前に選択を付け替える)。 */
  onrenamemodule?: (from: string, to: string) => void;
}

let { onchange, onrenamemodule }: Props = $props();

type PathAction =
  | "newFromPsd"
  | "open"
  | "openSample"
  | "saveAs"
  | "export"
  | "exportModule"
  | "importPsd"
  | "locateBaseProject"
  | "locateBaseBundle";

let status = $state<ProjectStatus | null>(null);
/** モジュールのプロジェクトなら、本体とスロット (docs/editor.md#モジュールのプロジェクト)。 */
let moduleInfo = $state<ModuleStatus | null>(null);
/** 新しいモジュールの画面を開いているか。 */
let newModuleOpen = $state(false);
/**
 * 本体の問題で開けなかった・読み直せなかったモジュールのプロジェクトと、その問題 (直し方を出す)。
 * `project` は付け替えて開き直すプロジェクトの場所 (未保存なら null)。`opening` は開けなかったこと
 * (読み直せなかっただけなら、開き直さずに reload_base で付け替えるので、保存していない編集は残る)。
 * `base` は選び直した本体 (それでも開けなかったとき。次の直し方でも使う)。
 */
let problem = $state<{
  project: string | null;
  problem: ModuleProblem;
  opening: boolean;
  base: string | null;
} | null>(null);
/** モジュールの設定の画面を開いているか。 */
let settingsOpen = $state(false);
/** 本体の変化 (確認のダイアログを開いている間だけ)。 */
let baseChange = $state<ExternalChange | null>(null);
/** 「後で」を選んだ本体の状態の目印。 */
let dismissedBaseToken: string | null = null;
/** ダイアログを開いているか、その後の処理の途中か。 */
let busy = $state(false);
/** 未保存の変更を破棄する操作 (確かめのダイアログで「Discard」を選んだら続ける)。 */
let pendingDiscard = $state<(() => void) | null>(null);
/** 失敗の文 (知らせの帯に出す)。 */
let message = $state<string | null>(null);
/** 最後の PSD の取り込みの警告。 */
let warnings = $state<PsdImportWarning[]>([]);
/** 書き出しの設定の欄を開いているか。 */
let exportOpen = $state(false);
/** 最後の書き出し (.ugoe か .ugoemod) の結果の要約の行 (テクスチャの枚数と状態の切り替えの変化・アトラス化)。 */
let exportLines = $state<string[] | null>(null);
/** 最後にダイアログで選んだ場所 (次のダイアログの最初の場所)。 */
let lastDir = $state<string | null>(null);
/** 外での書き換え (確認のダイアログを開いている間だけ)。 */
let external = $state<ExternalChange | null>(null);
/** 保存しようとして外での書き換えが見つかったか (ダイアログで上書きを選べる)。 */
let externalOnSave = $state(false);
/** 「後で」を選んだ外の状態の目印 (同じ変更を二度尋ねない。保存の前は尋ねる)。 */
let dismissedToken: string | null = null;
let autoReload = $state(loadAutoReload());

/** 状態 (名前・未保存の印) を取り直す。編集の後に親から呼ぶ。 */
export async function refresh() {
  try {
    status = await ipc.projectStatus();
    moduleInfo = await ipc.moduleStatus();
  } catch (e) {
    message = errorText(e);
  }
}

/**
 * 成功した後のモデルの読み直し: しない、同じモデルの続き (取り込み)、別のモデルに替わった、
 * 同じプロジェクトを開き直した (外での書き換えの後)。
 */
type Reload = "none" | "same" | "replaced" | "reloaded";

/** 成功したら `true`。失敗はメッセージに出す。 */
async function run(
  task: () => Promise<ProjectStatus | undefined>,
  reload: Reload,
): Promise<boolean> {
  try {
    const result = await task();
    if (result) status = result;
    message = null;
    moduleInfo = await ipc.moduleStatus();
    if (reload === "replaced") {
      // 別のプロジェクトの設定と結果を見せないよう、書き出しの欄を閉じる
      exportOpen = false;
      exportLines = null;
    }
    if (reload !== "none") onchange(reload === "replaced", reload === "reloaded");
    return true;
  } catch (e) {
    message = errorText(e);
    return false;
  }
}

/** 保存・書き出しなどの結果を知らせる (Toast)。`long` は読むのに時間のかかる案内。 */
function inform(text: string, long = false) {
  showToast(text, { tone: "success", duration: long ? 10000 : 5000 });
}

/** 保存していない変更を捨てる操作: 変更があれば確かめてから `proceed` を呼ぶ。 */
function withDiscard(proceed: () => void) {
  if (!status?.modified) {
    proceed();
    return;
  }
  pendingDiscard = proceed;
}

function newProject() {
  withDiscard(() => void run(ipc.newProject, "replaced"));
}

function newModule() {
  withDiscard(() => {
    newModuleOpen = true;
  });
}

async function createModule(base: string, slot: string, name: string) {
  // 失敗はダイアログに出す (例外をそのまま返す)
  status = await ipc.newModuleProject(base, slot, name);
  newModuleOpen = false;
  warnings = [];
  problem = null;
  await run(async () => undefined, "replaced");
}

/** ダイアログの最初の場所: 最後に選んだ場所、プロジェクトの隣、ドキュメント (なければホーム) の順。 */
async function startDir(): Promise<string | null> {
  return (
    lastDir ??
    (status?.path ? parentDir(status.path) : null) ??
    (await fileDialogs().defaultDirectory())
  );
}

/** 操作に合うダイアログを開き、選ばれたパスを返す。取り消したら `null`。 */
async function choose(kind: PathAction): Promise<string | null> {
  const dialogs = fileDialogs();
  const dir = (await startDir()) ?? undefined;
  /** ファイル名だけを既定にするときの、場所を付けたパス。 */
  const suggest = (name: string) => (dir ? joinPath(dir, name) : name);
  const name = safeFileName(status?.name ?? "Untitled");
  switch (kind) {
    case "newFromPsd":
      return dialogs.openFile({
        title: t("fileMenu.dialog.newFromPsd"),
        defaultPath: dir,
        filters: [PSD_FILTER],
      });
    case "importPsd":
      return dialogs.openFile({
        title: t("fileMenu.dialog.importPsd"),
        defaultPath: dir,
        filters: [PSD_FILTER],
      });
    case "open":
      // .ugoeproj はディレクトリなので、フォルダーを選ぶ
      return dialogs.openDirectory({ title: t("fileMenu.dialog.open"), defaultPath: dir });
    case "openSample":
      return dialogs.openDirectory({ title: t("fileMenu.dialog.openSample"), defaultPath: dir });
    case "saveAs": {
      const path = await dialogs.saveFile({
        title: t("fileMenu.dialog.saveAs"),
        defaultPath: suggest(`${name}.ugoeproj`),
        filters: [PROJECT_FILTER],
      });
      return path && withExtension(path, "ugoeproj");
    }
    case "exportModule": {
      const path = await dialogs.saveFile({
        title: t("fileMenu.dialog.exportModule"),
        defaultPath: suggest(`${safeFileName(moduleInfo?.name ?? name)}.ugoemod`),
        filters: [MODULE_FILTER],
      });
      return path && withExtension(path, "ugoemod");
    }
    case "locateBaseProject":
      return dialogs.openDirectory({ title: t("fileMenu.dialog.locateProject"), defaultPath: dir });
    case "locateBaseBundle":
      return dialogs.openFile({
        title: t("fileMenu.dialog.locateBundle"),
        defaultPath: dir,
        filters: [BUNDLE_FILTER],
      });
    case "export": {
      const path = await dialogs.saveFile({
        title: t("fileMenu.dialog.export"),
        defaultPath: suggest(`${name}.ugoe`),
        filters: [BUNDLE_FILTER],
      });
      return path && withExtension(path, "ugoe");
    }
  }
}

/** 選んだパスで操作を実行する。成功したら `true`。 */
function perform(kind: PathAction, path: string): Promise<boolean> {
  switch (kind) {
    case "newFromPsd":
      return run(async () => {
        warnings = [];
        warnings = await ipc.newProjectFromPsd(path);
        return ipc.projectStatus();
      }, "replaced");
    case "importPsd":
      return run(async () => {
        warnings = [];
        warnings = await ipc.importPsd(path);
        return ipc.projectStatus();
      }, "same");
    case "open":
      return run(() => ipc.openProject(path), "replaced").then(async (ok) => {
        problem = null;
        // モジュールのプロジェクトが本体の問題で開けなければ、直し方を出す
        if (!ok) {
          const found = await ipc.moduleOpenProblem(path).catch(() => null);
          if (found) {
            problem = { project: path, problem: found, opening: true, base: null };
            message = null;
          }
        }
        return ok;
      });
    case "openSample":
      // 選んだフォルダーの中に新しいフォルダーを作って書き出し、本体を開く (docs/manual/dress-up.md#見本のプロジェクトを開く)
      return run(() => ipc.openSampleProject(path), "replaced").then((ok) => {
        if (ok) {
          problem = null;
          const folder = status?.path ? parentDir(status.path) : null;
          inform(
            folder
              ? t("fileMenu.notice.sampleOpenedIn", { folder })
              : t("fileMenu.notice.sampleOpened"),
            true,
          );
        }
        return ok;
      });
    case "locateBaseProject":
    case "locateBaseBundle":
      // 開いているモジュールなら開き直さずに付け替える (保存していない編集を保つ)
      return problem?.opening ? reopenModule(path, null) : relocateBase(path, null);
    case "exportModule":
      return run(async () => {
        const report = await ipc.exportModule(path);
        warnings = report.warnings.map((w) => ({
          path: [],
          code: w.code,
          args: w.args,
          message: w.message,
        }));
        exportLines = moduleExportSummary(report);
        inform(t("fileMenu.notice.exported", { path }));
        return undefined;
      }, "none");
    case "saveAs":
      return run(async () => {
        const result = await ipc.saveProjectAs(path);
        inform(t("fileMenu.notice.saved", { path: result.path ?? path }));
        return result;
      }, "none");
    case "export":
      return run(async () => {
        exportLines = exportSummary(await ipc.exportBundle(path));
        inform(t("fileMenu.notice.exported", { path }));
        return undefined;
      }, "none");
  }
}

/**
 * 本体の問題で開けなかったモジュールのプロジェクトを、本体 `base` かスロット `slot` を付け替えて開き直す
 * (`base` が null なら、前に選び直した本体があればそれ)。それでも本体の問題で開けなければ、新しい問題を出す。
 */
async function reopenModule(
  base: string | null,
  slot: string | null,
  dropColliders = false,
): Promise<boolean> {
  const project = problem?.project;
  if (!project) return false;
  const useBase = base ?? problem?.base ?? null;
  const dropped = dropColliders ? (problem?.problem.collider_references ?? []) : [];
  const ok = await run(
    () => ipc.openModuleWithBase(project, useBase, slot, dropColliders),
    "replaced",
  );
  if (ok) {
    problem = null;
    inform(
      dropColliders
        ? droppedCollidersNotice(dropped)
        : slot === null
          ? t("fileMenu.notice.openedWithBase")
          : t("fileMenu.notice.openedOnSlot", { slot }),
      true,
    );
  } else {
    const found = await ipc.moduleOpenProblem(project, useBase).catch(() => null);
    if (found) {
      problem = { project, problem: found, opening: true, base: useBase };
      message = null;
    }
  }
  return ok;
}

/**
 * 開いているモジュールのプロジェクトの本体 `base` かスロット `slot` を、開き直さずに付け替える (`base` が null なら、
 * 前に選び直した本体があればそれ)。保存していない編集は残る。付け替えられなければ、新しい問題を出す。
 */
async function relocateBase(base: string | null, slot: string | null): Promise<boolean> {
  const useBase = base ?? problem?.base ?? null;
  baseChange = null;
  try {
    const result = await ipc.reloadBase(useBase, slot);
    status = result.status;
    if (result.problem) {
      problem = { project: status.path, problem: result.problem, opening: false, base: useBase };
      message = null;
      return false;
    }
    problem = null;
    await run(async () => undefined, "reloaded");
    inform(
      slot === null ? t("fileMenu.notice.usingBase") : t("fileMenu.notice.movedToSlot", { slot }),
      true,
    );
    return true;
  } catch (e) {
    message = errorText(e);
    return false;
  }
}

/** 本体の問題の枠の「Use this slot」。 */
async function useSlot(slot: string) {
  if (busy) return;
  busy = true;
  try {
    if (problem?.opening) await reopenModule(null, slot);
    else await relocateBase(null, slot);
  } finally {
    busy = false;
  }
}

/** 本体の問題の枠の「Remove and open」: 確かめた衝突体の参照を外して開き直す (T0661)。 */
async function dropColliders() {
  if (busy || !problem?.opening) return;
  busy = true;
  try {
    await reopenModule(null, null, true);
  } finally {
    busy = false;
  }
}

/** 本体の問題の枠の「Open base project」: 本体のプロジェクトを開いて直す (モジュールは後で開き直す)。 */
function openBase() {
  const base = problem?.problem.base;
  if (!base || busy) return;
  withDiscard(() => void openBaseNow(base));
}

async function openBaseNow(base: string) {
  busy = true;
  try {
    if (await run(() => ipc.openProject(base), "replaced")) {
      problem = null;
      inform(t("fileMenu.notice.openedBase"), true);
    }
  } finally {
    busy = false;
  }
}

/** モジュールの設定の画面の「Apply」(失敗は例外のまま画面に返す)。 */
async function applyModuleSettings(command: Command) {
  await ipc.applyCommand(command);
  settingsOpen = false;
  const from = moduleInfo?.name;
  if (command.op === "set_module_settings" && from && from !== command.name) {
    onrenamemodule?.(from, command.name);
  }
  await run(ipc.projectStatus, "reloaded");
}

function pick(kind: PathAction) {
  if (busy) return;
  if (kind === "open" || kind === "openSample" || kind === "newFromPsd") {
    withDiscard(() => void pickNow(kind));
  } else void pickNow(kind);
}

async function pickNow(kind: PathAction) {
  if (busy) return;
  busy = true;
  try {
    const path = await choose(kind);
    message = null;
    if (path && (await perform(kind, path))) {
      // 見本なら、次の「Open…」をモジュールのプロジェクトの並ぶフォルダーから始める
      const opened = kind === "openSample" && status?.path ? status.path : path;
      lastDir = parentDir(opened) ?? lastDir;
    }
  } catch (e) {
    message = errorText(e);
  } finally {
    busy = false;
  }
}

async function save() {
  if (!status?.path) return pick("saveAs");
  if (busy) return;
  // 保存の前に確かめ、外で変わっていれば選ばせる (バックエンドも保存の直前に確かめて断る)
  const change = await ipc.checkExternalChange().catch(() => null);
  if (change) {
    askExternal(change, true);
    return;
  }
  /** バックエンドが外での書き換えを見つけて保存を断ったか。 */
  let refused = false;
  const saved = await run(async () => {
    try {
      const result = await ipc.saveProject();
      inform(t("fileMenu.notice.saved", { path: result.path ?? "" }));
      return result;
    } catch (e) {
      refused = isExternalChangeError(e);
      throw e;
    }
  }, "none");
  if (!saved && refused) {
    const again = await ipc.checkExternalChange().catch(() => null);
    if (again) {
      message = null;
      askExternal(again, true);
    }
  }
}

function askExternal(change: ExternalChange, onSave: boolean) {
  externalOnSave = onSave;
  external = change;
}

/** ウィンドウに戻ったとき・一定の間隔で、外での書き換えと、モジュールのプロジェクトの本体の変化を確かめる。 */
async function checkExternal() {
  await checkBase();
  if (busy || external || baseChange || !status?.path) return;
  let change: ExternalChange | null;
  try {
    change = await ipc.checkExternalChange();
  } catch {
    return; // 書き込みの途中などで読めなければ、次の機会に確かめる
  }
  if (busy || external) return;
  const action = externalAction(change, {
    modified: status?.modified ?? false,
    autoReload,
    dismissed: dismissedToken,
  });
  if (action === "reload") await reloadProject(true);
  else if (action === "ask" && change) askExternal(change, false);
}

/**
 * モジュールのプロジェクトの本体が変わったか確かめ、変わっていれば尋ねる (保存していなくても確かめる)。
 * 「Reload automatically」なら尋ねずに読み直す (保存していないモジュールの編集は残る)。
 */
async function checkBase() {
  if (busy || external || baseChange || problem || !moduleInfo) return;
  const change = await ipc.checkBaseChange().catch(() => null);
  if (busy || external) return;
  const action = baseAction(change, { autoReload, dismissed: dismissedBaseToken });
  if (action === "reload" && change) {
    baseChange = change;
    await reloadBase(true);
  } else if (action === "ask") baseChange = change;
}

/** 本体を読み直す。読み直せなければ、問題と直し方を出す (モジュールはそのまま)。 */
async function reloadBase(auto = false) {
  const token = baseChange?.token ?? null;
  baseChange = null;
  dismissedBaseToken = null;
  busy = true;
  try {
    const result = await ipc.reloadBase();
    status = result.status;
    if (result.problem) {
      problem = { project: status.path, problem: result.problem, opening: false, base: null };
      message = null;
      // 読み直せなかった同じ本体の状態は、もう尋ねない (直し方の枠で選ぶ)
      dismissedBaseToken = token;
      return;
    }
    problem = null;
    await run(async () => undefined, "reloaded");
    inform(auto ? t("fileMenu.notice.baseReloadedAuto") : t("fileMenu.notice.baseReloaded"));
  } catch (e) {
    message = errorText(e);
  } finally {
    busy = false;
  }
}

function dismissBase() {
  if (baseChange) dismissedBaseToken = baseChange.token;
  baseChange = null;
}

async function reloadProject(auto: boolean) {
  external = null;
  dismissedToken = null;
  if (await run(ipc.reloadProject, "reloaded")) {
    inform(auto ? t("fileMenu.notice.reloadedAuto") : t("fileMenu.notice.reloaded"));
  }
}

async function overwriteExternal() {
  external = null;
  dismissedToken = null;
  await run(async () => {
    const result = await ipc.saveProject(true);
    inform(t("fileMenu.notice.saved", { path: result.path ?? "" }));
    return result;
  }, "none");
}

function saveAsExternal() {
  if (external) dismissedToken = external.token;
  external = null;
  pick("saveAs");
}

function dismissExternal() {
  if (external) dismissedToken = external.token;
  external = null;
}

function setAutoReload(enabled: boolean) {
  autoReload = enabled;
  saveAutoReload(enabled);
}

$effect(() => {
  const timer = setInterval(() => {
    if (document.visibilityState === "visible") checkExternal();
  }, CHECK_INTERVAL_MS);
  return () => clearInterval(timer);
});

refresh();
</script>

<svelte:window onfocus={() => checkExternal()} />

<nav class="file-menu" aria-label={t("fileMenu.aria")}>
  <Button variant="ghost" onclick={newProject} disabled={busy}>{t("fileMenu.new.label")}</Button>
  <Button variant="ghost" onclick={() => pick("newFromPsd")} disabled={busy}>
    {t("fileMenu.newFromPsd.label")}
  </Button>
  <TipButton variant="ghost" onclick={newModule} disabled={busy} tooltip={t("fileMenu.newModule.tooltip")}>
    {t("fileMenu.newModule.label")}
  </TipButton>
  <Button variant="ghost" onclick={() => pick("open")} disabled={busy}>{t("fileMenu.open.label")}</Button>
  <TipButton
    variant="ghost"
    onclick={() => pick("openSample")}
    disabled={busy}
    tooltip={t("fileMenu.openSample.tooltip")}>{t("fileMenu.openSample.label")}</TipButton
  >
  <Button variant="ghost" onclick={save} disabled={busy}>{t("fileMenu.save.label")}</Button>
  <Button variant="ghost" onclick={() => pick("saveAs")} disabled={busy}>{t("fileMenu.saveAs.label")}</Button>
  <span class="export">
    {#if moduleInfo}
      <TipButton
        variant="ghost"
        onclick={() => (exportOpen = !exportOpen)}
        disabled={busy}
        aria-expanded={exportOpen}
        tooltip={t("fileMenu.exportModule.tooltip")}>{t("fileMenu.exportModule.label")}</TipButton
      >
    {:else}
      <Button
        variant="ghost"
        onclick={() => (exportOpen = !exportOpen)}
        disabled={busy}
        aria-expanded={exportOpen}>{t("fileMenu.export.label")}</Button
      >
    {/if}
    {#if exportOpen}
      <ExportPanel
        {busy}
        summary={exportLines}
        label={moduleInfo ? t("fileMenu.export.module") : t("fileMenu.export.bundle")}
        onexport={() => pick(moduleInfo ? "exportModule" : "export")}
        onstatus={(s) => (status = s)}
      />
    {/if}
  </span>
  <Button variant="ghost" onclick={() => pick("importPsd")} disabled={busy}>{t("fileMenu.importPsd.label")}</Button>
  <!-- 長い名前は省略し、ツールチップに全体と保存先のパスを出す。未保存の印は省略されない。 -->
  <Tooltip
    text={status?.path
      ? t("fileMenu.project.tooltip", { name: status.name, path: status.path })
      : t("fileMenu.project.tooltipUnsaved", { name: status?.name ?? "…" })}
  >
    {#snippet children(attrs)}
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <span {...attrs} class="project" tabindex="0">
        <span class="name">{status?.name ?? "…"}</span>{#if status?.modified}<span
            class="dirty"
            role="img"
            aria-label={t("fileMenu.project.modified")}>●</span
          >{/if}
      </span>
    {/snippet}
  </Tooltip>
  {#if moduleInfo}
    <TipButton small class="module" tooltip={moduleTitle(moduleInfo)} onclick={() => (settingsOpen = true)} disabled={busy}
      >{moduleLabel(moduleInfo)}</TipButton
    >
  {/if}
  {#if warnings.length > 0}
    <Popover label={t("fileMenu.warnings.aria")}>
      {#snippet trigger(attrs)}
        <Button {...attrs} variant="ghost" class="warnings">
          {t("fileMenu.warnings.label", { count: warnings.length })}
        </Button>
      {/snippet}
      <ul class="warning-list">
        {#each warnings as w, i (i)}
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
<!-- 知らせの帯: ヘッダーの下に重ねる (本体の問題と、失敗の文)。 -->
{#if problem || message}
<div class="bands">
{#if problem}
  <ModuleProblemBar
    problem={problem.problem}
    canReopen={!problem.opening || problem.project !== null}
    {busy}
    onlocate={(kind) => pick(kind === "project" ? "locateBaseProject" : "locateBaseBundle")}
    onchooseslot={useSlot}
    onopenbase={openBase}
    ondropcolliders={dropColliders}
    ondismiss={() => (problem = null)}
  />
{/if}
{#if message}
  <Notice tone="danger" ondismiss={() => (message = null)}><p>{message}</p></Notice>
{/if}
</div>
{/if}
<AlertDialog
  open={pendingDiscard !== null}
  title={t("fileMenu.discard.title")}
  description={t("fileMenu.discard.description")}
  confirmLabel={t("fileMenu.discard.confirm")}
  onconfirm={() => {
    const proceed = pendingDiscard;
    pendingDiscard = null;
    proceed?.();
  }}
  oncancel={() => (pendingDiscard = null)}
/>
<ModuleSettingsDialog
  open={settingsOpen}
  module={moduleInfo}
  onapply={applyModuleSettings}
  onclose={() => (settingsOpen = false)}
/>
<BaseChangeDialog
  change={baseChange}
  base={moduleInfo?.base ?? ""}
  {autoReload}
  onautoreload={setAutoReload}
  onreload={() => reloadBase()}
  onlocate={(kind) => pick(kind === "project" ? "locateBaseProject" : "locateBaseBundle")}
  ondismiss={dismissBase}
/>
<NewModuleDialog
  open={newModuleOpen}
  startDir={lastDir ?? (status?.path ? parentDir(status.path) : null)}
  oncreate={createModule}
  onclose={() => (newModuleOpen = false)}
/>
<ExternalChangeDialog
  change={external}
  modified={status?.modified ?? false}
  saving={externalOnSave}
  {autoReload}
  onautoreload={setAutoReload}
  onreload={() => reloadProject(false)}
  onoverwrite={overwriteExternal}
  onsaveas={saveAsExternal}
  ondismiss={dismissExternal}
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
  .export {
    position: relative;
    flex-shrink: 0;
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
