<script lang="ts">
// ファイル操作 (新規・PSD から新規・開く・保存・別名で保存・書き出し・PSD の取り込み) と
// プロジェクト名・未保存の印。パスは OS のファイルダイアログで選ぶ (docs/editor.md#ファイルダイアログ)。
// 外でのプロジェクトの書き換えも、ウィンドウに戻ったとき・一定の間隔・保存の前に確かめる (docs/editor.md#外での書き換え)。
// モジュールのプロジェクトでは、本体の変化も同じ機会に確かめ (BaseChangeDialog。「Reload automatically」なら
// 尋ねずに読み直す)、本体の問題で開けない・読み直せないときは直し方を出す (ModuleProblemBar)。開いている
// モジュールの本体・スロットの付け替えは、開き直さずに reload_base で行う (保存していない編集を保つ)。
// `Module:` の印を押すとモジュールの設定 (ModuleSettingsDialog)。
import BaseChangeDialog from "./BaseChangeDialog.svelte";
import ExportPanel from "./ExportPanel.svelte";
import ExternalChangeDialog from "./ExternalChangeDialog.svelte";
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
import {
  type Command,
  type ExportReport,
  type ExternalChange,
  ipc,
  type ModuleProblem,
  type ModuleStatus,
  type ProjectStatus,
  type PsdImportWarning,
} from "./ipc";
import ModuleProblemBar from "./ModuleProblemBar.svelte";
import ModuleSettingsDialog from "./ModuleSettingsDialog.svelte";
import { moduleLabel, moduleTitle } from "./moduleProject";
import NewModuleDialog from "./NewModuleDialog.svelte";

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
/** 未保存の変更を破棄する操作。 */
type DiscardAction = "new" | "newFromPsd" | "newModule" | "open" | "openSample";

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
/** 未保存の変更を破棄する操作の確認待ち。 */
let pendingDiscard = $state<DiscardAction | null>(null);
let message = $state<string | null>(null);
/** 保存・書き出しの結果 (どこに書いたか)。 */
let notice = $state<string | null>(null);
/** 最後の PSD の取り込みの警告。 */
let warnings = $state<PsdImportWarning[]>([]);
/** 書き出しの設定の欄を開いているか。 */
let exportOpen = $state(false);
/** 最後の書き出しの結果 (テクスチャの枚数と状態の切り替えの変化)。 */
let exportReport = $state<ExportReport | null>(null);
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
    message = String(e);
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
      exportReport = null;
    }
    if (reload !== "none") onchange(reload === "replaced", reload === "reloaded");
    return true;
  } catch (e) {
    message = String(e);
    return false;
  }
}

function discardConfirmed(kind: DiscardAction): boolean {
  if (!status?.modified || pendingDiscard === kind) {
    pendingDiscard = null;
    return true;
  }
  pendingDiscard = kind;
  message = "Unsaved changes will be lost. Click again to continue.";
  return false;
}

function newProject() {
  notice = null;
  if (discardConfirmed("new")) run(ipc.newProject, "replaced");
}

