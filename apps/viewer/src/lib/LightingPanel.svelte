<script lang="ts">
// 光源のパネル (T0705)。ライティングを使うモデル (`WebPuppet.uses_lighting`) のときだけ App.svelte が出す。
// 環境光・平行光源 (向き)・点光源 (位置と範囲) の色と強さ、既定の光源の組。設定は Player がモデルごとに覚え (T0709)、
// 「既定に戻す」で開いているモデルを「標準」にして覚える (T0714)。覚えた設定を忘れるのは「覚えたもの」の一覧。
// スライダー・色・面をドラッグしている途中は描画だけを変え、離したときに覚える (`preview` と `commitLighting`)。
// ビューポートの光源の印 (LightMarkers.svelte、T0708) の表示もここで切り替える。

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
import LightPad from "./LightPad.svelte";

let { player, onshowmemory }: { player: Player; onshowmemory?: () => void } = $props();

let open = $state(true);

let s = $derived(player.lighting);
let preset = $derived(PRESETS.find((p) => sameLighting(p.settings, s)) ?? null);
let status = $derived(lightingLabel(s));
let directionPad = $derived(padFromAngles(s.directional.azimuth, s.directional.elevation));
// 既定 (標準) をこのモデル用に覚えていれば、戻すものが無い。
let canReset = $derived(!player.lightingRemembered || !sameLighting(s, defaultLighting()));

function edit(change: (settings: LightingSettings) => void, preview = false) {
  player.updateLighting(change, { preview });
}

function applyPreset(settings: LightingSettings) {
  player.setLighting(structuredClone(settings));
}

const percent = (v: number) => `${Math.round(v * 100)}%`;
const degrees = (v: number) => `${Math.round(v)}°`;
const times = (v: number) => `×${v.toFixed(2)}`;
</script>

{#snippet slider(
  label: string,
  value: number,
  [min, max]: readonly [number, number],
  step: number,
  format: (v: number) => string,
  set: (settings: LightingSettings, v: number) => void,
  disabled = false,
)}
  <label class="slider">
    <span class="label">{label}</span>
    <input
      type="range"
      {min}
      {max}
      {step}
      {value}
      {disabled}
      oninput={(e) => {
        const v = e.currentTarget.valueAsNumber;
        edit((next) => set(next, v), true);
      }}
      onchange={() => player.commitLighting()}
    />
    <span class="value">{format(value)}</span>
  </label>
{/snippet}

{#snippet colorRow(
  label: string,
  light: LightColor,
  pick: (settings: LightingSettings) => LightColor,
  disabled = false,
)}
  <div class="color-row">
    <label class="color" title="{label}の色">
      <input
        type="color"
        value={light.color}
        {disabled}
        oninput={(e) => {
          const color = e.currentTarget.value;
          edit((next) => {
            pick(next).color = color;
          }, true);
        }}
        onchange={() => player.commitLighting()}
      />
    </label>
    {@render slider(
      "強さ",
      light.intensity,
      LIMITS.intensity,
      0.01,
      times,
      (next, v) => {
        pick(next).intensity = v;
      },
      disabled,
    )}
  </div>
{/snippet}

