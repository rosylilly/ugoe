<!--
  プレビューの光源 (docs/editor.md#ライティングの画面)。ライティングのパネルとプレビューのパネルの両方に置く。
  環境光・平行光源・点光源の色と強さ、選んだ光源の向き・位置を変える。向きと位置はビューポートの取っ手でも動かせる
  (viewport/tools/lighting/LightingOverlay.svelte)。スライダーは動かしている間は表示だけを変え、離したときにプロジェクトに残す。
-->
<script lang="ts">
import type { PreviewLight, PreviewLighting } from "../../ipc";
import {
  DEFAULT_PREVIEW_LIGHTING,
  directionToHandle,
  elevationDegrees,
  handleToDirection,
  hexToRgb,
  MAX_PREVIEW_LIGHTS,
  newLight,
  type Rgb,
  rgbToHex,
} from "./lighting";
import type { LightingView } from "./lightingView.svelte";

interface Props {
  view: LightingView;
  canvas: { width: number; height: number };
}

let { view, canvas }: Props = $props();

const settings = $derived(view.settings);
const selected = $derived(view.selected === null ? null : (settings.lights[view.selected] ?? null));

function num(e: Event): number {
  return Number((e.currentTarget as HTMLInputElement).value);
}

function color(e: Event): Rgb | null {
  return hexToRgb((e.currentTarget as HTMLInputElement).value);
}

/** 入力中は表示だけ、確定 (change) でプロジェクトに残す。 */
function update(next: PreviewLighting, commit: boolean): void {
  if (commit) void view.commit(next);
  else view.preview(next);
}

function updateLight(index: number, light: PreviewLight, commit: boolean): void {
  update(view.withLight(index, light), commit);
}

function addLight(kind: PreviewLight["type"]): void {
  const lights = [...settings.lights, newLight(kind, canvas)];
  view.selected = lights.length - 1;
  void view.commit({ ...settings, enabled: true, lights });
}

function removeLight(index: number): void {
  const lights = settings.lights.filter((_, i) => i !== index);
  view.selected = lights.length > 0 ? Math.min(index, lights.length - 1) : null;
  void view.commit({ ...settings, lights });
}

function setBehind(
  index: number,
  light: Extract<PreviewLight, { type: "directional" }>,
  behind: boolean,
) {
  const h = directionToHandle(light.direction);
  updateLight(index, { ...light, direction: handleToDirection(h.x, h.y, behind) }, true);
}

function setAxis(index: number, light: PreviewLight, axis: number, value: number): void {
  if (!Number.isFinite(value)) return;
  if (light.type === "directional") {
    const direction = [...light.direction] as [number, number, number];
    direction[axis] = value;
    if (direction.every((v) => v === 0)) return;
    updateLight(index, { ...light, direction }, true);
  } else {
    const position = [...light.position] as [number, number, number];
    position[axis] = value;
    updateLight(index, { ...light, position }, true);
  }
}

function lightName(light: PreviewLight, index: number): string {
  return `${index + 1}. ${light.type === "directional" ? "Directional" : "Point"}`;
}

const fmt = (v: number) => String(Math.round(v * 100) / 100);
</script>

<section aria-label="Preview lights">
  <h3>Preview lights</h3>
  <label class="row">
    <span>Light the viewport</span>
    <input
      type="checkbox"
      checked={settings.enabled}
      onchange={(e) => update({ ...settings, enabled: (e.currentTarget as HTMLInputElement).checked }, true)}
    />
  </label>
  <p class="muted note">
    Lights are a preview setting kept in the project, not part of the model. Apps that show the model pass their own
    lights.
  </p>
  {#if view.error}<p class="error">{view.error}</p>{/if}

  <div class="row">
    <span>Ambient</span>
    <span class="controls">
      <input
        type="color"
        aria-label="Ambient color"
        value={rgbToHex(settings.ambient)}
        onchange={(e) => {
          const c = color(e);
          if (c) update({ ...settings, ambient: c }, true);
        }}
      />
      <input
        type="range"
        aria-label="Ambient intensity"
        min="0"
        max="2"
        step="0.01"
        value={settings.ambient_intensity}
        oninput={(e) => update({ ...settings, ambient_intensity: num(e) }, false)}
        onchange={(e) => update({ ...settings, ambient_intensity: num(e) }, true)}
      />
      <span class="value">{fmt(settings.ambient_intensity)}</span>
    </span>
  </div>

  <ul class="lights">
    {#each settings.lights as light, i (i)}
      <li class:current={view.selected === i}>
        <button class="name" onclick={() => (view.selected = i)} aria-pressed={view.selected === i}
          >{lightName(light, i)}</button>
        <input
          type="color"
          aria-label="Light {i + 1} color"
          value={rgbToHex(light.color)}
          onchange={(e) => {
            const c = color(e);
            if (c) updateLight(i, { ...light, color: c }, true);
          }}
        />
        <input
          type="range"
          aria-label="Light {i + 1} intensity"
          min="0"
          max="3"
          step="0.01"
          value={light.intensity}
          oninput={(e) => updateLight(i, { ...light, intensity: num(e) }, false)}
          onchange={(e) => updateLight(i, { ...light, intensity: num(e) }, true)}
        />
        <span class="value">{fmt(light.intensity)}</span>
        <button onclick={() => removeLight(i)} aria-label="Remove light {i + 1}">×</button>
      </li>
    {/each}
  </ul>
  <div class="row buttons">
    <button disabled={settings.lights.length >= MAX_PREVIEW_LIGHTS} onclick={() => addLight("directional")}
      >Add directional</button>
    <button disabled={settings.lights.length >= MAX_PREVIEW_LIGHTS} onclick={() => addLight("point")}
      >Add point</button>
    <button onclick={() => void view.commit(DEFAULT_PREVIEW_LIGHTING)} title="Weak ambient and one light from the top left">
      Reset</button>
  </div>

  {#if selected && view.selected !== null}
    {@const i = view.selected}
    <h4>{lightName(selected, i)}</h4>
    {#if selected.type === "directional"}
      <p class="muted note">
        Drag the handle on the circle in the viewport to aim the light. Elevation {Math.round(
          elevationDegrees(selected.direction),
        )}° (90° = straight from the front).
      </p>
      <label class="row">
        <span>From behind the canvas</span>
        <input
          type="checkbox"
          checked={directionToHandle(selected.direction).behind}
          onchange={(e) => setBehind(i, selected, (e.currentTarget as HTMLInputElement).checked)}
        />
      </label>
    {:else}
      <p class="muted note">Drag the light in the viewport to move it. Z is the distance in front of the canvas.</p>
    {/if}
    <div class="row">
      <span>{selected.type === "directional" ? "Direction" : "Position"}</span>
      <span class="controls">
        {#each ["x", "y", "z"] as axis, a (axis)}
          {@const v = selected.type === "directional" ? selected.direction[a] : selected.position[a]}
          <input
            type="number"
            class="axis"
            aria-label="{axis}"
            step={selected.type === "directional" ? 0.1 : 10}
            value={fmt(v ?? 0)}
            onchange={(e) => setAxis(i, selected, a, num(e))}
          />
        {/each}
      </span>
    </div>
    {#if selected.type === "point"}
      <label class="row">
        <span>Range (px)</span>
        <span class="controls">
          <input
            type="checkbox"
            aria-label="Limit the range"
            checked={selected.range !== null}
            onchange={(e) =>
              updateLight(
                i,
                {
                  ...selected,
                  range: (e.currentTarget as HTMLInputElement).checked
                    ? Math.round(Math.max(canvas.width, canvas.height) * 1.2)
                    : null,
                },
                true,
              )}
          />
          {#if selected.range !== null}
            <input
              type="number"
              class="axis"
              aria-label="Range"
              min="1"
              step="10"
              value={fmt(selected.range)}
              onchange={(e) => {
                const r = num(e);
                if (r > 0) updateLight(i, { ...selected, range: r }, true);
              }}
            />
          {:else}
            <span class="muted">unlimited</span>
          {/if}
        </span>
      </label>
    {/if}
  {/if}
</section>

<style>
.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin: 4px 0;
}

.controls {
  display: flex;
  align-items: center;
  gap: 4px;
  min-width: 0;
}

.controls input[type="range"],
.lights input[type="range"] {
  width: 90px;
  min-width: 0;
}

input[type="color"] {
  width: 28px;
  height: 22px;
  padding: 0;
  border: 1px solid var(--border);
  background: none;
}

.value {
  width: 32px;
  text-align: right;
  font-variant-numeric: tabular-nums;
}

.axis {
  width: 58px;
}

.lights {
  list-style: none;
  margin: 4px 0;
  padding: 0;
}

.lights li {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 2px 0;
}

.lights li.current .name {
  color: var(--accent);
  border-color: var(--accent);
}

.lights .name {
  flex: 1;
  min-width: 0;
  text-align: left;
}

.buttons {
  justify-content: flex-start;
  flex-wrap: wrap;
}

.note {
  margin: 2px 0 6px;
  font-size: 11px;
}

h3,
h4 {
  margin: 8px 0 4px;
}
</style>