function newModule() {
  notice = null;
  if (discardConfirmed("newModule")) newModuleOpen = true;
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
        title: "New Project from PSD",
        defaultPath: dir,
        filters: [PSD_FILTER],
      });
    case "importPsd":
      return dialogs.openFile({ title: "Import PSD", defaultPath: dir, filters: [PSD_FILTER] });
    case "open":
      // .ugoeproj はディレクトリなので、フォルダーを選ぶ
      return dialogs.openDirectory({ title: "Open Project (.ugoeproj folder)", defaultPath: dir });
    case "openSample":
      return dialogs.openDirectory({
        title: "Choose a folder to put the dress-up sample projects in",
        defaultPath: dir,
      });
    case "saveAs": {
      const path = await dialogs.saveFile({
        title: "Save Project As",
        defaultPath: suggest(`${name}.ugoeproj`),
        filters: [PROJECT_FILTER],
      });
      return path && withExtension(path, "ugoeproj");
    }
    case "exportModule": {
      const path = await dialogs.saveFile({
        title: "Export Module",
        defaultPath: suggest(`${safeFileName(moduleInfo?.name ?? name)}.ugoemod`),
        filters: [MODULE_FILTER],
      });
      return path && withExtension(path, "ugoemod");
    }
    case "locateBaseProject":
      return dialogs.openDirectory({
        title: "Locate the base project (.ugoeproj folder)",
        defaultPath: dir,
      });
    case "locateBaseBundle":
      return dialogs.openFile({
        title: "Locate the base bundle",
        defaultPath: dir,
        filters: [BUNDLE_FILTER],
      });
    case "export": {
      const path = await dialogs.saveFile({
        title: "Export Bundle",
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
          notice = `Opened the dress-up sample. Its module projects (long-hair, twin-tails, cat-ears, sailor) are next to it${folder ? ` in ${folder}` : ""}; open them with Open…`;
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
        warnings = report.warnings.map((message) => ({ path: [], message }));
        notice = `Exported to ${path}`;
        return undefined;
      }, "none");
    case "saveAs":
      return run(async () => {
        const result = await ipc.saveProjectAs(path);
        notice = `Saved to ${result.path ?? path}`;
        return result;
      }, "none");
    case "export":
      return run(async () => {
        exportReport = await ipc.exportBundle(path);
        notice = `Exported to ${path}`;
        return undefined;
      }, "none");
  }
}

/**
 * 本体の問題で開けなかったモジュールのプロジェクトを、本体 `base` かスロット `slot` を付け替えて開き直す
 * (`base` が null なら、前に選び直した本体があればそれ)。それでも本体の問題で開けなければ、新しい問題を出す。
 */
