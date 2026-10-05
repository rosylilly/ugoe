<!--
  プレビューの光源 (docs/editor.md#ライティングの画面)。ライティングのパネルとプレビューのパネルの両方に置く。
  環境光・平行光源・点光源の色と強さ、選んだ光源の向き・位置を変える。向きと位置はビューポートの取っ手でも動かせる
  (viewport/tools/lighting/LightingOverlay.svelte)。スライダーは動かしている間は表示だけを変え、離したときにプロジェクトに残す。
-->
<script lang="ts">
import { Button, Icon, IconButton, NumberField, Slider, Toggle } from "@ugoe/ui";
import { t } from "../../i18n";
import type { PreviewLight, PreviewLighting } from "../../ipc";
import TipButton from "../../ui/TipButton.svelte";
import LightColor from "./LightColor.svelte";
import {
  DEFAULT_PREVIEW_LIGHTING,
  directionToHandle,
  elevationDegrees,
  handleToDirection,
  MAX_PREVIEW_LIGHTS,
  newLight,
} from "./lighting";
import type { LightingView } from "./lightingView.svelte";

interface Props {
  view: LightingView;
  canvas: { width: number; height: number };
}

let { view, canvas }: Props = $props();

const settings = $derived(view.settings);
const selected = $derived(view.selected === null ? null : (settings.lights[view.selected] ?? null));

const headingId = $props.id();
const axesId = `${headingId}-axes`;

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
  return light.type === "directional"
    ? t("lighting.lights.name.directional", { index: index + 1 })
    : t("lighting.lights.name.point", { index: index + 1 });
}
</script>

