<!--
  インスペクタの深度の編集 (docs/editor.md#深度の編集)。主な選択がメッシュかワープデフォーマのときに出す。
  深度は深度からの回転 (推定の節) の入力で、プリセット (一様・ドーム)、選んだ点の値、外す操作がある。
  操作はすべて set_depth / apply_depth_preset を送るだけ (depth.ts で組み立てる)。

  - 選んだ点: メッシュはメッシュ編集モードで選んだ頂点、ワープはギズモで選んだ制御点。
  - 「Show in viewport」で、ビューポートに点を深度の色 (手前が赤、奥が青) で重ねる (tools/depth/DepthOverlay.svelte)。
-->
<script lang="ts">
import type { EditorDocument } from "../../../document.svelte";
import type { Command, Model } from "../../../ipc";
import type { Selection } from "../../../selection.svelte";
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

let uniform = $state<number | null>(0);
let peak = $state<number | null>(20);
let pointValue = $state<number | null>(10);
let busy = $state(false);

/** 選んだ点 (メッシュの頂点・ワープの制御点)。 */
const picked = $derived.by((): number[] => {
  if (!element) return [];
  const id = element.target.id;
  return (element.kind === "mesh" ? selection.vertices[id] : selection.controlPoints[id]) ?? [];
});
const pickHint = $derived(
  element?.kind === "mesh"
    ? "Select vertices in mesh edit mode."
    : "Select control points with the gizmo.",
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

function applyPoints(): void {
  const value = finiteValue(pointValue);
  if (element && value !== null) void send(pointsCommand(element, picked, value));
}
</script>

{#if element}
  <section>
    <h3>Depth</h3>
    <p class="muted">
      {element.points}
      {element.kind === "mesh" ? "vertices" : "control points"} · {describeDepth(element.depth)}
    </p>
    <label class="check">
      <input type="checkbox" bind:checked={viewport.showDepth} />
      Show in viewport
    </label>

    <div class="settings">
      <div class="field">
        <span>Uniform</span>
        <span class="row">
          <input type="number" step="1" bind:value={uniform} aria-label="Uniform depth" />
          <button disabled={busy || finiteValue(uniform) === null} onclick={applyUniform}>Set all</button>
        </span>
      </div>
      <div class="field">
        <span>Dome peak</span>
        <span class="row">
          <input type="number" step="1" bind:value={peak} aria-label="Dome peak" />
          <button
            disabled={busy || finiteValue(peak) === null}
            onclick={applyDome}
            title="Depth rises from 0 on the ellipse inscribed in the rest-pose bounds to the peak at its center"
            >Dome</button
          >
        </span>
      </div>
      <div class="field">
        <span>Selected ({picked.length})</span>
        <span class="row">
          <input type="number" step="1" bind:value={pointValue} aria-label="Selected points depth" />
          <button
            disabled={busy || picked.length === 0 || finiteValue(pointValue) === null}
            onclick={applyPoints}>Set selected</button
          >
        </span>
      </div>
    </div>
    {#if picked.length === 0}
      <p class="muted hint">{pickHint}</p>
    {/if}
    <span class="row">
      <button
        disabled={busy || element.depth === null}
        onclick={() => element && send(setDepthCommand(element.target, null))}>Clear depth</button
      >
    </span>
  </section>
{/if}

<style>
h3 {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
}

p {
  margin: 4px 0;
}

.settings {
  display: grid;
  gap: 4px;
  margin: 6px 0;
}

.settings .field {
  display: grid;
  grid-template-columns: 7em 1fr;
  align-items: center;
  gap: 4px;
}

.settings input[type="number"] {
  width: 4.5em;
  min-width: 0;
}

.field .row {
  flex-wrap: nowrap;
  margin: 0;
}

.check {
  display: flex;
  align-items: center;
  gap: 4px;
}

.row {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  align-items: center;
  margin: 4px 0;
}

.hint {
  font-size: 11px;
}
</style>
