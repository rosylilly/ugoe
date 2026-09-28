<script lang="ts">
// ファイル操作 (新規・PSD から新規・開く・保存・別名で保存・書き出し・PSD の取り込み) と
// プロジェクト名・未保存の印。パスは OS のファイルダイアログで選ぶ (docs/editor.md#ファイルダイアログ)。
import { fileDialogs } from "./fileDialogs";
import {
  BUNDLE_FILTER,
  joinPath,
  PROJECT_FILTER,
  PSD_FILTER,
  parentDir,
  safeFileName,
  withExtension,
} from "./filePaths";
import { ipc, type ProjectStatus, type PsdImportWarning } from "./ipc";

interface Props {
  /** 新規作成・開く・PSD の取り込みでモデルが変わったときに呼ぶ。 */
  onchange: () => void;
}

let { onchange }: Props = $props();

type PathAction = "newFromPsd" | "open" | "saveAs" | "export" | "importPsd";
/** 未保存の変更を破棄する操作。 */
type DiscardAction = "new" | "newFromPsd" | "open";

let status = $state<ProjectStatus | null>(null);
/** ダイアログを開いているか、その後の処理の途中か。 */
let busy = $state(false);
/** 未保存の変更を破棄する操作の確認待ち。 */
let pendingDiscard = $state<DiscardAction | null>(null);
let message = $state<string | null>(null);
/** 保存・書き出しの結果 (どこに書いたか)。 */
let notice = $state<string | null>(null);
/** 最後の PSD の取り込みの警告。 */
let warnings = $state<PsdImportWarning[]>([]);
/** 最後にダイアログで選んだ場所 (次のダイアログの最初の場所)。 */
let lastDir: string | null = null;

/** 状態 (名前・未保存の印) を取り直す。編集の後に親から呼ぶ。 */
export async function refresh() {
  try {
    status = await ipc.projectStatus();
  } catch (e) {
    message = String(e);
  }
}

/** 成功したら `true`。失敗はメッセージに出す。 */
async function run(
  task: () => Promise<ProjectStatus | undefined>,
  reload: boolean,
): Promise<boolean> {
  try {
    const result = await task();
    if (result) status = result;
    message = null;
    if (reload) onchange();
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
  if (discardConfirmed("new")) run(ipc.newProject, true);
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
      }, true);
    case "importPsd":
      return run(async () => {
        warnings = [];
        warnings = await ipc.importPsd(path);
        return ipc.projectStatus();
      }, true);
    case "open":
      return run(() => ipc.openProject(path), true);
    case "saveAs":
      return run(async () => {
        const result = await ipc.saveProjectAs(path);
        notice = `Saved to ${result.path ?? path}`;
        return result;
      }, false);
    case "export":
      return run(async () => {
        await ipc.exportBundle(path);
        notice = `Exported to ${path}`;
        return undefined;
      }, false);
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

function save() {
  notice = null;
  if (status?.path) run(ipc.saveProject, false);
  else pick("saveAs");
}

refresh();
</script>

<nav class="file-menu">
  <button onclick={newProject} disabled={busy}>New</button>
  <button onclick={() => pick("newFromPsd")} disabled={busy}>New from PSD…</button>
  <button onclick={() => pick("open")} disabled={busy}>Open…</button>
  <button onclick={save} disabled={busy}>Save</button>
  <button onclick={() => pick("saveAs")} disabled={busy}>Save As…</button>
  <button onclick={() => pick("export")} disabled={busy}>Export…</button>
  <button onclick={() => pick("importPsd")} disabled={busy}>Import PSD…</button>
  <span class="project" title={status?.path ?? "not saved yet"}>
    {status?.name ?? "…"}{#if status?.modified}<span class="dirty" aria-label="modified">●</span>{/if}
  </span>
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

<style>
  .file-menu {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
  }
  .file-menu > button {
    flex-shrink: 0;
    white-space: nowrap;
  }
  .project {
    margin-left: 8px;
    font-weight: 600;
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
