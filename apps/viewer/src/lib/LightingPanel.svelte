<script lang="ts">
// 光源のパネル (T0705)。ライティングを使うモデル (`WebPuppet.uses_lighting`) のときだけ App.svelte が出す。
// 環境光・平行光源 (向き)・点光源 (位置と範囲) の色と強さ、既定の光源の組。設定は Player がモデルごとに覚え (T0709)、
// 「既定に戻す」で開いているモデルを「標準」にして覚える (T0714)。覚えた設定を忘れるのは「覚えたもの」の一覧。
// スライダー・面をドラッグしている途中は描画だけを変え、離したときに覚える (`preview` と `commitLighting`)。
// 色はすぐ覚える (色の選択の枠の中の操作は、どれも確定した値として届くため)。
// ビューポートの光源の印 (LightMarkers.svelte、T0708) の表示もここで切り替える。
// 割合の値 (横・縦・手前・範囲) は、スライダーでは % で出す (保存する値は 0..1 のまま)。

import {
  anglesFromPad,
  defaultLighting,
  LIMITS,
  type LightColor,
  type LightingSettings,
  lightingLabel,
  type Player,
  PRESETS,
  padFromAngles,
  sameLighting,
} from "@ugoe/player";
import { Button, ColorPicker, Section, Slider, Toggle } from "@ugoe/ui";
import { t } from "./i18n";
import LightPad from "./LightPad.svelte";

let { player, onshowmemory }: { player: Player; onshowmemory?: () => void } = $props();

let open = $state(true);

let s = $derived(player.lighting);
let preset = $derived(PRESETS.find((p) => sameLighting(p.settings, s)) ?? null);
let directionPad = $derived(padFromAngles(s.directional.azimuth, s.directional.elevation));
// 既定 (標準) をこのモデル用に覚えていれば、戻すものが無い。
let canReset = $derived(!player.lightingRemembered || !sameLighting(s, defaultLighting()));

function edit(change: (settings: LightingSettings) => void, preview = false) {
  player.updateLighting(change, { preview });
}

function applyPreset(settings: LightingSettings) {
  player.setLighting(structuredClone(settings));
}

/** スライダーの表し方: そのまま (`1`)、% (`100`。値は 0..1)。 */
type Scale = 1 | 100;
</script>

{#snippet slider(
  label: string,
  value: number,
  [min, max]: readonly [number, number],
  step: number,
  scale: Scale,
  unit: string | undefined,
  set: (settings: LightingSettings, v: number) => void,
  disabled = false,
)}
  <Slider
    {label}
    value={Math.round(value * scale * 1000) / 1000}
    min={min * scale}
    max={max * scale}
    step={step * scale}
    {unit}
    disabled={disabled || !s.enabled}
    onchange={(v) => edit((next) => set(next, v / scale), true)}
    oncommit={() => player.commitLighting()}
  />
{/snippet}

{#snippet colorRow(
  label: string,
  light: LightColor,
  pick: (settings: LightingSettings) => LightColor,
  disabled = false,
)}
  <div class="color-row">
    <ColorPicker
      ariaLabel={t("lighting.color.aria", { light: label })}
      value={light.color}
      disabled={disabled || !s.enabled}
      onchange={(color) =>
        edit((next) => {
          pick(next).color = color;
        })}
    />
    {@render slider(
      t("lighting.intensity.label"),
      light.intensity,
      LIMITS.intensity,
      0.01,
      1,
      undefined,
      (next, v) => {
        pick(next).intensity = v;
      },
      disabled,
    )}
  </div>
{/snippet}