<section aria-labelledby={headingId}>
  <h3 id={headingId}>{t("lighting.lights.title")}</h3>
  <div class="row">
    <Toggle
      label={t("lighting.lights.enabled.label")}
      checked={settings.enabled}
      onchange={(enabled) => update({ ...settings, enabled }, true)}
    />
  </div>
  <p class="ed-hint note">{t("lighting.lights.description")}</p>
  {#if view.error}<p class="error ugoe-selectable">{view.error}</p>{/if}

  <div class="row">
    <span>{t("lighting.lights.ambient.label")}</span>
    <span class="controls">
      <LightColor
        value={settings.ambient}
        ariaLabel={t("lighting.lights.ambient.color.aria")}
        onpreview={(ambient) => update({ ...settings, ambient }, false)}
        oncommit={(ambient) => update({ ...settings, ambient }, true)}
      />
      <span class="slider">
        <Slider
          label={t("lighting.lights.ambient.intensity.aria")}
          value={settings.ambient_intensity}
          min={0}
          max={2}
          step={0.01}
          decimals={2}
          onchange={(v) => update({ ...settings, ambient_intensity: v }, false)}
          oncommit={(v) => update({ ...settings, ambient_intensity: v }, true)}
        />
      </span>
    </span>
  </div>

  <ul class="lights">
    {#each settings.lights as light, i (i)}
      <li class:current={view.selected === i}>
        <Button small class="name" pressed={view.selected === i} onclick={() => (view.selected = i)}
          >{lightName(light, i)}</Button
        >
        <LightColor
          value={light.color}
          ariaLabel={t("lighting.lights.color.aria", { index: i + 1 })}
          onpreview={(color) => updateLight(i, { ...light, color }, false)}
          oncommit={(color) => updateLight(i, { ...light, color }, true)}
        />
        <span class="slider">
          <Slider
            label={t("lighting.lights.intensity.aria", { index: i + 1 })}
            value={light.intensity}
            min={0}
            max={3}
            step={0.01}
            decimals={2}
            onchange={(v) => updateLight(i, { ...light, intensity: v }, false)}
            oncommit={(v) => updateLight(i, { ...light, intensity: v }, true)}
          />
        </span>
        <IconButton small label={t("lighting.lights.remove.aria", { index: i + 1 })} onclick={() => removeLight(i)}
          ><Icon name="close" /></IconButton
        >
      </li>
    {/each}
  </ul>
  <div class="ed-buttons row">
    <Button disabled={settings.lights.length >= MAX_PREVIEW_LIGHTS} onclick={() => addLight("directional")}
      >{t("lighting.lights.addDirectional.label")}</Button
    >
    <Button disabled={settings.lights.length >= MAX_PREVIEW_LIGHTS} onclick={() => addLight("point")}
      >{t("lighting.lights.addPoint.label")}</Button
    >
    <TipButton tooltip={t("lighting.lights.reset.tooltip")} onclick={() => void view.commit(DEFAULT_PREVIEW_LIGHTING)}
      >{t("lighting.lights.reset.label")}</TipButton
    >
  </div>

  {#if selected && view.selected !== null}
    {@const i = view.selected}
    <h4>{lightName(selected, i)}</h4>
    {#if selected.type === "directional"}
      <p class="ed-hint note">
        {t("lighting.lights.directional.description", {
          elevation: Math.round(elevationDegrees(selected.direction)),
        })}
      </p>
      <div class="row">
        <Toggle
          label={t("lighting.lights.behind.label")}
          checked={directionToHandle(selected.direction).behind}
          onchange={(behind) => setBehind(i, selected, behind)}
        />
      </div>
    {:else}
      <p class="ed-hint note">{t("lighting.lights.point.description")}</p>
    {/if}
    <div class="row">
      <span id={axesId}
        >{selected.type === "directional" ? t("lighting.lights.direction.label") : t("lighting.lights.position.label")}</span
      >
      <span class="controls" role="group" aria-labelledby={axesId}>
        {#each ["x", "y", "z"] as axis, a (axis)}
          {@const v = selected.type === "directional" ? selected.direction[a] : selected.position[a]}
          <NumberField
            label={axis}
            size={5}
            step={selected.type === "directional" ? 0.1 : 10}
            decimals={2}
            value={v ?? 0}
            onchange={(value) => setAxis(i, selected, a, value)}
          />
        {/each}
      </span>
    </div>
    {#if selected.type === "point"}
      <div class="row">
        <Toggle
          label={t("lighting.lights.limitRange.label")}
          checked={selected.range !== null}
          onchange={(on) =>
            updateLight(
              i,
              { ...selected, range: on ? Math.round(Math.max(canvas.width, canvas.height) * 1.2) : null },
              true,
            )}
        />
        {#if selected.range !== null}
          <NumberField
            label={t("lighting.lights.range.label")}
            unit={t("lighting.unit.px")}
            min={1}
            step={10}
            decimals={2}
            size={6}
            value={selected.range}
            onchange={(r) => {
              if (r > 0) updateLight(i, { ...selected, range: r }, true);
            }}
          />
        {:else}
          <span class="ed-hint">{t("lighting.lights.range.unlimited")}</span>
        {/if}
      </div>
    {/if}
  {/if}
</section>

<style>
.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: var(--ugoe-space-2) var(--ugoe-space-3);
  margin: var(--ugoe-space-2) 0;
}

.controls {
  display: flex;
  align-items: center;
  gap: var(--ugoe-space-2);
  min-width: 0;
}

.slider {
  flex: 1;
  min-width: 0;
}

/* 行の中のスライダーは名前を見せない (読み上げの名前には使う)。 */
.slider :global(.ugoe-slider__label) {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
}

.slider :global(.ugoe-slider__root) {
  min-width: 60px;
}

.lights {
  list-style: none;
  margin: var(--ugoe-space-2) 0;
  padding: 0;
}

.lights li {
  display: flex;
  align-items: center;
  gap: var(--ugoe-space-2);
  padding: var(--ugoe-space-1) 0;
}

.lights :global(.name) {
  flex: none;
  max-width: 7em;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.note {
  margin: var(--ugoe-space-1) 0 var(--ugoe-space-2);
}

h4 {
  margin: var(--ugoe-space-3) 0 var(--ugoe-space-2);
}
</style>
