<!--
  プレビューの着せ替え (docs/editor.md#プレビューの着せ替え)。`.ugoemod` を読み込み (ボタンか、プレビュー中のウィンドウへの
  ドロップ)、スロットごとに付けるモジュールを選ぶ。付け外しの失敗は DressUpError の code から分かる言葉で出す (dressUp.ts)。
  状態と WebPuppet の操作は DressUp (dressUp.svelte.ts)。モデルは変えない。
  モジュールのプロジェクトでは、今の編集 (`own` の行) も一覧に出し、同じスロットのモジュールと差し替えて比べられる (T0651)。
  同じ名前の書き出したファイルの行には、今の編集との違いを文字でも出す (T0654)。
-->
<script lang="ts">
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { onMount } from "svelte";
import { fileDialogs } from "../fileDialogs";
import type { Slot } from "../ipc";
import { isModuleFile, MODULE_FILTER, slotRows } from "./dressUp";
import type { DressUp, ModuleSource } from "./dressUp.svelte";

interface Props {
  dressUp: DressUp;
  /** 本体のスロット (モデルの定義順)。 */
  slots: readonly Slot[];
}

let { dressUp, slots }: Props = $props();

/** `.ugoemod` をウィンドウの上でドラッグしている。 */
let dragging = $state(false);
let dropError = $state<string | null>(null);

const rows = $derived(
  slotRows(
    slots,
    dressUp.entries.flatMap((e) =>
      e.loaded && e.slot !== null ? [{ name: e.name, slot: e.slot, attached: e.attached }] : [],
    ),
  ),
);

async function pick(): Promise<void> {
  const path = await fileDialogs().openFile({ title: "Load a module", filters: [MODULE_FILTER] });
  if (path) await dressUp.add([{ path }]);
}

/** ドロップしたもののうち `.ugoemod` だけを読み込む。 */
function dropped(sources: (ModuleSource & { name: string })[]): void {
  const modules = sources.filter((s) => isModuleFile(s.name));
  dropError = modules.length < sources.length ? "Only .ugoemod files can be loaded here." : null;
  if (modules.length > 0) void dressUp.add(modules);
}

// ブラウザ (と、ドロップを webview に渡す設定) では HTML のドラッグアンドドロップ。パスは分からない。
function hasFiles(e: DragEvent): boolean {
  return e.dataTransfer?.types.includes("Files") ?? false;
}

function onDragOver(e: DragEvent): void {
  if (!hasFiles(e)) return;
  e.preventDefault();
  if (e.dataTransfer) e.dataTransfer.dropEffect = "copy";
  dragging = true;
}

async function onDrop(e: DragEvent): Promise<void> {
  if (!hasFiles(e)) return;
  e.preventDefault();
  dragging = false;
  const files = [...(e.dataTransfer?.files ?? [])];
  const sources = await Promise.all(
    files.map(async (file) => ({
      name: file.name,
      path: null,
      bytes: isModuleFile(file.name) ? new Uint8Array(await file.arrayBuffer()) : new Uint8Array(),
    })),
  );
  dropped(sources);
}

// Tauri のウィンドウでは、OS からのドロップを Tauri が受けてパスを渡す (HTML のドロップにはファイルが来ない)。
onMount(() => {
  if (!isTauri()) return;
  let unlisten: (() => void) | null = null;
  let disposed = false;
  void getCurrentWebview()
    .onDragDropEvent(({ payload }) => {
      if (payload.type === "enter") dragging = payload.paths.some(isModuleFile);
      else if (payload.type === "leave") dragging = false;
      else if (payload.type === "drop") {
        dragging = false;
        dropped(payload.paths.map((path) => ({ name: path, path })));
      }
    })
    .then((f) => {
      if (disposed) f();
      else unlisten = f;
    });
  return () => {
    disposed = true;
    unlisten?.();
  };
});
</script>

<svelte:window ondragover={onDragOver} ondragleave={() => (dragging = false)} ondrop={onDrop} />

