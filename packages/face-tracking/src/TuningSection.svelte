<script lang="ts">
// 調整 (首振りの感度と遊び・瞼・視線・口・眉の境目。T0877・T0879・T0880、tuning.ts)。TrackingPanel の中の折りたためるまとまり。
// 「首」「目」「口」「眉」の小見出しで分け、各項目のスライダーと、今の値 (キャリブレーションの後の、調整の前と後) のメーターを並べ、
// 顔を動かしながら決められるようにする。閉じている見出しの中身は作らない (メーターが毎フレーム描き直されないように)。
import { Button, Section, Slider, Toggle } from "@ugoe/ui";
import { t } from "./i18n";
import type { FaceTracker } from "./tracker.svelte";
import {
  type AxisTuning,
  DEAD_ZONE_MAX,
  DEFAULT_TUNING,
  GAIN_MAX,
  GAIN_MIN,
  isDefaultTuning,
  moveRangeBound,
  RANGE_MIN_SPAN,
  setAxisSeparate,
  setEyeBlinkSeparate,
  TUNING_AXES,
  type TuningAxis,
  type TuningRange,
  type TuningRangeKey,
} from "./tuning";

interface Props {
  tracker: FaceTracker;
  /** 見出しの段。 */
  level: 3 | 4 | 5;
}

let { tracker, level }: Props = $props();

/** 頭のメーターの端 (度)。 */
const ANGLE_SCALE = 45;

/** メーターに出すブレンドシェイプと、その名前の文言 (`null` は項目の名前を見せずに読ませる)。 */
type Meters = readonly (readonly [string, (() => string) | null])[];

/** 境目のまとまりと、メーター (左右・向きは別のメーター)。 */
const METERS: Record<TuningRange, Meters> = {
  eyeBlink: [
    ["eyeBlinkLeft", () => t("tuning.left")],
    ["eyeBlinkRight", () => t("tuning.right")],
  ],
  eyeLook: [
    ["eyeLookInLeft", () => t("tuning.shapes.eyeLookInLeft")],
    ["eyeLookOutLeft", () => t("tuning.shapes.eyeLookOutLeft")],
    ["eyeLookUpLeft", () => t("tuning.shapes.eyeLookUpLeft")],
    ["eyeLookDownLeft", () => t("tuning.shapes.eyeLookDownLeft")],
    ["eyeLookInRight", () => t("tuning.shapes.eyeLookInRight")],
    ["eyeLookOutRight", () => t("tuning.shapes.eyeLookOutRight")],
    ["eyeLookUpRight", () => t("tuning.shapes.eyeLookUpRight")],
    ["eyeLookDownRight", () => t("tuning.shapes.eyeLookDownRight")],
  ],
  jawOpen: [["jawOpen", null]],
  mouthClose: [["mouthClose", null]],
  mouthForm: [
    ["mouthSmileLeft", () => t("tuning.shapes.mouthSmileLeft")],
    ["mouthSmileRight", () => t("tuning.shapes.mouthSmileRight")],
    ["mouthFrownLeft", () => t("tuning.shapes.mouthFrownLeft")],
    ["mouthFrownRight", () => t("tuning.shapes.mouthFrownRight")],
  ],
  brow: [
    ["browInnerUp", () => t("tuning.shapes.browInnerUp")],
    ["browOuterUpLeft", () => t("tuning.shapes.browOuterUpLeft")],
    ["browOuterUpRight", () => t("tuning.shapes.browOuterUpRight")],
    ["browDownLeft", () => t("tuning.shapes.browDownLeft")],
    ["browDownRight", () => t("tuning.shapes.browDownRight")],
  ],
};

/** 小見出しのまとまり (首は軸、ほかは境目)。 */
const GROUPS = [
  { key: "head", ranges: [] },
  { key: "eyes", ranges: ["eyeBlink", "eyeLook"] },
  { key: "mouth", ranges: ["jawOpen", "mouthClose", "mouthForm"] },
  { key: "brow", ranges: ["brow"] },
] as const satisfies readonly { key: string; ranges: readonly TuningRange[] }[];
type Group = (typeof GROUPS)[number]["key"];

