<script lang="ts">
// ファイル操作 (新規・開く・保存・別名で保存・書き出し・PSD の取り込み) とプロジェクト名・未保存の印。
// パスはファイルダイアログではなく入力欄で受け取る (docs/editor.md#プロジェクト)。
import { ipc, type ProjectStatus, type PsdImportWarning } from "./ipc";

interface Props {
  /** 新規作成・開く・PSD の取り込みでモデルが変わったときに呼ぶ。 */
  onchange: () => void;
}

let { onchange }: Props = $props();

type PathAction = "open" | "saveAs" | "export" | "importPsd";

const labels: Record<PathAction, string> = {
  open: "Open project (.ugoeproj directory)",
  saveAs: "Save project as (.ugoeproj directory)",
  export: "Export bundle (.ugoe file)",
  importPsd: "Import PSD (.psd file)",
};

let status = $state<ProjectStatus | null>(null);
let action = $state<PathAction | null>(null);
let path = $state("");
/** 未保存の変更を破棄する操作の確認待ち。 */
let pendingDiscard = $state<"new" | "open" | null>(null);
let message = $state<string | null>(null);
/** 最後の PSD の取り込みの警告。 */
let warnings = $state<PsdImportWarning[]>([]);

/** 状態 (名前・未保存の印) を取り直す。編集の後に親から呼ぶ。 */
export async function refresh() {
  try {
    status = await ipc.projectStatus();
  } catch (e) {
    message = String(e);
  }
}

async function run(task: () => Promise<ProjectStatus | undefined>, reload: boolean) {
  try {
    const result = await task();
    if (result) status = result;
    message = null;
    action = null;
    if (reload) onchange();
  } catch (e) {
    message = String(e);
  }
}

function discardConfirmed(kind: "new" | "open"): boolean {
  if (!status?.modified || pendingDiscard === kind) {
    pendingDiscard = null;
    return true;
  }
  pendingDiscard = kind;
  message = "Unsaved changes will be lost. Click again to continue.";
  return false;
}

function newProject() {
  if (discardConfirmed("new")) run(ipc.newProject, true);
}

function ask(next: PathAction) {
  if (next === "open" && !discardConfirmed("open")) return;
  action = next;
  path = next === "saveAs" && status?.path ? status.path : "";
  if (next !== "open") message = null;
}

function save() {
  if (status?.path) run(ipc.saveProject, false);
  else ask("saveAs");
}

function submit(event: SubmitEvent) {
  event.preventDefault();
  const target = path.trim();
  if (action === "open") run(() => ipc.openProject(target), true);
  else if (action === "saveAs") run(() => ipc.saveProjectAs(target), false);
  else if (action === "export")
    run(async () => {
      await ipc.exportBundle(target);
      return undefined;
    }, false);
  else if (action === "importPsd")
    run(async () => {
      warnings = [];
      warnings = await ipc.importPsd(target);
      return ipc.projectStatus();
    }, true);
}

refresh();
</script>

<nav class="file-menu">
  <button onclick={newProject}>New</button>
  <button onclick={() => ask("open")}>Open…</button>
  <button onclick={save}>Save</button>
  <button onclick={() => ask("saveAs")}>Save As…</button>
  <button onclick={() => ask("export")}>Export…</button>
  <button onclick={() => ask("importPsd")}>Import PSD…</button>
  <span class="project" title={status?.path ?? "not saved yet"}>
    {status?.name ?? "…"}{#if status?.modified}<span class="dirty" aria-label="modified">●</span>{/if}
  </span>
  {#if action}
    <form onsubmit={submit}>
      <input bind:value={path} placeholder={labels[action]} aria-label={labels[action]} size="40" />
      <button type="submit">OK</button>
      <button type="button" onclick={() => (action = null)}>Cancel</button>
    </form>
  {/if}
  {#if message}<span class="error">{message}</span>{/if}
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
  .project {
    margin-left: 8px;
    font-weight: 600;
  }
  .dirty {
    margin-left: 4px;
    color: var(--accent);
  }
  form {
    display: flex;
    gap: 4px;
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
  .error {
    color: #dc2626;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
