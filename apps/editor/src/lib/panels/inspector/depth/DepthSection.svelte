<!--
  インスペクタの深度の編集 (docs/editor.md#深度の編集)。主な選択がメッシュかワープデフォーマのときに出す。
  深度は深度からの回転 (推定の節) の入力で、プリセット (一様・ドーム)、選んだ点の値、外す操作がある。
  操作はすべて set_depth / apply_depth_preset を送るだけ (depth.ts で組み立てる)。set_depth は深度を丸ごと置き換えるので、
  送る直前の要素から組み立てる (send)。

  - 選んだ点: メッシュはメッシュ編集モードで選んだ頂点、ワープはギズモで選んだ制御点。
  - 「Show in viewport」で、ビューポートに点を深度の色 (手前が赤、奥が青) で重ねる (tools/depth/DepthOverlay.svelte)。
-->
<script lang="ts">
import { Button, Checkbox, NumberField } from "@ugoe/ui";
import type { EditorDocument } from "../../../document.svelte";
import { t } from "../../../i18n";
import type { Command } from "../../../ipc";
import type { ElementRef, Selection } from "../../../selection.svelte";
import TipButton from "../../../ui/TipButton.svelte";
import type { ViewportController } from "../../../viewport/controller.svelte";
import {
  type DepthElement,
  depthElement,
  describeDepth,
  finiteValue,
  pointsCommand,
  presetCommand,
  setDepthCommand,
} from "./depth";

interface Props {
  doc: EditorDocument;
  selection: Selection;
  viewport: ViewportController;
}

let { doc, selection, viewport }: Props = $props();

const model = $derived(doc.parsedModel);
const element = $derived(model ? depthElement(model, selection.primary) : null);

let uniform = $state(0);
let peak = $state(20);
let pointValue = $state(10);
/** 編集を送っている間はボタンを無効にする (docs/editor.md#パネルからの編集)。 */
const busy = $derived(doc.pending > 0);

/** 選んだ点 (メッシュの頂点・ワープの制御点)。 */
const picked = $derived.by((): number[] => {
  if (!element) return [];
  const id = element.target.id;
  return (element.kind === "mesh" ? selection.vertices[id] : selection.controlPoints[id]) ?? [];
});
const pickHint = $derived(
  element?.kind === "mesh"
    ? t("inspector.depth.pickHint.mesh")
    : t("inspector.depth.pickHint.warp"),
);

/** 前の編集を読み直した後の、選んでいる要素の深度からコマンドを組み立てて送る (要素が消えていれば送らない)。 */
function send(build: (current: DepthElement) => Command | null): void {
  const ref: ElementRef | null = selection.primary;
  void doc.send(() => {
    const current = model ? depthElement(model, ref) : null;
    return current ? build(current) : null;
  });
}

function applyUniform(): void {
  const value = finiteValue(uniform);
  if (value !== null) send((e) => presetCommand(e.target, { kind: "uniform", value }));
}

function applyDome(): void {
  const value = finiteValue(peak);
  if (value !== null) send((e) => presetCommand(e.target, { kind: "dome", peak: value }));
}

const ids = $props.id();

function applyPoints(): void {
  const value = finiteValue(pointValue);
  const points = [...picked];
  if (value !== null) send((e) => pointsCommand(e, points, value));
}
</script>

{#if element}
  <section>
    <h3>{t("inspector.depth.title")}</h3>
    <p class="muted">
      {t("inspector.depth.summary", {
        points:
          element.kind === "mesh"
            ? t("inspector.depth.vertices", { count: element.points })
            : t("inspector.depth.controlPoints", { count: element.points }),
        depth: describeDepth(element.depth),
      })}
    </p>
    <Checkbox label={t("inspector.depth.show")} bind:checked={viewport.showDepth} />

    <div class="settings">
      <div class="ed-field">
        <span id="{ids}-uniform">{t("inspector.depth.uniform.label")}</span>
        <span class="row">
          <NumberField aria-labelledby="{ids}-uniform" size={4} bind:value={uniform} />
          <Button small disabled={busy || finiteValue(uniform) === null} onclick={applyUniform}
            >{t("inspector.depth.uniform.apply")}</Button
          >
        </span>
      </div>
      <div class="ed-field">
        <span id="{ids}-peak">{t("inspector.depth.dome.label")}</span>
        <span class="row">
          <NumberField aria-labelledby="{ids}-peak" size={4} bind:value={peak} />
          <TipButton
            small
            disabled={busy || finiteValue(peak) === null}
            onclick={applyDome}
            tooltip={t("inspector.depth.dome.tooltip")}>{t("inspector.depth.dome.apply")}</TipButton
          >
        </span>
      </div>
      <div class="ed-field">
        <span id="{ids}-points">{t("inspector.depth.selected.label", { count: picked.length })}</span>
        <span class="row">
          <NumberField aria-labelledby="{ids}-points" size={4} bind:value={pointValue} />
          <Button
            small
            disabled={busy || picked.length === 0 || finiteValue(pointValue) === null}
            onclick={applyPoints}>{t("inspector.depth.selected.apply")}</Button
          >
        </span>
      </div>
    </div>
    {#if picked.length === 0}
      <p class="muted hint">{pickHint}</p>
    {/if}
    <span class="row">
      <Button
        small
        disabled={busy || element.depth === null}
        onclick={() => send((e) => setDepthCommand(e.target, null))}
        >{t("inspector.depth.clear")}</Button
      >
    </span>
  </section>
{/if}

<style>
p {
  margin: var(--ugoe-space-2) 0;
}

.settings {
  display: grid;
  gap: var(--ugoe-space-2);
  margin: var(--ugoe-space-3) 0;
}

.settings .ed-field {
  margin: 0;
}

.ed-field .row {
  flex-wrap: nowrap;
  margin: 0;
}

.row {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ugoe-space-2);
  align-items: center;
  margin: var(--ugoe-space-2) 0;
}

.hint {
  font-size: var(--ugoe-font-size-sm);
}
</style>
