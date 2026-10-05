<!--
  ポーズのパネル (docs/editor.md#ポーズの編集)。右の列の「Poses」タブ。
  操作はすべて、ポーズを丸ごと置き換える set_pose を送るだけで、組み立ては poses.ts にある。ポーズは送る直前の
  ものから組み立てる (send。前の編集を読み直す前の写しから組み立てると、その編集を戻してしまう)。

  - グループの一覧。グループの要素のうち表示されるのは 1 つで、先頭の要素が最初に表示される (ドラッグか ▲▼ で並べ替える。ドラッグはポインタのイベントで作る)。
    要素の `links` (要素と一緒に切り替わるパーツ。腕の差分に付く袖など) は要素ごとに選ぶ。
  - 「+ Group」「+ Entry」で、ポーズでまだ使っていないパーツを足す。「+ Group from selection」は、ツリーで選んだパーツで
    グループを作る。要素の名前を押すとそのパーツを選ぶ。
  - モジュールのプロジェクトでは、本体のグループ (先頭の `base_pose_groups` 個) は読み取り専用。本体の要素のうちスロットが
    公開するもの (`pose_parts`) にだけ、モジュールのパーツをリンクに加えられる。モジュールのグループは後ろに足す
    (docs/spec/modules.md#ポーズ)。
  - 「Show the pose in the viewport」で、編集中のビューポートにポーズを効かせ (グループごとに 1 つの要素だけが見える)、
    要素の「Show」で表示する要素を切り替えて確かめる (ViewportController.setPosePreview。モデルは変えない)。
    タブを離れると効かせるのをやめる。
-->
<script lang="ts">
import { formatList } from "@ugoe/i18n";
import { Button, Icon, IconButton, NumberField, Select, Toggle, Tooltip } from "@ugoe/ui";
import { onDestroy } from "svelte";
import type { EditorDocument } from "../../document.svelte";
import { t } from "../../i18n";
import type { Pose } from "../../ipc";
import { elementLabel, labelOf } from "../../labels";
import type { Selection } from "../../selection.svelte";
import TipButton from "../../ui/TipButton.svelte";
import type { ViewportController } from "../../viewport/controller.svelte";
import PickList from "../slots/PickList.svelte";
import {
  addEntry,
  addGroup,
  canLinkTo,
  dropIndex,
  editableLinks,
  fadeSeconds,
  freeParts,
  isBaseGroup,
  isBasePart,
  moveEntry,
  moveEntryTo,
  removeEntry,
  removeGroup,
  setLinks,
  setPoseCommand,
  showEntry,
  shownEntry,
} from "./poses";

interface Props {
  doc: EditorDocument;
  selection: Selection;
  viewport: Pick<ViewportController, "posePreview" | "setPosePreview">;
}

let { doc, selection, viewport }: Props = $props();

/** ビューポートにポーズを効かせているときの、表示する要素のパーツ (効かせていなければ null)。 */
const shown = $derived(viewport.posePreview);

// タブを離れたら、編集中のビューポートはすべての要素が見える姿に戻す。
onDestroy(() => viewport.setPosePreview(null));

// ---- 要素の並べ替えのドラッグ ----
// Tauri のファイルのドロップ (着せ替えのプレビュー) を有効にしていると、Windows (WebView2) では HTML5 の DnD が
// ウィンドウに届かないので、ツリー (TreePanel.svelte) と同じくポインタで実装する。並べ替えは落としたときに 1 回だけ送る。
// ポインタの取り消し (pointercancel・取り込みを失う) と Esc では何も送らずに元へ戻す。

/** 押した要素 (動かして初めてドラッグになる)。 */
let press: {
  group: number;
  entry: number;
  x: number;
  y: number;
  pointer: number;
  list: HTMLElement;
} | null = null;
/** ドラッグ中の要素 (グループと要素の番号) と、落とす先 (その要素の前。要素の数なら末尾)。 */
let dragging = $state<{ group: number; entry: number } | null>(null);
let dropAt = $state<{ group: number; index: number } | null>(null);
/** ドラッグを離した直後のクリック (要素の名前を押したことになる) を捨てる。 */
let suppressClick = false;

function entryPointerDown(event: PointerEvent, group: number, entry: number, count: number): void {
  if (event.button !== 0 || busy || count < 2 || isBaseGroup(module, group)) return;
  // ボタン (Show・▲▼・✕) はドラッグを始めずにそのまま押せる。
  if ((event.target as Element).closest(".buttons")) return;
  const list = (event.currentTarget as HTMLElement).closest<HTMLElement>("ol");
  if (!list) return;
  press = { group, entry, x: event.clientX, y: event.clientY, pointer: event.pointerId, list };
}

/** ポインタの下の要素と、その要素の中の位置から落とす先を決める (別のグループの上なら null)。 */
function dropIndexAt(list: HTMLElement, x: number, y: number): number | null {
  const el = document.elementFromPoint(x, y)?.closest<HTMLElement>("li[data-entry]");
  if (!el || el.parentElement !== list) return null;
  const box = el.getBoundingClientRect();
  return dropIndex(Number(el.dataset.entry), box.top, box.height, y);
}

function dragMove(event: PointerEvent): void {
  if (!press || event.pointerId !== press.pointer) return;
  // 動かす前にリストの外で離していた (pointerup が届いていない)。
  if ((event.buttons & 1) === 0) {
    cancelDrag();
    return;
  }
  if (!dragging) {
    if (Math.hypot(event.clientX - press.x, event.clientY - press.y) < 5) return;
    dragging = { group: press.group, entry: press.entry };
    press.list.setPointerCapture(event.pointerId);
  }
  const index = dropIndexAt(press.list, event.clientX, event.clientY);
  dropAt = index === null ? null : { group: press.group, index };
}

function dragEnd(event: PointerEvent): void {
  if (!press || event.pointerId !== press.pointer) return;
  const from = dragging;
  const to = dropAt;
  cancelDrag();
  if (!from) return;
  suppressClick = true;
  // クリックが起きない場合 (取り込んだ要素の外で離したとき) に備えて戻す。
  setTimeout(() => {
    suppressClick = false;
  });
  if (!to || from.group !== to.group) return;
  void send((current) => {
    const next = moveEntryTo(current, from.group, from.entry, to.index);
    return next.groups[from.group] !== current.groups[from.group] ? next : null;
  });
}

function cancelDrag(): void {
  press = null;
  dragging = null;
  dropAt = null;
}

function dragKeydown(event: KeyboardEvent): void {
  if (dragging && event.key === "Escape") {
    event.preventDefault();
    cancelDrag();
  }
}

const model = $derived(doc.parsedModel);
const module = $derived(doc.module);
const pose = $derived(model?.pose ?? null);
const parts = $derived(model?.parts ?? []);
const free = $derived(pose ? freeParts(parts, pose, module) : []);
/** ツリーで選んだパーツのうち、グループにできるもの。 */
const selectedFree = $derived(
  free.filter((p) => selection.has({ kind: "part", id: p.id })).map((p) => p.id),
);

/** 編集を送っている間は、番号に頼るボタンを無効にする (欄の編集は列に並べて送る。docs/editor.md#パネルからの編集)。 */
const busy = $derived(doc.pending > 0);
/** 「+ Group」「+ Entry」で選んでいるパーツ (グループの番号ごと。-1 は新しいグループ)。 */
let picks = $state<Record<number, string>>({});

/**
 * 前の編集を読み直した後のポーズを `change` で作り直して送る (null なら送らない)。
 * 失敗の理由は doc.error (パネルの上) に出る。
 */
async function send(change: (current: Pose) => Pose | null): Promise<boolean> {
  const report = await doc.send(() => {
    const current = model?.pose;
    const next = current ? change(current) : null;
    return next ? setPoseCommand(next) : null;
  });
  return report !== null;
}

function pickOf(group: number): string {
  const current = picks[group];
  return current && free.some((p) => p.id === current) ? current : (free[0]?.id ?? "");
}

/** 待ち時間の欄を今の値に戻すために作り直す (受け付けなかったとき)。 */
let fadeKey = $state(0);

function setFade(seconds: number): void {
  const value = fadeSeconds(String(seconds));
  if (!pose || value === null) {
    fadeKey++;
    return;
  }
  void send((current) => ({ ...current, fade_duration: value })).then((ok) => {
    if (!ok) fadeKey++;
  });
}

const ids = $props.id();

/** 「+ Entry」「+ Group」で選ぶパーツの選択肢。 */
const freeOptions = $derived(free.map((p) => ({ value: p.id, label: elementLabel(p.name, p.id) })));

function select(part: string): void {
  selection.set([{ kind: "part", id: part }]);
}

/** リンクに選べるパーツ: 使っていないものと、今のリンク (外せるように)。 */
function linkOptions(current: readonly string[]): { id: string; label: string }[] {
  const ids = [...free.map((p) => p.id), ...current.filter((id) => !free.some((p) => p.id === id))];
  return ids.map((id) => ({ id, label: labelOf(parts, id) }));
}
</script>

<svelte:window onkeydown={dragKeydown} />

<h2>{t("poses.title")}</h2>
<p class="help intro">{t("poses.intro")}</p>
{#if module}
  <p class="base-note" role="note">{t("poses.moduleNote")}</p>
{/if}

{#if model && pose}
  <div class="field">
    {#key fadeKey}
      <NumberField
        label={t("poses.fade.label")}
        unit={t("poses.fade.unit")}
        min={0}
        step={0.1}
        decimals={3}
        value={pose.fade_duration}
        disabled={module !== null}
        aria-describedby="{ids}-fade"
        onchange={setFade}
      />
    {/key}
  </div>
  <p class="help" id="{ids}-fade">{module ? t("poses.fade.baseHint") : t("poses.fade.description")}</p>

  <div class="check">
    <Toggle
      label={t("poses.preview.label")}
      checked={shown !== null}
      disabled={pose.groups.length === 0}
      aria-describedby="{ids}-preview"
      onchange={(on) => viewport.setPosePreview(on ? [] : null)}
    />
  </div>
  <p class="help" id="{ids}-preview">{t("poses.preview.description")}</p>

  {#if pose.groups.length === 0}
    <p class="ed-hint">{t("poses.empty")}</p>
  {/if}
  {#each pose.groups as group, gi (gi)}
    {@const base = isBaseGroup(module, gi)}
    <section class="group" class:base aria-labelledby="{ids}-group-{gi}">
      <header>
        <h3 id="{ids}-group-{gi}">{t("poses.group.title", { index: gi + 1 })}</h3>
        {#if base}
          <Tooltip text={t("poses.group.base.tooltip")}>
            {#snippet trigger(attrs)}<span {...attrs} class="tag">{t("poses.group.base.label")}</span>{/snippet}
          </Tooltip>
        {/if}
        <span class="remove">
          <TipButton
            small
            disabled={busy || base}
            tooltip={t("poses.group.remove.tooltip")}
            onclick={() => send((p) => removeGroup(p, gi))}>{t("poses.group.remove.label")}</TipButton
          >
        </span>
      </header>
      {#if base}
        <p class="help">{t("poses.group.baseHint")}</p>
      {:else if group.entries.length > 1}
        <p class="help">{t("poses.entry.dragHint")}</p>
      {/if}
      <ol
        class="entries"
        onpointermove={dragMove}
        onpointerup={dragEnd}
        onpointercancel={cancelDrag}
        onlostpointercapture={() => {
          if (dragging) cancelDrag();
        }}
        onclickcapture={(e) => {
          if (!suppressClick) return;
          suppressClick = false;
          e.stopPropagation();
          e.preventDefault();
        }}
      >
        {#each group.entries as entry, ei (entry.part)}
          {@const linkable = !base || canLinkTo(module, entry.part)}
          {@const isShown = shown !== null && shownEntry(group, shown) === ei}
          <li
            data-entry={ei}
            class:dragging={dragging?.group === gi && dragging.entry === ei}
            class:drop-before={dropAt?.group === gi && dropAt.index === ei}
            class:drop-after={dropAt?.group === gi && dropAt.index === ei + 1 && ei === group.entries.length - 1}
          >
            <!-- ドラッグはポインタだけの近道。キーボードでは ▲▼ で並べ替える。 -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class="entry"
              class:grab={!base && !busy && group.entries.length > 1}
              onpointerdown={(e) => entryPointerDown(e, gi, ei, group.entries.length)}
            >
              <Tooltip text={t("poses.entry.select.tooltip")}>
                {#snippet trigger(attrs)}
                  <button
                    {...attrs}
                    class="name"
                    class:selected={selection.has({ kind: "part", id: entry.part })}
                    onclick={(e) => {
                      (attrs.onclick as ((e: MouseEvent) => void) | undefined)?.(e);
                      select(entry.part);
                    }}>{labelOf(parts, entry.part)}</button
                  >
                {/snippet}
              </Tooltip>
              {#if ei === 0}
                <Tooltip text={t("poses.entry.first.tooltip")}>
                  {#snippet trigger(attrs)}<span {...attrs} class="tag">{t("poses.entry.first.label")}</span>{/snippet}
                </Tooltip>
              {/if}
              <span class="buttons">
                {#if shown !== null}
                  <TipButton
                    small
                    pressed={isShown}
                    tooltip={isShown ? t("poses.entry.shown.tooltip") : t("poses.entry.show.tooltip")}
                    onclick={() => viewport.setPosePreview(showEntry(shown, group, entry.part))}
                    >{isShown ? t("poses.entry.shown.label") : t("poses.entry.show.label")}</TipButton
                  >
                {/if}
                <IconButton
                  small
                  disabled={busy || base || ei === 0}
                  label={t("poses.entry.moveUp.aria")}
                  tooltip={t("poses.entry.moveUp.tooltip")}
                  onclick={() => send((p) => moveEntry(p, gi, ei, -1))}><Icon name="arrowUp" /></IconButton
                >
                <IconButton
                  small
                  disabled={busy || base || ei === group.entries.length - 1}
                  label={t("poses.entry.moveDown.aria")}
                  onclick={() => send((p) => moveEntry(p, gi, ei, 1))}><Icon name="arrowDown" /></IconButton
                >
                <IconButton
                  small
                  disabled={busy || base}
                  label={t("poses.entry.remove.aria")}
                  tooltip={t("poses.entry.remove.tooltip")}
                  onclick={() => send((p) => removeEntry(p, gi, ei))}><Icon name="close" /></IconButton
                >
              </span>
            </div>
            {#if base && entry.links.some((l) => isBasePart(module, l))}
              <p class="help">
                {t("poses.entry.baseLinks", {
                  parts: formatList(entry.links.filter((l) => isBasePart(module, l)).map((l) => labelOf(parts, l))),
                })}
              </p>
            {/if}
            {#if linkable}
              <PickList
                title={t("poses.links.title")}
                help={base ? t("poses.links.baseHelp") : t("poses.links.help")}
                options={linkOptions(editableLinks(entry.links, module))}
                selected={editableLinks(entry.links, module)}
                empty={t("poses.links.empty")}
                onchange={(links) => send((p) => setLinks(p, gi, ei, links, module))}
              />
            {:else}
              <p class="help">{t("poses.links.notExposed")}</p>
            {/if}
          </li>
        {/each}
      </ol>
      {#if !base}
        <div class="row">
          <span class="pick">
            <Select
              aria-label={t("poses.entry.add.aria", { index: gi + 1 })}
              value={pickOf(gi)}
              options={freeOptions}
              disabled={free.length === 0}
              onchange={(v) => (picks[gi] = v)}
            />
          </span>
          <TipButton
            disabled={busy || free.length === 0}
            tooltip={t("poses.entry.add.tooltip")}
            onclick={() => {
              const part = pickOf(gi);
              void send((p) => addEntry(p, gi, part));
            }}>{t("poses.entry.add.label")}</TipButton
          >
        </div>
      {/if}
    </section>
  {/each}

  <section class="add" aria-labelledby="{ids}-new">
    <h3 id="{ids}-new">{t("poses.newGroup.title")}</h3>
    {#if free.length === 0}
      <p class="ed-hint">{module ? t("poses.newGroup.noModuleParts") : t("poses.newGroup.noParts")}</p>
    {:else}
      <div class="row">
        <span class="pick">
          <Select
            aria-label={t("poses.newGroup.first.aria")}
            value={pickOf(-1)}
            options={freeOptions}
            onchange={(v) => (picks[-1] = v)}
          />
        </span>
        <Button disabled={busy} onclick={() => {
          const part = pickOf(-1);
          void send((p) => addGroup(p, part));
        }}>{t("poses.newGroup.add.label")}</Button>
      </div>
      <div class="row">
        <TipButton
          disabled={busy || selectedFree.length < 2}
          tooltip={t("poses.newGroup.fromSelection.tooltip", { count: selectedFree.length })}
          onclick={() => {
            const [first, ...rest] = selectedFree;
            if (first === undefined) return;
            void send((current) =>
              rest.reduce((p, id) => addEntry(p, p.groups.length - 1, id), addGroup(current, first)),
            );
          }}>{t("poses.newGroup.fromSelection.label")}</TipButton
        >
      </div>
      {#if selectedFree.length < 2}
        <p class="help">{t("poses.newGroup.fromSelection.disabled")}</p>
      {/if}
    {/if}
  </section>
{/if}

<style>
h3 {
  margin: 0;
}

.help {
  margin: var(--ugoe-space-1) 0 var(--ugoe-space-2);
  color: var(--ugoe-color-text-muted);
  font-size: var(--ugoe-font-size-sm);
  overflow-wrap: anywhere;
}

.intro {
  margin-bottom: var(--ugoe-space-3);
}

.base-note {
  margin: var(--ugoe-space-2) 0 var(--ugoe-space-3);
  padding: var(--ugoe-space-2) var(--ugoe-space-3);
  border-left: 3px solid var(--ugoe-color-border);
  color: var(--ugoe-color-text-muted);
  font-size: var(--ugoe-font-size-sm);
}

.field {
  margin: var(--ugoe-space-2) 0 0;
}

.group,
.add {
  margin: var(--ugoe-space-3) 0;
  padding: var(--ugoe-space-2) var(--ugoe-space-3) var(--ugoe-space-3);
  border: 1px solid var(--ugoe-color-border);
  border-radius: var(--ugoe-radius-md);
}

.group.base {
  border-style: dashed;
}

.group header {
  display: flex;
  align-items: center;
  gap: var(--ugoe-space-2);
  margin: var(--ugoe-space-1) 0 var(--ugoe-space-2);
}

.remove {
  margin-left: auto;
}

.entries {
  margin: 0;
  padding-left: 1.5em;
}

.entries li {
  margin: var(--ugoe-space-2) 0;
  border-top: 2px solid transparent;
  border-bottom: 2px solid transparent;
}

.entries .entry.grab {
  cursor: grab;
}

.entries li.dragging,
.entries li.dragging .entry {
  cursor: grabbing;
}

.entries li.dragging {
  opacity: 0.5;
}

.entries li.drop-before {
  border-top-color: var(--ugoe-color-accent);
}

.entries li.drop-after {
  border-bottom-color: var(--ugoe-color-accent);
}

.check {
  margin: var(--ugoe-space-2) 0 0;
}

.entry {
  display: flex;
  align-items: center;
  gap: var(--ugoe-space-2);
  min-width: 0;
}

.entry .name {
  min-width: 0;
  border: 1px solid transparent;
  background: none;
  padding: 0 var(--ugoe-space-1);
  text-align: left;
  overflow-wrap: anywhere;
}

.entry .name.selected {
  border-color: var(--ugoe-color-accent);
}

.buttons {
  display: flex;
  flex: none;
  gap: var(--ugoe-space-1);
  margin-left: auto;
}

.tag {
  flex: none;
  padding: 0 var(--ugoe-space-2);
  border: 1px solid var(--ugoe-color-accent);
  border-radius: var(--ugoe-radius-sm);
  color: var(--ugoe-color-accent);
  font-size: var(--ugoe-font-size-xs);
}

.row {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ugoe-space-2);
  align-items: center;
  margin: var(--ugoe-space-2) 0;
}

.pick {
  display: grid;
  flex: 1;
  min-width: 0;
}
</style>
