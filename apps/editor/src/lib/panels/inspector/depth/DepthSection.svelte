<!--
  インスペクタの深度の編集 (docs/editor.md#深度の編集)。主な選択がメッシュかワープデフォーマのときに出す。
  深度は深度からの回転 (推定の節) の入力で、プリセット (一様・ドーム)、選んだ点の値、外す操作がある。
  操作はすべて set_depth / apply_depth_preset を送るだけ (depth.ts で組み立てる)。

  - 選んだ点: メッシュはメッシュ編集モードで選んだ頂点、ワープはギズモで選んだ制御点。
  - 「Show in viewport」で、ビューポートに点を深度の色 (手前が赤、奥が青) で重ねる (tools/depth/DepthOverlay.svelte)。
-->
<script lang="ts">
import { Button, Checkbox, NumberField } from "@ugoe/ui";
import type { EditorDocument } from "../../../document.svelte";
import { t } from "../../../i18n";
import type { Command, Model } from "../../../ipc";
import type { Selection } from "../../../selection.svelte";
import TipButton from "../../../ui/TipButton.svelte";
import type { ViewportController } from "../../../viewport/controller.svelte";
import {
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

const model = $derived.by((): Model | null => {
  try {
    return doc.model ? (JSON.parse(doc.model.json) as Model) : null;
  } catch {
    return null;
  }
});
const element = $derived(model ? depthElement(model, selection.primary) : null);

let uniform = $state(0);
let peak = $state(20);
let pointValue = $state(10);
let busy = $state(false);

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

async function send(command: Command | null): Promise<void> {
  if (!command || busy) return;
  busy = true;
  try {
    await doc.apply(command);
  } finally {
    busy = false;
  }
}

function applyUniform(): void {
  const value = finiteValue(uniform);
  if (element && value !== null)
    void send(presetCommand(element.target, { kind: "uniform", value }));
}

function applyDome(): void {
  const value = finiteValue(peak);
  if (element && value !== null) {
    void send(presetCommand(element.target, { kind: "dome", peak: value }));
  }
}

const ids = $props.id();

function applyPoints(): void {
  const value = finiteValue(pointValue);
  if (element && value !== null) void send(pointsCommand(element, picked, value));
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
      <div class="field">
        <span id="{ids}-uniform">{t("inspector.depth.uniform.label")}</span>
        <span class="row">
          <NumberField labelledby="{ids}-uniform" size={4} bind:value={uniform} />
          <Button small disabled={busy || finiteValue(uniform) === null} onclick={applyUniform}
            >{t("inspector.depth.uniform.apply")}</Button
          >
        </span>
      </div>
      <div class="field">
        <span id="{ids}-peak">{t("inspector.depth.dome.label")}</span>
        <span class="row">
          <NumberField labelledby="{ids}-peak" size={4} bind:value={peak} />
          <TipButton
            small
            disabled={busy || finiteValue(peak) === null}
            onclick={applyDome}
            tooltip={t("inspector.depth.dome.tooltip")}>{t("inspector.depth.dome.apply")}</TipButton
          >
        </span>
      </div>
      <div class="field">
        <span id="{ids}-points">{t("inspector.depth.selected.label", { count: picked.length })}</span>
        <span class="row">
          <NumberField labelledby="{ids}-points" size={4} bind:value={pointValue} />
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
        onclick={() => element && send(setDepthCommand(element.target, null))}
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

.settings .field {
  display: grid;
  grid-template-columns: minmax(5em, auto) 1fr;
  align-items: center;
  gap: var(--ugoe-space-2);
}

.field .row {
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
