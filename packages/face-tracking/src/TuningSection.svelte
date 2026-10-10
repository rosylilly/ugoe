<script lang="ts">
// 調整 (首振りの感度と遊び・瞼と口の開きの境目。T0877、tuning.ts)。TrackingPanel の中の折りたためるまとまり。
// 各項目のスライダーと、今の値 (キャリブレーションの後の、調整の前と後) のメーターを並べ、顔を動かしながら決められるようにする。
import { Button, Section, Slider } from "@ugoe/ui";
import { t } from "./i18n";
import type { FaceTracker } from "./tracker.svelte";
import {
  DEAD_ZONE_MAX,
  DEFAULT_TUNING,
  GAIN_MAX,
  GAIN_MIN,
  isDefaultTuning,
  moveRangeBound,
  RANGE_MIN_SPAN,
  TUNING_AXES,
  type TuningAxis,
  type TuningRange,
} from "./tuning";

interface Props {
  tracker: FaceTracker;
  /** 見出しの段。 */
  level: 3 | 4 | 5;
}

let { tracker, level }: Props = $props();

/** 頭のメーターの端 (度)。 */
const ANGLE_SCALE = 45;

/** 境目のまとまりと、メーターに出すブレンドシェイプ (左右は別のメーター)。 */
const RANGES: readonly {
  key: TuningRange;
  meters: readonly [string, "left" | "right" | null][];
}[] = [
  {
    key: "eyeBlink",
    meters: [
      ["eyeBlinkLeft", "left"],
      ["eyeBlinkRight", "right"],
    ],
  },
  { key: "jawOpen", meters: [["jawOpen", null]] },
];

/** 開いているか (閉じている間は中身を作らない。メーターが毎フレーム描き直されないように)。 */
let open = $state(false);
let tuning = $derived(tracker.tuning);
let preview = $derived(tracker.tuningPreview);

function setAxis(axis: TuningAxis, patch: Partial<{ gain: number; deadZone: number }>) {
  tracker.setTuning({ ...tuning, [axis]: { ...tuning[axis], ...patch } });
}

function setBound(key: TuningRange, bound: "low" | "high", value: number) {
  tracker.setTuning({ ...tuning, [key]: moveRangeBound(tuning[key], bound, value) });
}

function degrees(v: number): string {
  return `${v.toFixed(1)}°`;
}

function score(v: number): string {
  return v.toFixed(2);
}

/** 値の位置 (0〜100%)。 */
function position(v: number, min: number, max: number): number {
  return Math.min(100, Math.max(0, ((v - min) / (max - min)) * 100));
}
</script>