/** 開いているか (閉じている間は中身を作らない。メーターが毎フレーム描き直されないように)。 */
let open = $state(false);
let groupOpen = $state<Record<Group, boolean>>({
  head: false,
  eyes: false,
  mouth: false,
  brow: false,
});
let tuning = $derived(tracker.tuning);
let preview = $derived(tracker.tuningPreview);
let groupLevel = $derived((level + 1) as 4 | 5 | 6);

function setAxis(
  axis: TuningAxis,
  patch: Partial<Pick<AxisTuning, "gain" | "deadZone" | "gainNegative">>,
) {
  tracker.setTuning({ ...tuning, [axis]: { ...tuning[axis], ...patch } });
}

function setBound(key: TuningRangeKey, bound: "low" | "high", value: number) {
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

  {#each GROUPS as group (group.key)}
    <Section title={t(`tuning.groups.${group.key}`)} level={groupLevel} bind:open={groupOpen[group.key]}>
      {#if groupOpen[group.key]}
        {#if group.key === "head"}
          {#each TUNING_AXES as axis (axis)}
            {@render axisItem(axis)}
          {/each}
        {/if}
        {#each group.ranges as key (key)}
          {#if key === "eyeBlink"}
            <div class="item">
              <Toggle
                label={t("tuning.eyeBlinkSeparate.label")}
                tooltip={t("tuning.eyeBlinkSeparate.tooltip")}
                checked={tuning.eyeBlinkSeparate}
                onchange={(v) => tracker.setTuning(setEyeBlinkSeparate(tuning, v))}
              />
            </div>
            {#if tuning.eyeBlinkSeparate}
              {@render rangeItem("eyeBlink", "eyeBlink", t("tuning.eyeBlink.leftLabel"), [
                ["eyeBlinkLeft", null],
              ])}
              {@render rangeItem("eyeBlinkRight", "eyeBlink", t("tuning.eyeBlink.rightLabel"), [
                ["eyeBlinkRight", null],
              ])}
            {:else}
              {@render rangeItem("eyeBlink", "eyeBlink", t("tuning.eyeBlink.label"), METERS.eyeBlink)}
            {/if}
          {:else}
            {@render rangeItem(key, key, t(`tuning.${key}.label`), METERS[key])}
          {/if}
        {/each}
      {/if}
    </Section>
  {/each}

  <div class="row">
    <Button small disabled={isDefaultTuning(tuning)} onclick={() => tracker.setTuning(DEFAULT_TUNING)}
      >{t("tuning.reset.label")}</Button
    >
  </div>
  {/if}
</Section>

<!--
  首の 1 つの軸 (感度と遊び)。感度を向きで分けているときは、正の向き (`gain`) と負の向き (`gainNegative`) の 2 つの感度を出し、
  メーターの両端に向きを書く (メーターは負の向きが左、正の向きが右)。
-->
{#snippet axisItem(axis: TuningAxis)}
  {@const separate = tuning[axis].separate}
  <div class="item">
    {@render meter(
      t(`tuning.${axis}`),
      preview?.before.head[axis] ?? null,
      preview?.after.head[axis] ?? null,
      -ANGLE_SCALE,
      ANGLE_SCALE,
      [-tuning[axis].deadZone, tuning[axis].deadZone],
      degrees,
      "title",
      separate
        ? [t(`tuning.directions.${axis}.negative`), t(`tuning.directions.${axis}.positive`)]
        : null,
    )}
    <Toggle
      label={t("tuning.gainSeparate.label")}
      tooltip={t("tuning.gainSeparate.tooltip")}
      checked={separate}
      onchange={(v) => tracker.setTuning(setAxisSeparate(tuning, axis, v))}
    />
    {#if separate}
      {@render gainSlider(
        t("tuning.gainToward.label", { direction: t(`tuning.directions.${axis}.positive`) }),
        t("tuning.gainToward.tooltip"),
        tuning[axis].gain,
        (v) => setAxis(axis, { gain: v }),
      )}
      {@render gainSlider(
        t("tuning.gainToward.label", { direction: t(`tuning.directions.${axis}.negative`) }),
        t("tuning.gainToward.tooltip"),
        tuning[axis].gainNegative,
        (v) => setAxis(axis, { gainNegative: v }),
      )}
    {:else}
      {@render gainSlider(t("tuning.gain.label"), t("tuning.gain.tooltip"), tuning[axis].gain, (v) =>
        setAxis(axis, { gain: v }),
      )}
    {/if}
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
{/snippet}

<!-- 感度のスライダー (既定は 1 倍)。 -->
{#snippet gainSlider(label: string, tooltip: string, value: number, onchange: (v: number) => void)}
  <Slider
    {label}
    {tooltip}
    min={GAIN_MIN}
    max={GAIN_MAX}
    step={0.05}
    decimals={2}
    unit={t("tuning.times")}
    defaultValue={DEFAULT_TUNING.yaw.gain}
    {value}
    {onchange}
  />
{/snippet}

<!--
  境目の 1 つの組。`key` は値の項目、`text` はスライダーの文言の項目 (右目の瞼は瞼の文言を使う)。
  メーターの名前が `null` なら、項目の名前を見せずに読ませる (上に項目の名前を出している)。
-->
{#snippet rangeItem(key: TuningRangeKey, text: TuningRange, title: string, meters: Meters)}
  {@const range = tuning[key]}
  <div class="item">
    <span class="item-title">{title}</span>
    {#each meters as [name, label] (name)}
      {@render meter(
        label === null ? title : label(),
        preview?.before.blendShapes[name] ?? null,
        preview?.after.blendShapes[name] ?? null,
        0,
        1,
        [range.low, range.high],
        score,
        label === null ? "hidden" : "plain",
      )}
    {/each}
    <!-- 下の境目は 1 − 最小の幅まで、上の境目は最小の幅から (間を保つためにもう片方を押す。moveRangeBound) -->
    <Slider
      label={t(`tuning.${text}.low.label`)}
      tooltip={t(`tuning.${text}.low.tooltip`)}
      min={0}
      max={1 - RANGE_MIN_SPAN}
      step={0.01}
      decimals={2}
      defaultValue={DEFAULT_TUNING[key].low}
      value={range.low}
      onchange={(v) => setBound(key, "low", v)}
    />
    <Slider
      label={t(`tuning.${text}.high.label`)}
      tooltip={t(`tuning.${text}.high.tooltip`)}
      min={RANGE_MIN_SPAN}
      max={1}
      step={0.01}
      decimals={2}
      defaultValue={DEFAULT_TUNING[key].high}
      value={range.high}
      onchange={(v) => setBound(key, "high", v)}
    />
  </div>
{/snippet}

<!--
  今の値のメーター。帯は調整の後の値 (中央から、または左から)、縦の線は調整の前の値、点線は遊び・境目。
  帯と線は見た目だけで (読み上げは数の文で足りる)、数の文は毎フレーム変わるので live region にしない。
  `nameStyle`: `title` は名前を項目の名前として太く出す、`hidden` は見せずに読ませる (上に項目の名前を出しているとき)。
  `ends`: 中央からのメーターの左端と右端の向きの名前 (感度を向きで分けているとき。スライダーの名前で読めるので読み上げない)。
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
  ends: [string, string] | null = null,
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
    {#if ends}
      <span class="meter-ends" aria-hidden="true"><span>{ends[0]}</span><span>{ends[1]}</span></span>
    {/if}
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
  .meter-ends {
    grid-column: 2;
    display: flex;
    justify-content: space-between;
    margin-top: calc(-1 * var(--ugoe-space-1));
    font-size: var(--ugoe-font-size-xs);
    color: var(--ugoe-color-text-muted);
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
