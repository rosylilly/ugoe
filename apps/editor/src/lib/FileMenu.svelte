<script lang="ts">
// ファイル操作 (新規・PSD から新規・開く・保存・別名で保存・書き出し・PSD の取り込み) と
// プロジェクト名・未保存の印。パスは OS のファイルダイアログで選ぶ (docs/editor.md#ファイルダイアログ)。
// 外でのプロジェクトの書き換えも、ウィンドウに戻ったとき・一定の間隔・保存の前に確かめる (docs/editor.md#外での書き換え)。
import ExportPanel from "./ExportPanel.svelte";
import ExternalChangeDialog from "./ExternalChangeDialog.svelte";
import {
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
  type ExportReport,
  type ExternalChange,
  ipc,
  type ModuleStatus,
  type ProjectStatus,
  type PsdImportWarning,
} from "./ipc";
import { isBaseNotFoundError, moduleLabel, moduleTitle } from "./moduleProject";
import NewModuleDialog from "./NewModuleDialog.svelte";

interface Props {
  /**
   * 新規作成・開く・PSD の取り込みでモデルが変わったときに呼ぶ。`replaced` は別のモデルに替わったこと
   * (新規・PSD から新規・開く。パラメータのスライダーを既定値に戻す)。PSD の取り込みは `false`。
   * `reloaded` は同じプロジェクトを開き直したこと (外での書き換えの後。表示の位置を保つ)。
   */
  onchange: (replaced: boolean, reloaded?: boolean) => void;
}

let { onchange }: Props = $props();

type PathAction =
  | "newFromPsd"
  | "open"
  | "saveAs"
  | "export"
  | "exportModule"
  | "importPsd"
  | "locateBaseProject"
  | "locateBaseBundle";
/** 未保存の変更を破棄する操作。 */
type DiscardAction = "new" | "newFromPsd" | "newModule" | "open";

let status = $state<ProjectStatus | null>(null);
/** モジュールのプロジェクトなら、本体とスロット (docs/editor.md#モジュールのプロジェクト)。 */
let moduleInfo = $state<ModuleStatus | null>(null);
/** 新しいモジュールの画面を開いているか。 */
let newModuleOpen = $state(false);
/** 本体が見つからずに開けなかったモジュールのプロジェクト (本体を選び直して開く)。 */
let baseMissing = $state<string | null>(null);
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
  baseMissing = null;
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
      return run(() => ipc.openProject(path), "replaced").then((ok) => {
        // モジュールのプロジェクトの本体が見つからなければ、本体を選び直して開けるようにする
        baseMissing = !ok && isBaseNotFoundError(message) ? path : null;
        return ok;
      });
    case "locateBaseProject":
    case "locateBaseBundle": {
      const project = baseMissing;
      if (!project) return Promise.resolve(false);
      return run(() => ipc.openModuleWithBase(project, path), "replaced").then((ok) => {
        if (ok) {
          baseMissing = null;
          notice = "Opened with the new base. Save to remember it.";
        }
        return ok;
      });
    }
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

async function pick(kind: PathAction) {
  if (busy) return;
  if ((kind === "open" || kind === "newFromPsd") && !discardConfirmed(kind)) return;
  busy = true;
  notice = null;
  try {
    const path = await choose(kind);
    message = null;
    if (path && (await perform(kind, path))) lastDir = parentDir(path) ?? lastDir;
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

/** ウィンドウに戻ったとき・一定の間隔で、外での書き換えを確かめる。 */
async function checkExternal() {
  if (busy || external || !status?.path) return;
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
    <span class="module" title={moduleTitle(moduleInfo)}>{moduleLabel(moduleInfo)}</span>
  {/if}
  {#if baseMissing}
    <button onclick={() => pick("locateBaseProject")} disabled={busy}>Locate base project…</button>
    <button onclick={() => pick("locateBaseBundle")} disabled={busy}>Locate base .ugoe…</button>
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