<details class="panel lighting" bind:open>
  <summary>
    <h2>ライティング</h2>
    <span class="status">{status}</span>
  </summary>

  <label class="toggle" title="切ると光源を渡さず、ライティングを受けないのと同じ色で描く">
    <input
      type="checkbox"
      checked={s.enabled}
      onchange={(e) => {
        const enabled = e.currentTarget.checked;
        edit((next) => {
          next.enabled = enabled;
        });
      }}
    />
    光源を当てる
  </label>
  <label
    class="toggle"
    title="ビューポートに光源の印を出す。印をドラッグすると、平行光源の向きと点光源の位置を変えられる"
  >
    <input
      type="checkbox"
      checked={player.lightMarkers}
      onchange={(e) => player.setLightMarkers(e.currentTarget.checked)}
    />
    ビューポートに光源の印を出す
  </label>

  <div class="memory">
    <span title="光源の設定はモデルごとに覚える。覚えていないモデルでは、最後に使った設定を使う">
      {player.lightingRemembered ? "このモデル用に覚えた設定" : "このモデル用の設定はまだありません"}
    </span>
    <span class="memory-actions">
      <button
        type="button"
        disabled={!canReset}
        title="このモデルの光源を既定 (標準) に戻す。次に開いたときも標準で始まる"
        onclick={() => player.resetLighting()}>既定に戻す</button
      >
      {#if onshowmemory}
        <button
          type="button"
          title="モデルごとに覚えた光源と着せ替えの組の一覧。ここで忘れられる"
          onclick={onshowmemory}>覚えたもの…</button
        >
      {/if}
    </span>
  </div>

  <div class="presets" role="group" aria-label="既定の光源の組">
    {#each PRESETS as p (p.id)}
      <button
        type="button"
        aria-pressed={preset?.id === p.id}
        class:active={preset?.id === p.id}
        onclick={() => applyPreset(p.settings)}>{p.name}</button
      >
    {/each}
  </div>

  <fieldset disabled={!s.enabled}>
    <section>
      <h3>環境光</h3>
      {@render colorRow("環境光", s.ambient, (next) => next.ambient)}
    </section>

    <section>
      <label class="toggle">
        <input
          type="checkbox"
          checked={s.directional.enabled}
          onchange={(e) => {
            const enabled = e.currentTarget.checked;
            edit((next) => {
              next.directional.enabled = enabled;
            });
          }}
        />
        <h3>平行光源</h3>
      </label>
      <div class="light">
        <div class="pad">
          <LightPad
            round
            label="平行光源の来る向き (中心が正面)"
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
            "方位",
            s.directional.azimuth,
            LIMITS.azimuth,
            1,
            degrees,
            (next, v) => {
              next.directional.azimuth = v;
            },
            !s.directional.enabled,
          )}
          {@render slider(
            "高さ",
            s.directional.elevation,
            LIMITS.elevation,
            1,
            degrees,
            (next, v) => {
              next.directional.elevation = v;
            },
            !s.directional.enabled,
          )}
          {@render colorRow(
            "平行光源",
            s.directional,
            (next) => next.directional,
            !s.directional.enabled,
          )}
        </div>
      </div>
    </section>

    <section>
      <label class="toggle">
        <input
          type="checkbox"
          checked={s.point.enabled}
          onchange={(e) => {
            const enabled = e.currentTarget.checked;
            edit((next) => {
              next.point.enabled = enabled;
            });
          }}
        />
        <h3>点光源</h3>
      </label>
      <div class="light">
        <div class="pad">
          <LightPad
            label="点光源の位置 (キャンバスの上での位置)"
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
            "横",
            s.point.x,
            LIMITS.x,
            0.01,
            percent,
            (next, v) => {
              next.point.x = v;
            },
            !s.point.enabled,
          )}
          {@render slider(
            "縦",
            s.point.y,
            LIMITS.y,
            0.01,
            percent,
            (next, v) => {
              next.point.y = v;
            },
            !s.point.enabled,
          )}
          {@render slider(
            "手前",
            s.point.z,
            LIMITS.z,
            0.01,
            percent,
            (next, v) => {
              next.point.z = v;
            },
            !s.point.enabled,
          )}
          {@render slider(
            "範囲",
            s.point.range,
            LIMITS.range,
            0.01,
            percent,
            (next, v) => {
              next.point.range = v;
            },
            !s.point.enabled,
          )}
          {@render colorRow("点光源", s.point, (next) => next.point, !s.point.enabled)}
        </div>
      </div>
    </section>
  </fieldset>
  <p class="hint">位置・手前・範囲はキャンバスに対する割合 (手前と範囲は長い辺に対する割合)</p>
</details>

<style>
  .lighting {
    flex-shrink: 0;
    max-height: 60%;
    overflow-y: auto;
    font-size: 0.8rem;
  }
  .lighting > summary {
    display: flex;
    align-items: center;
    justify-content: space-between;
    cursor: pointer;
  }
  h2 {
    margin: 0;
    font-size: 0.85rem;
    font-weight: 600;
    display: inline;
  }
  h3 {
    margin: 0;
    font-size: 0.8rem;
    font-weight: 600;
    display: inline;
  }
  .status {
    color: var(--muted);
  }
  fieldset {
    border: none;
    margin: 0;
    padding: 0;
    min-width: 0;
  }
  section {
    margin-top: 0.6rem;
  }
  .toggle {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    margin-top: 0.4rem;
  }
  .memory {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.4rem;
    margin-top: 0.4rem;
    color: var(--muted);
    font-size: 0.75rem;
  }
  .memory-actions {
    display: flex;
    gap: 0.3rem;
    flex-shrink: 0;
  }
  .memory button {
    font-size: 0.75rem;
    padding: 0.1rem 0.5rem;
  }
  .presets {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
    margin-top: 0.4rem;
  }
  .presets button {
    font-size: 0.75rem;
    padding: 0.15rem 0.5rem;
  }
  .presets button.active {
    border-color: var(--accent);
    color: var(--accent);
  }
  .light {
    display: grid;
    grid-template-columns: 72px 1fr;
    gap: 0.5rem;
    align-items: start;
    margin-top: 0.3rem;
  }
  .controls {
    min-width: 0;
  }
  .slider {
    display: grid;
    grid-template-columns: 2.5em 1fr 3.5em;
    align-items: center;
    gap: 0.3rem;
  }
  .slider input {
    width: 100%;
    min-width: 0;
    margin: 0;
  }
  .value {
    font-variant-numeric: tabular-nums;
    color: var(--muted);
    text-align: right;
  }
  .color-row {
    display: grid;
    grid-template-columns: auto 1fr;
    align-items: center;
    gap: 0.3rem;
  }
  .color input {
    width: 1.8rem;
    height: 1.4rem;
    padding: 0;
    border: 1px solid var(--border);
    background: none;
  }
  .hint {
    margin: 0.5rem 0 0;
    color: var(--muted);
    font-size: 0.7rem;
  }
</style>
