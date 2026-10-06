<!--
  パラメータのパネルの「Groups」: パラメータのグループ (Model::parameter_groups。パラメータの一覧・プレビュー・ビューアのスライダーの区切り) を
  作る・名前を変える・消す・並べ替える (docs/editor.md#パラメータのグループ)。どれも Command を 1 つ送る (Undo の 1 単位)。
  並べ替えは行のドラッグ・▲▼・名前のボタンの Alt + ↑↓ のどれでも、reorder_parameter_group を 1 つ送る。
  消してもパラメータは残り、割り当てだけが外れる。パラメータをグループに入れるのは各パラメータの編集欄の「Group」。
  モジュールのプロジェクトでは本体のものなので変えられない。
-->
<script lang="ts">
import { Button, Icon, IconButton, Section, TextField, Tooltip } from "@ugoe/ui";
import { tick } from "svelte";
import BaseLock from "../../BaseLock.svelte";
import type { EditorDocument, ParameterDef } from "../../document.svelte";
import { t } from "../../i18n";
import { elementLabel } from "../../labels";
import {
  addGroupCommand,
  dropGroupCommand,
  dropIndexAt,
  memberCount,
  moveGroupCommand,
} from "./groups";

interface Props {
  doc: EditorDocument;
  parameters: ParameterDef[];
}

let { doc, parameters }: Props = $props();

const groups = $derived(doc.parsedModel?.parameter_groups ?? []);
const locked = $derived(doc.module !== null);
/** 編集を送っている間は並べ替えない (並びは送った後に読み直したものを使う)。 */
const busy = $derived(doc.pending > 0);

let open = $state(false);
let creating = $state(false);
let fresh = $state({ id: "", name: "" });
/** 名前を編集中のグループ (null なら閉じている)。 */
let editing = $state<string | null>(null);
let draftName = $state("");

const ids = $props.id();

function startCreating() {
  open = true;
  creating = true;
}

async function create(e: SubmitEvent) {
  e.preventDefault();
  const command = addGroupCommand(fresh.id, fresh.name);
  if (!command) return;
  if ((await doc.apply(command)) !== null) {
    creating = false;
    fresh = { id: "", name: "" };
  }
}

function openEditor(id: string, name: string) {
  editing = editing === id ? null : id;
  draftName = name;
}

async function rename(e: SubmitEvent, id: string, name: string) {
  e.preventDefault();
  const next = draftName.trim() || id;
  if (
    next === name ||
    (await doc.apply({ op: "rename_parameter_group", id, name: next })) !== null
  ) {
    editing = null;
  }
}

async function remove(id: string) {
  if ((await doc.apply({ op: "remove_parameter_group", id })) !== null) editing = null;
}

async function move(id: string, delta: -1 | 1) {
  const command = moveGroupCommand(groups, id, delta);
  if (command) await doc.apply(command);
}

let list = $state<HTMLUListElement | null>(null);

/** 名前のボタンで Alt + ↑↓: 1 つ上・下へ移し、移した行の名前にフォーカスを戻す (ドラッグと ▲▼ のキーボードでの近道)。 */
async function nameKeydown(event: KeyboardEvent, id: string) {
  if (!event.altKey || (event.key !== "ArrowUp" && event.key !== "ArrowDown")) return;
  event.preventDefault();
  if (busy) return;
  await move(id, event.key === "ArrowUp" ? -1 : 1);
  await tick();
  list?.querySelector<HTMLElement>(`li[data-group="${CSS.escape(id)}"] .name`)?.focus();
}

// ---- グループの並べ替えのドラッグ ----
// ポーズの要素 (PosesPanel.svelte) と同じく、HTML5 の DnD ではなくポインタで実装する (Windows の WebView2 で
// ファイルのドロップを有効にしていると DnD が届かないため)。落としたときに 1 回だけ送り、ポインタの取り消しと Esc では何も送らない。

/** 押した行 (動かして初めてドラッグになる)。 */
let press: { id: string; x: number; y: number; pointer: number } | null = null;
/** ドラッグ中のグループと、落とす先 (動かす前の並びの番号の行の前。行の数なら末尾)。 */
let dragging = $state<string | null>(null);
let dropAt = $state<number | null>(null);
/** ドラッグを離した直後のクリック (名前を押したことになる) を捨てる。 */
let suppressClick = false;

function rowPointerDown(event: PointerEvent, id: string): void {
  if (event.button !== 0 || locked || busy || groups.length < 2) return;
  // ▲▼ と名前の編集欄はドラッグを始めずにそのまま使える。
  if ((event.target as Element).closest(".moves, .editor")) return;
  press = { id, x: event.clientX, y: event.clientY, pointer: event.pointerId };
}

function dragMove(event: PointerEvent): void {
  if (!press || !list || event.pointerId !== press.pointer) return;
  // 動かす前に一覧の外で離していた (pointerup が届いていない)。
  if ((event.buttons & 1) === 0) {
    cancelDrag();
    return;
  }
  if (dragging === null) {
    if (Math.hypot(event.clientX - press.x, event.clientY - press.y) < 5) return;
    dragging = press.id;
    list.setPointerCapture?.(event.pointerId);
  }
  const rows = [...list.querySelectorAll<HTMLElement>(":scope > li")].map((li) =>
    li.getBoundingClientRect(),
  );
  dropAt = dropIndexAt(rows, event.clientY);
}

function dragEnd(event: PointerEvent): void {
  if (!press || event.pointerId !== press.pointer) return;
  const id = dragging;
  const index = dropAt;
  cancelDrag();
  if (id === null) return;
  suppressClick = true;
  // クリックが起きない場合 (取り込んだ要素の外で離したとき) に備えて戻す。
  setTimeout(() => {
    suppressClick = false;
  });
  const command = index === null ? null : dropGroupCommand(groups, id, index);
  if (command) void doc.apply(command);
}

function cancelDrag(): void {
  press = null;
  dragging = null;
  dropAt = null;
}

function dragKeydown(event: KeyboardEvent): void {
  if (dragging !== null && event.key === "Escape") {
    event.preventDefault();
    cancelDrag();
  }
}

/** 落とす先の印を出す行: 行の前 (上の線) か、末尾なら最後の行の後ろ (下の線)。自分の前後 (動かない所) には出さない。 */
function dropMark(index: number): "before" | "after" | null {
  if (dragging === null || dropAt === null) return null;
  const from = groups.findIndex((g) => g.id === dragging);
  if (dropAt === from || dropAt === from + 1) return null;
  if (dropAt === index) return "before";
  return dropAt === groups.length && index === groups.length - 1 ? "after" : null;
}
</script>

<svelte:window onkeydown={dragKeydown} />

<Section title={t("parameters.groups.title", { count: groups.length })} bind:open>
  {#snippet actions()}
    <IconButton
      small
      variant="secondary"
      onclick={startCreating}
      disabled={locked}
      label={t("parameters.groups.add.aria")}
      aria-describedby={locked ? `${ids}-note` : undefined}>+</IconButton
    >
  {/snippet}
  <BaseLock {locked} hint={t("parameters.groups.baseHint")}>
    <p class="ed-hint" id="{ids}-note">{t("parameters.groups.description")}</p>
    {#if creating}
      <form class="editor" onsubmit={create}>
        <!-- i18n-ignore: グループの ID の例 (訳さない) -->
        <TextField label={t("parameters.field.id")} bind:value={fresh.id} required mono placeholder="Face" />
        <TextField
          label={t("parameters.field.name")}
          bind:value={fresh.name}
          placeholder={t("parameters.field.namePlaceholder")}
        />
        <span class="ed-buttons">
          <Button small type="submit">{t("parameters.create.label")}</Button>
          <Button small onclick={() => (creating = false)}>{t("parameters.cancel.label")}</Button>
        </span>
      </form>
    {:else if groups.length === 0}
      <p class="ed-hint">{t("parameters.groups.empty")}</p>
      <span class="ed-buttons">
        <Button small onclick={startCreating}>{t("parameters.groups.add.label")}</Button>
      </span>
    {/if}
    {#if groups.length > 1 && !locked}
      <p class="ed-help">{t("parameters.groups.dragHint")}</p>
    {/if}
    <ul
      bind:this={list}
      onpointermove={dragMove}
      onpointerup={dragEnd}
      onpointercancel={cancelDrag}
      onlostpointercapture={() => {
        if (dragging !== null) cancelDrag();
      }}
      onclickcapture={(e) => {
        if (!suppressClick) return;
        suppressClick = false;
        e.stopPropagation();
        e.preventDefault();
      }}
    >
      {#each groups as g, i (g.id)}
        {@const label = elementLabel(g.name, g.id)}
        {@const mark = dropMark(i)}
        <li
          data-group={g.id}
          class:dragging={dragging === g.id}
          class:drop-before={mark === "before"}
          class:drop-after={mark === "after"}
        >
          <!-- ドラッグはポインタだけの近道。キーボードでは ▲▼ か名前のボタンの Alt + ↑↓ で並べ替える。 -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <span
            class="row"
            class:grab={!locked && !busy && groups.length > 1}
            onpointerdown={(e) => rowPointerDown(e, g.id)}
          >
            <Tooltip text={t("parameters.groups.edit.tooltip", { id: g.id })}>
              {#snippet trigger(attrs)}
                <button
                  {...attrs}
                  class="name"
                  aria-expanded={editing === g.id}
                  aria-keyshortcuts={groups.length > 1 ? "Alt+ArrowUp Alt+ArrowDown" : undefined}
                  onclick={(e) => {
                    (attrs.onclick as ((e: MouseEvent) => void) | undefined)?.(e);
                    openEditor(g.id, g.name);
                  }}
                  onkeydown={(e) => nameKeydown(e, g.id)}>{label}</button
                >
              {/snippet}
            </Tooltip>
            <span class="count"
              >{t("parameters.groups.members", { count: memberCount(parameters, g.id) })}</span
            >
            <span class="moves">
              <IconButton
                small
                disabled={i === 0}
                label={t("parameters.groups.moveUp.aria", { name: label })}
                onclick={() => move(g.id, -1)}><Icon name="arrowUp" /></IconButton
              >
              <IconButton
                small
                disabled={i === groups.length - 1}
                label={t("parameters.groups.moveDown.aria", { name: label })}
                onclick={() => move(g.id, 1)}><Icon name="arrowDown" /></IconButton
              >
            </span>
          </span>
          {#if editing === g.id}
            <form class="editor" onsubmit={(e) => rename(e, g.id, g.name)}>
              <TextField label={t("parameters.field.name")} bind:value={draftName} />
              <p class="ed-hint" id="{ids}-delete-{g.id}">{t("parameters.groups.delete.description")}</p>
              <span class="ed-buttons">
                <Button small type="submit">{t("parameters.apply.label")}</Button>
                <Button
                  small
                  variant="danger"
                  aria-describedby="{ids}-delete-{g.id}"
                  onclick={() => remove(g.id)}>{t("parameters.groups.delete.label")}</Button
                >
              </span>
            </form>
          {/if}
        </li>
      {/each}
    </ul>
  </BaseLock>
</Section>

<style>
ul {
  list-style: none;
  margin: 0;
  padding: 0;
}

/* 落とす先は行の上 (末尾なら最後の行の下) の強調色の線で示す (ポーズの要素の並べ替えと同じ) */
li {
  border-top: 2px solid transparent;
  border-bottom: 2px solid transparent;
}

li.dragging {
  opacity: 0.5;
}

li.drop-before {
  border-top-color: var(--ugoe-color-accent);
}

li.drop-after {
  border-bottom-color: var(--ugoe-color-accent);
}

.row.grab {
  cursor: grab;
}

li.dragging .row,
li.dragging .name {
  cursor: grabbing;
}

.moves {
  display: flex;
  gap: var(--ugoe-space-1);
}

.row {
  display: flex;
  align-items: center;
  gap: var(--ugoe-space-1);
}

.name {
  flex: 1;
  min-width: 0;
  min-height: var(--ugoe-target-min);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  background: none;
  border: none;
  padding: 0;
  color: inherit;
  cursor: pointer;
  text-align: left;
}

.count {
  color: var(--ugoe-color-text-muted);
  font-size: var(--ugoe-font-size-sm);
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}

.editor {
  display: flex;
  flex-direction: column;
  gap: var(--ugoe-space-1);
  padding: var(--ugoe-space-1) 0 var(--ugoe-space-2);
}
</style>