<div class="panel lighting">
  <Section title={t("lighting.title")} level={2} bind:open>
    {#snippet actions()}
      <span class="status">{lightingLabel(s)}</span>
    {/snippet}

    <div class="toggles">
      <Toggle
        label={t("lighting.enabled.label")}
        checked={s.enabled}
        onchange={(enabled) =>
          edit((next) => {
            next.enabled = enabled;
          })}
      />
      <Toggle
        label={t("lighting.markers.label")}
        checked={player.lightMarkers}
        onchange={(on) => player.setLightMarkers(on)}
      />
    </div>

    <div class="memory">
      <span>
        {player.lightingRemembered ? t("lighting.memory.remembered") : t("lighting.memory.none")}
      </span>
      <span class="memory-actions">
        <Button small disabled={!canReset} onclick={() => player.resetLighting()}
          >{t("lighting.reset.label")}</Button
        >
        {#if onshowmemory}
          <Button small onclick={onshowmemory}>{t("lighting.showMemory.label")}</Button>
        {/if}
      </span>
    </div>

    <div class="presets" role="group" aria-label={t("lighting.presets.aria")}>
      {#each PRESETS as p (p.id)}
        <Button small pressed={preset?.id === p.id} onclick={() => applyPreset(p.settings)}
          >{p.name}</Button
        >
      {/each}
    </div>

    <section aria-labelledby="lighting-ambient">
      <h3 id="lighting-ambient">{t("lighting.ambient.title")}</h3>
      {@render colorRow(t("lighting.ambient.title"), s.ambient, (next) => next.ambient)}
    </section>

    <section>
      <Toggle
        label={t("lighting.directional.title")}
        checked={s.directional.enabled}
        disabled={!s.enabled}
        onchange={(enabled) =>
          edit((next) => {
            next.directional.enabled = enabled;
          })}
      />
      <div class="light">
        <div class="pad">
          <LightPad
            round
            label={t("lighting.directional.pad.aria")}
            x={directionPad[0]}
            y={directionPad[1]}
            disabled={!s.enabled || !s.directional.enabled}
            onchange={(x, y, preview) => {
              const [azimuth, elevation] = anglesFromPad(x, y);
              edit((next) => {
                next.directional.azimuth = azimuth;
                next.directional.elevation = elevation;
              }, preview);
            }}
            oncommit={() => player.commitLighting()}
          />
        </div>
        <div class="controls">
          {@render slider(
            t("lighting.directional.azimuth.label"),
            s.directional.azimuth,
            LIMITS.azimuth,
            1,
            1,
            "°",
            (next, v) => {
              next.directional.azimuth = v;
            },
            !s.directional.enabled,
          )}
          {@render slider(
            t("lighting.directional.elevation.label"),
            s.directional.elevation,
            LIMITS.elevation,
            1,
            1,
            "°",
            (next, v) => {
              next.directional.elevation = v;
            },
            !s.directional.enabled,
          )}
          {@render colorRow(
            t("lighting.directional.title"),
            s.directional,
            (next) => next.directional,
            !s.directional.enabled,
          )}
        </div>
      </div>
    </section>

    <section>
      <Toggle
        label={t("lighting.point.title")}
        checked={s.point.enabled}
        disabled={!s.enabled}
        onchange={(enabled) =>
          edit((next) => {
            next.point.enabled = enabled;
          })}
      />
      <div class="light">
        <div class="pad">
          <LightPad
            label={t("lighting.point.pad.aria")}
            x={Math.min(1, Math.max(-1, s.point.x * 2 - 1))}
            y={Math.min(1, Math.max(-1, s.point.y * 2 - 1))}
            disabled={!s.enabled || !s.point.enabled}
            onchange={(x, y, preview) => {
              edit((next) => {
                next.point.x = Math.round(((x + 1) / 2) * 1000) / 1000;
                next.point.y = Math.round(((y + 1) / 2) * 1000) / 1000;
              }, preview);
            }}
            oncommit={() => player.commitLighting()}
          />
        </div>
        <div class="controls">
          {@render slider(
            t("lighting.point.x.label"),
            s.point.x,
            LIMITS.x,
            0.01,
            100,
            "%",
            (next, v) => {
              next.point.x = v;
            },
            !s.point.enabled,
          )}
          {@render slider(
            t("lighting.point.y.label"),
            s.point.y,
            LIMITS.y,
            0.01,
            100,
            "%",
            (next, v) => {
              next.point.y = v;
            },
            !s.point.enabled,
          )}
          {@render slider(
            t("lighting.point.z.label"),
            s.point.z,
            LIMITS.z,
            0.01,
            100,
            "%",
            (next, v) => {
              next.point.z = v;
            },
            !s.point.enabled,
          )}
          {@render slider(
            t("lighting.point.range.label"),
            s.point.range,
            LIMITS.range,
            0.01,
            100,
            "%",
            (next, v) => {
              next.point.range = v;
            },
            !s.point.enabled,
          )}
          {@render colorRow(t("lighting.point.title"), s.point, (next) => next.point, !s.point.enabled)}
        </div>
      </div>
    </section>
    <p class="hint">{t("lighting.hint")}</p>
  </Section>
</div>

<style>
  /* 縦が足りないときは、見出しが見える高さまで縮んで中を送る (パラメータの一覧の高さを残す)。 */
  .lighting {
    flex: 0 1 auto;
    min-height: calc(var(--ugoe-control-height) + var(--ugoe-space-3));
    max-height: 60%;
    overflow-y: auto;
    font-size: var(--ugoe-font-size-sm);
  }
  .lighting :global(.ugoe-slider),
  .lighting :global(.ugoe-toggle) {
    font-size: var(--ugoe-font-size-sm);
  }
  h3 {
    margin: 0 0 var(--ugoe-space-1);
    font-size: var(--ugoe-font-size-sm);
    font-weight: var(--ugoe-font-weight-strong);
  }
  .status,
  .memory,
  .hint {
    color: var(--ugoe-color-text-muted);
  }
  .status {
    font-size: var(--ugoe-font-size-xs);
  }
  section {
    margin-top: var(--ugoe-space-4);
  }
  .toggles {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-2);
  }
  .memory {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--ugoe-space-2);
    margin-top: var(--ugoe-space-3);
    font-size: var(--ugoe-font-size-xs);
  }
  .memory-actions {
    display: flex;
    gap: var(--ugoe-space-2);
  }
  .presets {
    display: flex;
    flex-wrap: wrap;
    gap: var(--ugoe-space-2);
    margin-top: var(--ugoe-space-3);
  }
  .light {
    display: grid;
    grid-template-columns: 72px minmax(0, 1fr);
    gap: var(--ugoe-space-3);
    align-items: start;
    margin-top: var(--ugoe-space-2);
  }
  .controls {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-2);
    min-width: 0;
  }
  .color-row {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-1);
  }
  .hint {
    margin: var(--ugoe-space-3) 0 0;
    font-size: var(--ugoe-font-size-xs);
  }
</style>
