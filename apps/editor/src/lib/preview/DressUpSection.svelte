<!--
  プレビューの着せ替え (docs/editor.md#プレビューの着せ替え)。アバタープロファイルがアプリに公開していないスロットには印を付ける
  (ビューアは隠すが、作者のツールなので試せるようにしておく。T0655)。`.ugoemod` を読み込み (ボタンか、プレビュー中のウィンドウへの
  ドロップ)、スロットごとに付けるモジュールを選ぶ。付け外しの失敗は DressUpError の code から分かる言葉で出す (dressUp.ts)。
  状態と WebPuppet の操作は DressUp (dressUp.svelte.ts)。モデルは変えない。
  モジュールのプロジェクトでは、今の編集 (`own` の行) も一覧に出し、同じスロットのモジュールと差し替えて比べられる (T0651)。
  同じ名前の書き出したファイルの行には、今の編集との違いを文字でも出す (T0654)。足した・変えた・除いた要素は何が違うかを
  添える (T0656)。要素の ID を押すと、プレビューのままビューポートで強調し (もう一度押すとやめる。T0657)、
  「Select」でプレビューを出てツリーか Physics タブで選ぶ。
  知らせの形はプレビューのパネルとそろえる (T0676): 読み込みの失敗 (パネルの頭) と付けられないこと (行) は `Notice` の danger
  (元の文言は「Details」)、行の読み込みの警告は `Notice` の warning。自動で外れたことは、その行の danger に「… was taken off: 理由」
  として 1 回だけ出す (パネルの頭に同じ理由を重ねない。T1644)。行のファイルは名前だけを出す (全体は `title`)。
  違いの一覧は、行の文の下に要素の表 (ID・何が違うか・選ぶ) を置き、要素が多ければ絞り込みの欄を出し、1 行の要素は
  `DIFFERENCE_ROWS` 件まで出して残りは「さらに N 件」で開く (パネルの中に縦の送りを重ねない)。閉じている間は中を描かない。
-->
<script lang="ts">
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { Button, Checkbox, Icon, IconButton, Notice, Select, TextField, Tooltip } from "@ugoe/ui";
import { onMount } from "svelte";
import { fileDialogs } from "../fileDialogs";
import { MODULE_FILTER } from "../filePaths";
import { t } from "../i18n";
import type { DifferenceItem, ModuleDifference, Slot } from "../ipc";
import TipButton from "../ui/TipButton.svelte";
import { selectTitle } from "./differenceHighlight";
import {
  DIFFERENCE_FILTER_FROM,
  DIFFERENCE_ROWS,
  differenceItemCount,
  filterDifferences,
  isModuleFile,
  slotRows,
} from "./dressUp";
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

/** 違いの一覧を開いているか・絞り込みの語・全部を出している行 (モジュールの名前ごと)。 */
let differencesOpen = $state<Record<string, boolean>>({});
let differenceQuery = $state<Record<string, string>>({});
let differencesExpanded = $state<Record<string, boolean>>({});
/** 失敗の詳細を開いているか (鍵は行のモジュールの名前。パネルの頭は `#error`)。 */
let detailsOpen = $state<Record<string, boolean>>({});

/** パスのファイル名 (行には名前だけを出す)。 */
function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

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
    {@const error = dressUp.error}
    <Notice
      tone="danger"
      dismissLabel={t("preview.dressUp.dismiss")}
      ondismiss={() => (dressUp.error = null)}
    >
      {error.summary}
      {#if error.detail !== error.summary}{@render detail("#error", error.detail)}{/if}
    </Notice>
  {/if}
  {#if dressUp.ownProblem}<Notice tone="danger">{dressUp.ownProblem}</Notice>{/if}
  {#if dropError}
    <Notice tone="danger" dismissLabel={t("preview.dressUp.dismiss")} ondismiss={() => (dropError = null)}
      >{dropError}</Notice
    >
  {/if}

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
                <span {...attrs} class="ed-tag" tabindex="0">{t("preview.dressUp.slots.authorsOnly.label")}</span>
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
                    <span {...attrs} class="ed-tag accent" tabindex="0">{t("preview.dressUp.modules.editing.label")}</span>
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
                onclick={() => void dressUp.reload(entry.name)}><Icon name="reload" /></IconButton
              >
              <IconButton
                small
                variant="ghost"
                label={t("preview.dressUp.modules.remove.aria", { name: entry.name })}
                tooltip={t("preview.dressUp.modules.remove.tooltip")}
                onclick={() => dressUp.remove(entry.name)}><Icon name="close" /></IconButton
              >
            {/if}
          </div>
          {#if entry.path}
            <!-- 全体のパスは title (フォーカスを受けない文の補い。docs/ui.md の「残した title」) -->
            <p class="ed-hint path" title={entry.path}>{fileName(entry.path)}</p>
          {/if}
          {#if entry.problem}
            {@const problem = entry.problem}
            <Notice tone="danger">
              {problem.summary}
              {#if problem.detail !== problem.summary}{@render detail(entry.name, problem.detail)}{/if}
            </Notice>
          {/if}
          {#each entry.warnings as warning, i (i)}<Notice tone="warning">{warning}</Notice>{/each}
          {#if entry.path === null && !entry.own}<p class="ed-hint">{t("preview.dressUp.modules.dropped")}</p>{/if}
          {#if entry.differences === null}
            <p class="ed-hint" role="status">{t("preview.dressUp.differences.comparing")}</p>
          {:else if entry.differences?.length === 0}
            <p class="ed-hint">{t("preview.dressUp.differences.same")}</p>
          {:else if entry.differences}
            {@render differences(entry.name, entry.differences)}
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

<!-- 失敗の元の文言 (報告に貼れるよう選択できる)。閉じている間は中を描かない -->
{#snippet detail(key: string, text: string)}
  <details
    class="detail"
    open={detailsOpen[key] ?? false}
    ontoggle={(e) => (detailsOpen[key] = e.currentTarget.open)}
  >
    <summary>{t("preview.dressUp.details")}</summary>
    {#if detailsOpen[key]}<p class="ugoe-selectable">{text}</p>{/if}
  </details>
{/snippet}

<!-- 今の編集との違い: 行の文と、要素の表 (ID・何が違うか・選ぶ)。要素が多ければ絞り込み、1 行は DIFFERENCE_ROWS 件まで -->
{#snippet differences(name: string, lines: ModuleDifference[])}
  {@const total = differenceItemCount(lines)}
  {@const query = differenceQuery[name] ?? ""}
  {@const shownLines = filterDifferences(lines, query)}
  <details
    class="differences"
    open={differencesOpen[name] ?? false}
    ontoggle={(e) => (differencesOpen[name] = e.currentTarget.open)}
  >
    <summary>{t("preview.dressUp.differences.summary", { count: lines.length })}</summary>
    {#if differencesOpen[name]}
      <p class="ed-hint">{t("preview.dressUp.differences.description")}</p>
      {#if total > DIFFERENCE_FILTER_FROM}
        <TextField
          type="search"
          label={t("preview.dressUp.differences.filter.label")}
          placeholder={t("preview.dressUp.differences.filter.placeholder")}
          value={query}
          oninput={(v) => (differenceQuery[name] = v)}
        />
      {/if}
      {#if shownLines.length === 0}
        <p class="ed-hint" role="status">{t("preview.dressUp.differences.filter.none")}</p>
      {/if}
      {#each shownLines as line, i (i)}
        {@const key = `${name}\u0000${line.text}`}
        {@const limit = differencesExpanded[key] || query !== "" ? line.items.length : DIFFERENCE_ROWS}
        <p class="line">{line.text}</p>
        {#if line.items.length > 0}
          <table class="items">
            <thead class="ugoe-visually-hidden">
              <tr>
                <th scope="col">{t("preview.dressUp.differences.columns.element")}</th>
                <th scope="col">{t("preview.dressUp.differences.columns.change")}</th>
                <th scope="col">{t("preview.dressUp.differences.columns.select")}</th>
              </tr>
            </thead>
            <tbody>
              {#each line.items.slice(0, limit) as item (`${item.kind}:${item.id}`)}
                {@const shown =
                  dressUp.highlight?.module === name &&
                  dressUp.highlight.item.kind === item.kind &&
                  dressUp.highlight.item.id === item.id}
                <tr>
                  <th scope="row">
                    <Tooltip
                      text={shown
                        ? t("preview.dressUp.differences.unhighlight", { id: item.id })
                        : t("preview.dressUp.differences.highlight", { id: item.id })}
                    >
                      {#snippet trigger(attrs)}
                        <button
                          {...attrs}
                          class="ed-link"
                          class:highlighted={shown}
                          aria-pressed={shown}
                          onclick={(e) => {
                            (attrs.onclick as ((e: MouseEvent) => void) | undefined)?.(e);
                            dressUp.toggleHighlight(name, item);
                          }}>{item.id}</button
                        >
                      {/snippet}
                    </Tooltip>
                  </th>
                  <td>{item.text}</td>
                  <td class="action">
                    {#if item.element !== null && onselect}
                      <TipButton small tooltip={selectTitle(item)} onclick={() => onselect?.(item)}
                        >{t("preview.dressUp.differences.select")}</TipButton
                      >
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
          {#if line.items.length > limit}
            <Button small onclick={() => (differencesExpanded[key] = true)}
              >{t("preview.dressUp.differences.more", { count: line.items.length - limit })}</Button
            >
          {/if}
        {/if}
      {/each}
    {/if}
  </details>
{/snippet}

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

section :global(.ugoe-notice) {
  margin: var(--ugoe-space-2) 0;
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


.slot-of {
  white-space: nowrap;
}

/* ファイルの名前は 1 行 (長ければ省略記号。全体は title) */
.path {
  margin: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.detail {
  margin-top: var(--ugoe-space-1);
}

.detail summary,
.differences summary {
  min-height: var(--ugoe-target-min);
  line-height: var(--ugoe-target-min);
  cursor: pointer;
}

.detail p {
  color: var(--ugoe-color-text-muted);
  font-size: var(--ugoe-font-size-sm);
}

.differences {
  margin-top: var(--ugoe-space-1);
}

.differences > :global(.ugoe-text-field) {
  margin: var(--ugoe-space-2) 0;
}

.line {
  margin: var(--ugoe-space-3) 0 var(--ugoe-space-1);
  font-weight: var(--ugoe-font-weight-strong);
}

/* 要素の表: ID・何が違うか・選ぶ。何が違うかは折り返す。見せない見出し (thead) の位置の基準にもなる (docs/ui.md#レイアウト の「送り」) */
.items {
  position: relative;
  width: 100%;
  border-collapse: collapse;
  font-size: var(--ugoe-font-size-sm);
}

.items th,
.items td {
  padding: 0 var(--ugoe-space-2) 0 0;
  border-top: 1px solid var(--ugoe-color-border);
  text-align: start;
  vertical-align: top;
  font-weight: var(--ugoe-font-weight-normal);
}

.items td {
  padding-top: var(--ugoe-space-1);
  color: var(--ugoe-color-text-muted);
  overflow-wrap: anywhere;
}

.items .action {
  width: 1%;
  padding: 0;
  white-space: nowrap;
}

/* 要素の ID は文字のように見せ (app.css の .ed-link)、押すとビューポートで強調する */
.highlighted {
  font-weight: var(--ugoe-font-weight-strong);
  text-decoration-style: double;
}
</style>