async function reopenModule(base: string | null, slot: string | null): Promise<boolean> {
  const project = problem?.project;
  if (!project) return false;
  const useBase = base ?? problem?.base ?? null;
  const ok = await run(() => ipc.openModuleWithBase(project, useBase, slot), "replaced");
  if (ok) {
    problem = null;
    notice =
      slot === null
        ? "Opened with the new base. Save to remember it."
        : `Opened on slot ${slot}. Save to remember it.`;
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
    notice =
      slot === null
        ? "Using the new base. Save to remember it."
        : `Moved to slot ${slot}. Save to remember it.`;
    return true;
  } catch (e) {
    message = String(e);
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

/** 本体の問題の枠の「Open base project」: 本体のプロジェクトを開いて直す (モジュールは後で開き直す)。 */
async function openBase() {
  const base = problem?.problem.base;
  if (!base || busy || !discardConfirmed("open")) return;
  busy = true;
  try {
    if (await run(() => ipc.openProject(base), "replaced")) {
      problem = null;
      notice = "Opened the base project. Fix it, save, then open the module again.";
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

async function pick(kind: PathAction) {
  if (busy) return;
  if (
    (kind === "open" || kind === "openSample" || kind === "newFromPsd") &&
    !discardConfirmed(kind)
  )
    return;
  busy = true;
  notice = null;
  try {
    const path = await choose(kind);
    message = null;
    if (path && (await perform(kind, path))) {
      // 見本なら、次の「Open…」をモジュールのプロジェクトの並ぶフォルダーから始める
      const opened = kind === "openSample" && status?.path ? status.path : path;
      lastDir = parentDir(opened) ?? lastDir;
    }
  } catch (e) {
    message = String(e);
  } finally {
    busy = false;
  }
}

async function save() {
  notice = null;
  if (!status?.path) return pick("saveAs");
  if (busy) return;
  // 保存の前に確かめ、外で変わっていれば選ばせる (バックエンドも保存の直前に確かめて断る)
  const change = await ipc.checkExternalChange().catch(() => null);
  if (change) {
    askExternal(change, true);
    return;
  }
  const saved = await run(ipc.saveProject, "none");
  if (!saved && isExternalChangeError(message)) {
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
    notice = auto
      ? "Reloaded the base model: it was changed outside the editor"
      : "Reloaded the base model";
  } catch (e) {
    message = String(e);
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
    notice = auto
      ? "Reloaded: the project was changed outside the editor"
      : "Reloaded the project from disk";
  }
}

async function overwriteExternal() {
  external = null;
  dismissedToken = null;
  await run(async () => {
    const result = await ipc.saveProject(true);
    notice = `Saved to ${result.path ?? ""}`;
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

<nav class="file-menu">
  <button onclick={newProject} disabled={busy}>New</button>
  <button onclick={() => pick("newFromPsd")} disabled={busy}>New from PSD…</button>
  <button onclick={newModule} disabled={busy} title="Make a dress-up module for a slot of a base model"
    >New Module…</button
  >
  <button onclick={() => pick("open")} disabled={busy}>Open…</button>
  <button
    onclick={() => pick("openSample")}
    disabled={busy}
    title="Write the dress-up sample (a base and its module projects) into a new folder and open the base"
    >Open Sample…</button
  >
  <button onclick={save} disabled={busy}>Save</button>
  <button onclick={() => pick("saveAs")} disabled={busy}>Save As…</button>
  {#if moduleInfo}
    <button onclick={() => pick("exportModule")} disabled={busy} title="Check the module against the base and write .ugoemod"
      >Export Module…</button
    >
  {:else}
  <span class="export">
    <button
      onclick={() => (exportOpen = !exportOpen)}
      disabled={busy}
      aria-expanded={exportOpen}>Export…</button
    >
    {#if exportOpen}
      <ExportPanel
        {busy}
        report={exportReport}
        onexport={() => pick("export")}
        onstatus={(s) => (status = s)}
      />
    {/if}
  </span>
  {/if}
  <button onclick={() => pick("importPsd")} disabled={busy}>Import PSD…</button>
  <span class="project" title={status?.path ?? "not saved yet"}>
    {status?.name ?? "…"}{#if status?.modified}<span class="dirty" aria-label="modified">●</span>{/if}
  </span>
  {#if moduleInfo}
    <button class="module" title={moduleTitle(moduleInfo)} onclick={() => (settingsOpen = true)} disabled={busy}
      >{moduleLabel(moduleInfo)}</button
    >
  {/if}
  {#if message}<span class="error">{message}</span>{/if}
  {#if notice}<span class="notice" title={notice}>{notice}</span>{/if}
  {#if warnings.length > 0}
    <details class="warnings">
      <summary>{warnings.length} import warning{warnings.length === 1 ? "" : "s"}</summary>
      <ul>
        {#each warnings as w, i (i)}
          <li>{w.path.length > 0 ? `${w.path.join(" / ")}: ` : ""}{w.message}</li>
        {/each}
      </ul>
    </details>
  {/if}
</nav>
{#if problem}
  <ModuleProblemBar
    problem={problem.problem}
    canReopen={!problem.opening || problem.project !== null}
    {busy}
    onlocate={(kind) => pick(kind === "project" ? "locateBaseProject" : "locateBaseBundle")}
    onchooseslot={useSlot}
    onopenbase={openBase}
    ondismiss={() => (problem = null)}
  />
{/if}
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
  .file-menu {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
  }
  .export {
    position: relative;
    flex-shrink: 0;
  }
  .file-menu > button,
  .export > button {
    flex-shrink: 0;
    white-space: nowrap;
  }
  .project {
    margin-left: 8px;
    font-weight: 600;
  }
  .module {
    flex-shrink: 0;
    padding: 0 6px;
    border: 1px solid var(--accent);
    border-radius: 3px;
    color: var(--accent);
    white-space: nowrap;
  }
  .dirty {
    margin-left: 4px;
    color: var(--accent);
  }
  .warnings {
    position: relative;
    color: #b45309;
    white-space: nowrap;
  }
  .warnings[open] ul {
    position: absolute;
    z-index: 10;
    margin: 4px 0 0;
    padding: 8px 8px 8px 24px;
    background: var(--panel);
    border: 1px solid currentColor;
    white-space: normal;
    min-width: 320px;
  }
  .error,
  .notice {
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .error {
    color: #dc2626;
  }
  .notice {
    opacity: 0.7;
  }
</style>