<Section title={t("tuning.title")} {level} bind:open>
  {#if open}
  <p class="note" class:warn={!tracker.calibrated}>
    {tracker.calibrated ? t("tuning.calibrated") : t("tuning.uncalibrated")}
  </p>

  {#each TUNING_AXES as axis (axis)}
    {@const before = preview?.before.head[axis] ?? null}
    {@const after = preview?.after.head[axis] ?? null}
    <div class="item">
      {@render meter(
        t(`tuning.${axis}`),
        before,
        after,
        -ANGLE_SCALE,
        ANGLE_SCALE,
        [-tuning[axis].deadZone, tuning[axis].deadZone],
        degrees,
        "title",
      )}
      <Slider
        label={t("tuning.gain.label")}
        tooltip={t("tuning.gain.tooltip")}
        min={GAIN_MIN}
        max={GAIN_MAX}
        step={0.05}
        decimals={2}
        unit={t("tuning.times")}
        defaultValue={DEFAULT_TUNING[axis].gain}
        value={tuning[axis].gain}
        onchange={(v) => setAxis(axis, { gain: v })}
      />
      <Slider
        label={t("tuning.deadZone.label")}
        tooltip={t("tuning.deadZone.tooltip")}
        min={0}
        max={DEAD_ZONE_MAX}
        step={0.5}
        decimals={1}
        unit={t("tuning.degrees")}
        defaultValue={DEFAULT_TUNING[axis].deadZone}
        value={tuning[axis].deadZone}
        onchange={(v) => setAxis(axis, { deadZone: v })}
      />
    </div>
  {/each}

  {#each RANGES as { key, meters } (key)}
    {@const range = tuning[key]}
    <div class="item">
      <span class="item-title">{t(`tuning.${key}.label`)}</span>
      {#each meters as [name, side] (name)}
        {@render meter(
          side === null ? t(`tuning.${key}.label`) : t(`tuning.${side}`),
          preview?.before.blendShapes[name] ?? null,
          preview?.after.blendShapes[name] ?? null,
          0,
          1,
          [range.low, range.high],
          score,
          side === null ? "hidden" : "plain",
        )}
      {/each}
      <!-- 下の境目は 1 − 最小の幅まで、上の境目は最小の幅から (間を保つためにもう片方を押す。moveRangeBound) -->
      <Slider
        label={t(`tuning.${key}.low.label`)}
        tooltip={t(`tuning.${key}.low.tooltip`)}
        min={0}
        max={1 - RANGE_MIN_SPAN}
        step={0.01}
        decimals={2}
        defaultValue={DEFAULT_TUNING[key].low}
        value={range.low}
        onchange={(v) => setBound(key, "low", v)}
      />
      <Slider
        label={t(`tuning.${key}.high.label`)}
        tooltip={t(`tuning.${key}.high.tooltip`)}
        min={RANGE_MIN_SPAN}
        max={1}
        step={0.01}
        decimals={2}
        defaultValue={DEFAULT_TUNING[key].high}
        value={range.high}
        onchange={(v) => setBound(key, "high", v)}
      />
    </div>
  {/each}

  <div class="row">
    <Button small disabled={isDefaultTuning(tuning)} onclick={() => tracker.setTuning(DEFAULT_TUNING)}
      >{t("tuning.reset.label")}</Button
    >
  </div>
  {/if}
</Section>

<!--
  今の値のメーター。帯は調整の後の値 (中央から、または左から)、縦の線は調整の前の値、点線は遊び・境目。
  帯と線は見た目だけで (読み上げは数の文で足りる)、数の文は毎フレーム変わるので live region にしない。
  `nameStyle`: `title` は名前を項目の名前として太く出す、`hidden` は見せずに読ませる (上に項目の名前を出しているとき)。
-->
{#snippet meter(
  name: string,
  before: number | null,
  after: number | null,
  min: number,
  max: number,
  marks: number[],
  format: (v: number) => string,
  nameStyle: "title" | "plain" | "hidden",
)}
  {@const centered = min < 0}
  {@const zero = position(0, min, max)}
  <div class="meter">
    <span
      class="meter-name"
      class:item-title={nameStyle === "title"}
      class:ugoe-visually-hidden={nameStyle === "hidden"}>{name}</span
    >
    <span class="meter-track" aria-hidden="true">
      {#each marks as mark, i (i)}
        <span class="meter-mark" style:left="{position(mark, min, max)}%"></span>
      {/each}
      {#if after !== null}
        {@const p = position(after, min, max)}
        <span
          class="meter-fill"
          style:left="{centered ? Math.min(zero, p) : 0}%"
          style:width="{centered ? Math.abs(p - zero) : p}%"
        ></span>
      {/if}
      {#if before !== null}
        <span class="meter-before" style:left="{position(before, min, max)}%"></span>
      {/if}
    </span>
    <span class="meter-value"
      >{before === null || after === null
        ? "—"
        : t("tuning.meter", { before: format(before), after: format(after) })}</span
    >
  </div>
{/snippet}

<style>
  .note {
    margin: var(--ugoe-space-1) 0 0;
    font-size: var(--ugoe-font-size-sm);
    color: var(--ugoe-color-text-muted);
  }
  .note.warn {
    color: var(--ugoe-color-warning);
  }
  .item {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-1);
    margin-top: var(--ugoe-space-3);
  }
  .item-title {
    font-size: var(--ugoe-font-size-sm);
    font-weight: var(--ugoe-font-weight-strong);
  }
  .meter {
    /* 見せない名前 (ugoe-visually-hidden は absolute) を行の中に留める */
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 7rem) 1fr max-content;
    align-items: center;
    gap: var(--ugoe-space-2);
    font-size: var(--ugoe-font-size-sm);
  }
  .meter-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meter-value {
    font-variant-numeric: tabular-nums;
    color: var(--ugoe-color-text-muted);
    text-align: right;
    min-width: 7.5rem;
  }
  .meter-track {
    position: relative;
    height: 0.6rem;
    border-radius: var(--ugoe-radius-sm);
    background: var(--ugoe-color-sunken);
    overflow: hidden;
  }
  .meter-fill {
    position: absolute;
    top: 0;
    bottom: 0;
    background: var(--ugoe-color-accent);
    opacity: 0.6;
  }
  .meter-before {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 2px;
    margin-left: -1px;
    background: var(--ugoe-color-text);
  }
  .meter-mark {
    position: absolute;
    top: 0;
    bottom: 0;
    border-left: 1px dashed var(--ugoe-color-text-muted);
  }
  .row {
    display: flex;
    margin-top: var(--ugoe-space-3);
  }
</style>