<section aria-label="Dress-up" class="dress-up" class:dragging>
  <div class="head">
    <h3>Dress-up</h3>
    <button onclick={() => void pick()} disabled={dressUp.busy} title="Load a module file (.ugoemod) and put it on"
      >Load .ugoemod…</button
    >
  </div>
  <p class="muted hint">
    {dragging ? "Drop to load the modules." : "Try modules on this model. Load a file or drop .ugoemod files onto the window. The model is not changed."}
  </p>
  {#if dressUp.error}<p class="error" title={dressUp.error.detail}>{dressUp.error.summary}</p>{/if}
  {#if dressUp.ownProblem}<p class="error">{dressUp.ownProblem}</p>{/if}
  {#if dropError}<p class="error">{dropError}</p>{/if}
  {#each dressUp.notices as notice, i (i)}
    <p class="notice">
      <span>{notice}</span>
      <button class="dismiss" aria-label="Dismiss" title="Dismiss" onclick={() => dressUp.dismissNotice(i)}>✕</button>
    </p>
  {/each}

  <h4>Slots</h4>
  {#if slots.length === 0}
    <p class="muted">This model has no slots. Add them in the Slots tab.</p>
  {:else}
    {#each rows as row (row.id)}
      <label class="slot">
        <span class="slot-id">{row.id}</span>
        <select
          value={row.attached ?? ""}
          disabled={row.candidates.length === 0 || dressUp.busy}
          aria-label="Module in slot {row.id}"
          title={row.candidates.length === 0 ? "No loaded module goes in this slot" : "Choose the module to put on this slot"}
          onchange={(e) => dressUp.setSlot(row.id, e.currentTarget.value || null)}
        >
          <option value="">{row.candidates.length === 0 ? "(no module)" : "(none)"}</option>
          {#each row.candidates as name (name)}
            <option value={name}>{name}</option>
          {/each}
        </select>
      </label>
    {/each}
  {/if}

  <h4>Modules</h4>
  {#if dressUp.entries.length === 0}
    <p class="muted">No modules loaded.</p>
  {:else}
    <ul class="modules">
      {#each dressUp.entries as entry (entry.own ? "#own" : (entry.path ?? entry.name))}
        <li class:own={entry.own}>
          <div class="module">
            <label
              class="check"
              title={entry.own
                ? "Your current edits (not saved to a file). Swap with other modules of the slot to compare."
                : (entry.path ?? "Dropped file (not kept in the project)")}
            >
              <input
                type="checkbox"
                checked={entry.attached}
                disabled={!entry.loaded || dressUp.busy}
                aria-label="Put on {entry.name}"
                onchange={(e) => (e.currentTarget.checked ? dressUp.attach(entry.name) : dressUp.detach(entry.name))}
              />
              <span class="name">{entry.label ?? entry.name}</span>
              {#if entry.own}<span class="badge">editing</span>{/if}
            </label>
            <span class="muted slot-of">{entry.slot ? `→ ${entry.slot}` : ""}</span>
            {#if !entry.own}
            <button
              class="icon"
              disabled={entry.path === null || dressUp.busy}
              aria-label="Reload {entry.name}"
              title="Read the file again (after exporting it again)"
              onclick={() => void dressUp.reload(entry.name)}>↻</button
            >
            <button
              class="icon"
              aria-label="Remove {entry.name}"
              title="Take it off and remove it from the list"
              onclick={() => dressUp.remove(entry.name)}>✕</button
            >
            {/if}
          </div>
          {#if entry.problem}
            <p class="error">{entry.problem.summary}</p>
            {#if entry.problem.detail !== entry.problem.summary}
              <details class="detail"><summary>Details</summary>{entry.problem.detail}</details>
            {/if}
          {/if}
          {#if entry.differences === null}
            <p class="muted">Comparing with your current edits…</p>
          {:else if entry.differences?.length === 0}
            <p class="muted">Same as your current edits.</p>
          {:else if entry.differences}
            <details class="differences">
              <summary
                title="What changed from this file to your current edits (file → edits)"
                >{entry.differences.length} difference{entry.differences.length === 1 ? "" : "s"} from your current edits</summary
              >
              <ul>
                {#each entry.differences as line, i (i)}<li>{line}</li>{/each}
              </ul>
            </details>
          {/if}
          {#each entry.warnings as warning, i (i)}<p class="muted">{warning}</p>{/each}
          {#if entry.path === null && !entry.own}<p class="muted">Dropped file: not kept in the project.</p>{/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
/* 見出しと区切りはプレビューのパネルのほかの節に揃える。 */
section {
  border-top: 1px solid var(--border);
  padding-top: 4px;
}

h3 {
  margin: 4px 0;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
}

.dress-up.dragging {
  outline: 2px dashed var(--accent);
  outline-offset: 2px;
}

.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 4px;
}

h4 {
  margin: 6px 0 2px;
  font-size: 11px;
  font-weight: 600;
}

.hint {
  margin: 2px 0 4px;
}

p {
  margin: 2px 0;
  overflow-wrap: anywhere;
}

.notice {
  display: flex;
  align-items: flex-start;
  gap: 4px;
  padding: 4px 6px;
  border: 1px solid #d97706;
  border-radius: 4px;
}

.notice span {
  flex: 1;
  min-width: 0;
}

.dismiss,
.icon {
  padding: 0 6px;
}

/* エディタの label は縦に並べる (app.css) ので、スロットの行は横に並べ直す。 */
.slot {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1.4fr);
  align-items: center;
  gap: 6px;
  margin-bottom: 2px;
}

.slot-id {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 600;
}

.modules {
  margin: 0;
  padding: 0;
  list-style: none;
}

.modules > li {
  margin-bottom: 4px;
}

.module {
  display: flex;
  align-items: center;
  gap: 4px;
}

.check {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: 1;
  min-width: 0;
}

.badge {
  font-size: 10px;
  padding: 0 4px;
  border: 1px solid var(--accent);
  border-radius: 3px;
  color: var(--accent);
  white-space: nowrap;
}

.name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.slot-of {
  white-space: nowrap;
}

.detail {
  color: var(--muted);
  overflow-wrap: anywhere;
}

.detail summary,
.differences summary {
  cursor: pointer;
}

.differences ul {
  margin: 2px 0;
  padding-left: 16px;
  overflow-wrap: anywhere;
}
</style>
