<!--
  プレビューの着せ替え (docs/editor.md#プレビューの着せ替え)。アバタープロファイルがアプリに公開していないスロットには印を付ける
  (ビューアは隠すが、作者のツールなので試せるようにしておく。T0655)。`.ugoemod` を読み込み (ボタンか、プレビュー中のウィンドウへの
  ドロップ)、スロットごとに付けるモジュールを選ぶ。付け外しの失敗は DressUpError の code から分かる言葉で出す (dressUp.ts)。
  状態と WebPuppet の操作は DressUp (dressUp.svelte.ts)。モデルは変えない。
  モジュールのプロジェクトでは、今の編集 (`own` の行) も一覧に出し、同じスロットのモジュールと差し替えて比べられる (T0651)。
  同じ名前の書き出したファイルの行には、今の編集との違いを文字でも出す (T0654)。足した・変えた・除いた要素は何が違うかを
  添える (T0656)。要素の ID を押すと、プレビューのままビューポートで強調し (もう一度押すとやめる。T0657)、
  「Select」でプレビューを出てツリーか Physics タブで選ぶ。
-->
<script lang="ts">
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { Checkbox, IconButton, Notice, Select, Tooltip } from "@ugoe/ui";
import { onMount } from "svelte";
import { fileDialogs } from "../fileDialogs";
import { MODULE_FILTER } from "../filePaths";
import { t } from "../i18n";
import type { DifferenceItem, Slot } from "../ipc";
import TipButton from "../ui/TipButton.svelte";
import { selectTitle } from "./differenceHighlight";
import { isModuleFile, slotRows } from "./dressUp";
import type { DressUp, ModuleSource } from "./dressUp.svelte";

interface Props {
  dressUp: DressUp;
  /** 本体のスロット (モデルの定義順)。 */
  slots: readonly Slot[];
  /** アバタープロファイルがアプリに公開するスロット (`avatar.slots`)。プロファイルが無ければ `null`。 */
  published?: readonly string[] | null;
  /** 違いの行の要素を選ぶ (プレビューを出る)。無ければ選べない。 */
  onselect?: (item: DifferenceItem) => void;
}

let { dressUp, slots, published = null, onselect }: Props = $props();

/** `.ugoemod` をウィンドウの上でドラッグしている。 */
let dragging = $state(false);
let dropError = $state<string | null>(null);

const rows = $derived(
  slotRows(
    slots,
    dressUp.entries.flatMap((e) =>
      e.loaded && e.slot !== null ? [{ name: e.name, slot: e.slot, attached: e.attached }] : [],
    ),
    published,
  ),
);

/** スロットに何も付けない選択肢の値 (モジュールの名前は `#` で始まらない)。 */
const NONE = "#none";

/** スロットに付けるモジュールの選択肢。 */
function slotOptions(candidates: readonly string[]): { value: string; label: string }[] {
  return [
    {
      value: NONE,
      label:
        candidates.length === 0
          ? t("preview.dressUp.slots.noModule")
          : t("preview.dressUp.slots.none"),
    },
    ...candidates.map((name) => ({ value: name, label: name })),
  ];
}

async function pick(): Promise<void> {
  const path = await fileDialogs().openFile({
    title: t("preview.dressUp.load.title"),
    filters: [MODULE_FILTER],
  });
  if (path) await dressUp.add([{ path }]);
}

/** ドロップしたもののうち `.ugoemod` だけを読み込む。 */
function dropped(sources: (ModuleSource & { name: string })[]): void {
  const modules = sources.filter((s) => isModuleFile(s.name));
  dropError = modules.length < sources.length ? t("preview.dressUp.dropError") : null;
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

<section aria-label={t("preview.dressUp.title")} class="dress-up" class:dragging>
  <div class="head">
    <h3>{t("preview.dressUp.title")}</h3>
    <TipButton small onclick={() => void pick()} disabled={dressUp.busy} tooltip={t("preview.dressUp.load.tooltip")}
      >{t("preview.dressUp.load.label")}</TipButton
    >
  </div>
  <p class="ed-hint hint">
    {dragging ? t("preview.dressUp.dropping") : t("preview.dressUp.description")}
  </p>
  {#if dressUp.error}
    <p class="error ugoe-selectable">{dressUp.error.summary}</p>
    {#if dressUp.error.detail !== dressUp.error.summary}
      <details class="detail ugoe-selectable"><summary>{t("preview.dressUp.details")}</summary>{dressUp.error.detail}</details>
    {/if}
  {/if}
  {#if dressUp.ownProblem}<p class="error ugoe-selectable">{dressUp.ownProblem}</p>{/if}
  {#if dropError}<p class="error ugoe-selectable">{dropError}</p>{/if}
  {#each dressUp.notices as notice, i (i)}
    <Notice tone="warning" ondismiss={() => dressUp.dismissNotice(i)}>{notice}</Notice>
  {/each}

  <h4>{t("preview.dressUp.slots.title")}</h4>
  {#if slots.length === 0}
    <p class="ed-hint">{t("preview.dressUp.slots.empty")}</p>
  {:else}
    {#each rows as row (row.id)}
      <div class="slot">
        <span class="slot-name">
          <span class="slot-id">{row.id}</span>
          {#if !row.published}
            <Tooltip text={t("preview.dressUp.slots.authorsOnly.tooltip")}>
              {#snippet trigger(attrs)}
                <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
                <span {...attrs} class="badge authors" tabindex="0">{t("preview.dressUp.slots.authorsOnly.label")}</span>
              {/snippet}
            </Tooltip>
          {/if}
        </span>
        <Select
          aria-label={t("preview.dressUp.slots.aria", { slot: row.id })}
          value={row.attached ?? NONE}
          options={slotOptions(row.candidates)}
          disabled={row.candidates.length === 0 || dressUp.busy}
          onchange={(v) => dressUp.setSlot(row.id, v === NONE ? null : v)}
        />
      </div>
    {/each}
  {/if}

  <h4>{t("preview.dressUp.modules.title")}</h4>
  {#if dressUp.entries.length === 0}
    <p class="ed-hint">{t("preview.dressUp.modules.empty")}</p>
  {:else}
    <ul class="modules">
      {#each dressUp.entries as entry (entry.own ? "#own" : (entry.path ?? entry.name))}
        <li class:own={entry.own}>
          <div class="module">
            <span class="check">
              <Checkbox
                label={entry.label ?? entry.name}
                checked={entry.attached}
                disabled={!entry.loaded || dressUp.busy}
                onchange={(on) => (on ? dressUp.attach(entry.name) : dressUp.detach(entry.name))}
              />
              {#if entry.own}
                <Tooltip text={t("preview.dressUp.modules.editing.tooltip")}>
                  {#snippet trigger(attrs)}
                    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
                    <span {...attrs} class="badge" tabindex="0">{t("preview.dressUp.modules.editing.label")}</span>
                  {/snippet}
                </Tooltip>
              {/if}
            </span>
            <span class="muted slot-of">{entry.slot ? `→ ${entry.slot}` : ""}</span>
            {#if !entry.own}
              <IconButton
                small
                variant="ghost"
                disabled={entry.path === null || dressUp.busy}
                label={t("preview.dressUp.modules.reload.aria", { name: entry.name })}
                tooltip={t("preview.dressUp.modules.reload.tooltip")}
                onclick={() => void dressUp.reload(entry.name)}>↻</IconButton
              >
              <IconButton
                small
                variant="ghost"
                label={t("preview.dressUp.modules.remove.aria", { name: entry.name })}
                tooltip={t("preview.dressUp.modules.remove.tooltip")}
                onclick={() => dressUp.remove(entry.name)}>✕</IconButton
              >
            {/if}
          </div>
          {#if entry.path}<p class="ed-hint path">{entry.path}</p>{/if}
          {#if entry.problem}
            <p class="error ugoe-selectable">{entry.problem.summary}</p>
            {#if entry.problem.detail !== entry.problem.summary}
              <details class="detail ugoe-selectable"><summary>{t("preview.dressUp.details")}</summary>{entry.problem.detail}</details>
            {/if}
          {/if}
          {#if entry.differences === null}
            <p class="ed-hint">{t("preview.dressUp.differences.comparing")}</p>
          {:else if entry.differences?.length === 0}
            <p class="ed-hint">{t("preview.dressUp.differences.same")}</p>
          {:else if entry.differences}
            <details class="differences">
              <summary>{t("preview.dressUp.differences.summary", { count: entry.differences.length })}</summary>
              <p class="ed-hint">{t("preview.dressUp.differences.description")}</p>
              <ul>
                {#each entry.differences as line, i (i)}
                  <li>
                    {line.text}
                    {#if line.items.length > 0}
                      <ul class="items">
                        {#each line.items as item (item.id)}
                          {@const shown =
                            dressUp.highlight?.module === entry.name &&
                            dressUp.highlight.item.kind === item.kind &&
                            dressUp.highlight.item.id === item.id}
                          <li>
                            <Tooltip
                              text={shown
                                ? t("preview.dressUp.differences.unhighlight", { id: item.id })
                                : t("preview.dressUp.differences.highlight", { id: item.id })}
                            >
                              {#snippet trigger(attrs)}
                                <button
                                  {...attrs}
                                  class="link"
                                  class:highlighted={shown}
                                  aria-pressed={shown}
                                  onclick={(e) => {
                                    (attrs.onclick as ((e: MouseEvent) => void) | undefined)?.(e);
                                    dressUp.toggleHighlight(entry.name, item);
                                  }}>{item.id}</button
                                >
                              {/snippet}
                            </Tooltip>: {item.text}
                            {#if item.element !== null && onselect}
                              <TipButton small tooltip={selectTitle(item)} onclick={() => onselect?.(item)}
                                >{t("preview.dressUp.differences.select")}</TipButton
                              >
                            {/if}
                          </li>
                        {/each}
                      </ul>
                    {/if}
                  </li>
                {/each}
              </ul>
            </details>
          {/if}
          {#each entry.warnings as warning, i (i)}<p class="ed-hint">{warning}</p>{/each}
          {#if entry.path === null && !entry.own}<p class="ed-hint">{t("preview.dressUp.modules.dropped")}</p>{/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
/* 見出しと区切りはプレビューのパネルのほかの節に揃える。 */
section {
  border-top: 1px solid var(--ugoe-color-border);
  padding-top: var(--ugoe-space-2);
}

h3 {
  margin: var(--ugoe-space-2) 0;
}

.dress-up.dragging {
  outline: 2px dashed var(--ugoe-color-accent);
  outline-offset: 2px;
}

.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ugoe-space-2);
}

h4 {
  margin: var(--ugoe-space-2) 0 var(--ugoe-space-1);
  font-size: var(--ugoe-font-size-sm);
  font-weight: var(--ugoe-font-weight-strong);
}

.hint {
  margin: var(--ugoe-space-1) 0 var(--ugoe-space-2);
}

p {
  margin: var(--ugoe-space-1) 0;
  overflow-wrap: anywhere;
}

.error {
  color: var(--ugoe-color-danger);
}

.muted {
  color: var(--ugoe-color-text-muted);
}

.slot {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1.4fr);
  align-items: center;
  gap: var(--ugoe-space-2);
  margin-bottom: var(--ugoe-space-1);
}

/* 狭いときは印をスロットの名前の下に回し、名前を切らない。 */
.slot-name {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0 var(--ugoe-space-2);
  min-width: 0;
}

.slot-id {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: var(--ugoe-font-weight-strong);
}

.modules {
  margin: 0;
  padding: 0;
  list-style: none;
}

.modules > li {
  margin-bottom: var(--ugoe-space-2);
}

.module {
  display: flex;
  align-items: center;
  gap: var(--ugoe-space-2);
}

.check {
  display: flex;
  align-items: center;
  gap: var(--ugoe-space-2);
  flex: 1;
  min-width: 0;
}

.badge {
  font-size: var(--ugoe-font-size-xs);
  padding: 0 var(--ugoe-space-2);
  border: 1px solid var(--ugoe-color-accent);
  border-radius: var(--ugoe-radius-sm);
  color: var(--ugoe-color-accent);
  white-space: nowrap;
}

.badge.authors {
  border-color: var(--ugoe-color-text-muted);
  color: var(--ugoe-color-text-muted);
}

.slot-of {
  white-space: nowrap;
}

.path {
  margin: 0;
}

.detail {
  color: var(--ugoe-color-text-muted);
  overflow-wrap: anywhere;
}

.detail summary,
.differences summary {
  cursor: pointer;
}

.differences ul {
  margin: var(--ugoe-space-1) 0;
  padding-left: 16px;
  overflow-wrap: anywhere;
}

.differences .items {
  margin: 0;
  color: var(--ugoe-color-text-muted);
}

/* 要素の ID は文字のように見せ、押すとビューポートで強調する。 */
.link {
  padding: 0;
  border: none;
  background: none;
  color: var(--ugoe-color-accent);
  font: inherit;
  text-decoration: underline;
  cursor: pointer;
}

.link:focus-visible {
  outline: var(--ugoe-focus-ring);
  outline-offset: 1px;
}

.link.highlighted {
  font-weight: var(--ugoe-font-weight-strong);
  text-decoration-style: double;
}
</style>
