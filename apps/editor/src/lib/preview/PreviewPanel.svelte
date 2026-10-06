<!--
  プレビューモードの右の列 (docs/editor.md#プレビューモード)。段 (物理・目パチ・呼吸・リップシンク・ポーズ) の切り替え、
  ポーズの表示する要素の切り替え (T0660。着せ替えで付けたモジュールのグループも。T0661。欄は @ugoe/player の PosePicker で、
  見出しと本体の要素の表示名 (パーツの名前) だけを差し替える。T1640)、着せ替え (DressUpSection、T0649)、フェイストラッキング (@ugoe/face-tracking、T0636)、プロジェクトのモーション・表情の再生、すべてのパラメータの
  スライダー (グループごと)。
  状態と WebPuppet の操作は PreviewMode (preview.svelte.ts)。コマンドは送らない。
-->
<script lang="ts">
import { TrackingPanel } from "@ugoe/face-tracking";
import { PosePicker, poseEntryLabel } from "@ugoe/player";
import { Button, Icon, Notice, Section, Select, Slider, TextField, Toggle } from "@ugoe/ui";
import type { EditorDocument } from "../document.svelte";
import { t } from "../i18n";
import type { DifferenceItem } from "../ipc";
import LicenseDialog from "../LicenseDialog.svelte";
import { labelOf } from "../labels";
import type { LightingView } from "../panels/lighting/lightingView.svelte";
import PreviewLightsSection from "../panels/lighting/PreviewLightsSection.svelte";
import { formatValue } from "../panels/parameterValue";
import TipButton from "../ui/TipButton.svelte";
import type { ViewportController } from "../viewport/controller.svelte";
import DressUpSection from "./DressUpSection.svelte";
import {
  filterSections,
  groupParameters,
  previewPoseGroupLabel,
  previewPoseGroups,
  STAGES,
  sliderStep,
  stageAvailability,
  stageLabel,
} from "./preview";
import type { PreviewMode } from "./preview.svelte";

interface Props {
  preview: PreviewMode;
  doc: EditorDocument;
  viewport: ViewportController;
  /** プレビューの光源 (ライティングのパネルと共有。docs/editor.md#ライティングの画面)。 */
  lighting: LightingView;
  /** 着せ替えの違いの行の要素を選ぶ (プレビューを出てツリーか Physics タブで選ぶ。T0656・T0657)。 */
  onselect?: (item: DifferenceItem) => void;
}

let { preview, doc, viewport, lighting, onselect }: Props = $props();

let filter = $state("");
let licenses = $state<LicenseDialog | null>(null);

/** 表情を付けない選択肢の値 (表情の名前は空にならないが、念のため Select の「未選択」の空文字列と分ける)。 */
const NO_EXPRESSION = "#none";

const model = $derived(doc.parsedModel);
const sections = $derived(
  groupParameters(doc.parameters, model?.parameter_groups ?? [], t("preview.parameters.other")),
);
const shown = $derived(filterSections(sections, filter));
const available = $derived.by(() => {
  const found = stageAvailability(model, doc.physics !== null);
  // 着せ替えで付けたモジュールのポーズのグループもポーズの段で動く (T0661)。
  return preview.modulePoses.length > 0 ? { ...found, pose: true } : found;
});
const poseGroups = $derived(model?.pose.groups ?? []);
const posePicker = $derived(
  previewPoseGroups(poseGroups, preview.modulePoses, preview.poseShown, preview.modulePoseShown),
);

/** 欄のグループ (`posePicker.groups` での位置) で要素を選んだ: 本体のグループの後にモジュールのグループが並ぶ。 */
function choosePose(part: string, index: number) {
  const own = poseGroups[index];
  if (own !== undefined) preview.showPose(own, part);
  else {
    const found = preview.modulePoses[index - poseGroups.length];
    if (found !== undefined) preview.showModulePose(found.group, part);
  }
}

const expressionOptions = $derived([
  { value: NO_EXPRESSION, label: t("preview.expressions.none") },
  ...doc.expressions.map((e) => ({ value: e.name, label: e.name })),
]);
</script>

<div class="preview-panel">
  <h2>{t("preview.title")}</h2>
  {#if preview.error}<p class="error ugoe-selectable">{preview.error}</p>{/if}
  {#each preview.warnings as warning, i (i)}<Notice tone="warning">{warning}</Notice>{/each}
  {#if viewport.status !== "ready"}<p class="ed-hint">{viewport.message ?? t("preview.loadingViewport")}</p>{/if}

  <section aria-label={t("preview.stages.aria")}>
    <h3>{t("preview.stages.title")}</h3>
    {#each STAGES as stage (stage)}
      <div class="stage">
        <Toggle
          label={available[stage] ? stageLabel(stage) : t("preview.stages.notSetUp", { stage: stageLabel(stage) })}
          checked={preview.stages[stage]}
          onchange={(on) => preview.setStage(stage, on)}
        />
        {#if stage === "physics"}
          <TipButton
            small
            onclick={() => preview.resetPhysics()}
            disabled={!preview.stages.physics || !available.physics}
            tooltip={t("preview.stages.resetPhysics.tooltip")}>{t("preview.stages.resetPhysics.label")}</TipButton
          >
        {/if}
      </div>
      {#if stage === "lip_sync"}
        <div class="level">
          <Slider
            label={t("preview.stages.lipSyncLevel")}
            min={0}
            max={1}
            step={0.01}
            field={false}
            value={preview.lipSyncLevel}
            disabled={!preview.stages.lip_sync}
            onchange={(v) => preview.setLipSyncLevel(v)}
          />
        </div>
      {/if}
    {/each}
    {#if STAGES.some((s) => !available[s])}<p class="ed-hint">{t("preview.stages.notSetUpHint")}</p>{/if}
  </section>

  {#if poseGroups.length > 0 || preview.modulePoses.length > 0}
    <!-- 編集中のモデルのポーズのグループ (T0660) と、着せ替えで付けたモジュールのグループ (T0661) -->
    <section aria-label={t("preview.pose.title")}>
      <h3>{t("preview.pose.title")}</h3>
      {#if !preview.stages.pose}
        <p class="ed-hint">{t("preview.pose.off")}</p>
      {:else}
        <p class="ed-hint">{t("preview.pose.hint")}</p>
      {/if}
      <PosePicker
        groups={posePicker.groups}
        shown={posePicker.shown}
        disabled={!preview.stages.pose}
        groupLabel={previewPoseGroupLabel}
        entryLabel={(group, part) => (group.module === null ? labelOf(model?.parts, part) : poseEntryLabel(group, part))}
        onchange={choosePose}
      />
    </section>
  {/if}

  <!-- モジュールのプロジェクトでは、今の編集を付けて比べている間だけ自分のスロットも出す (T0650・T0651。
       比べられないときは、編集中のモデルで自分のスロットはもう埋まっている) -->
  <DressUpSection
    dressUp={preview.dressUp}
    slots={(model?.slots ?? []).filter((s) => preview.comparing || s.id !== doc.module?.slot.id)}
    published={model?.avatar ? model.avatar.slots : null}
    {onselect}
  />

  <section aria-label={t("preview.tracking.aria")} class="tracking">
    <TrackingPanel
      tracker={preview.tracker}
      parameterIds={preview.trackingParameterIds}
      values={preview.values}
      open={false}
      level={3}
    />
    <button class="ed-link licenses" type="button" onclick={() => void licenses?.open()}
      >{t("preview.licenses.open")}</button
    >
  </section>
  <LicenseDialog bind:this={licenses} />

  {#if viewport.usesLighting}
    <PreviewLightsSection view={lighting} canvas={model?.meta.canvas ?? { width: 1024, height: 1024 }} />
  {/if}

  <section aria-label={t("preview.motions.title")}>
    <div class="head">
      <h3>{t("preview.motions.title")}</h3>
      <Button small onclick={() => preview.stopMotions()} disabled={!preview.motionPlaying}
        >{t("preview.motions.stop")}</Button
      >
    </div>
    {#if doc.motions.length === 0}
      <p class="ed-hint">{t("preview.motions.empty")}</p>
    {:else}
      <ul class="list">
        {#each doc.motions as motion (motion.name)}
          {@const current = preview.motionPlaying && preview.motion === motion.name}
          <li>
            <Button
              small
              class="play"
              pressed={current}
              aria-label={current
                ? t("preview.motions.stopOne", { name: motion.name })
                : t("preview.motions.play", { name: motion.name })}
              onclick={() => (current ? preview.stopMotions() : preview.playMotion(motion))}
              ><Icon name={current ? "stop" : "play"} /> {motion.name}</Button
            >
            <span class="muted"
              >{motion.looping
                ? t("preview.motions.durationLoop", { value: formatValue(motion.duration) })
                : t("preview.motions.duration", { value: formatValue(motion.duration) })}</span
            >
          </li>
        {/each}
      </ul>
    {/if}
    {#if preview.events.length > 0}
      <p class="ed-hint events">{t("preview.motions.events", { events: preview.events.join(", ") })}</p>
    {/if}
  </section>

  <section aria-label={t("preview.expressions.title")}>
    <h3>{t("preview.expressions.title")}</h3>
    {#if doc.expressions.length === 0}
      <p class="ed-hint">{t("preview.expressions.empty")}</p>
    {:else}
      <Select
        aria-label={t("preview.expressions.title")}
        value={preview.expression ?? NO_EXPRESSION}
        options={expressionOptions}
        onchange={(v) => preview.setExpression(v === NO_EXPRESSION ? null : v)}
      />
    {/if}
  </section>

  <section aria-label={t("preview.parameters.title")} class="parameters">
    <div class="head">
      <h3>{t("preview.parameters.title")}</h3>
      <TipButton small onclick={() => preview.resetAllParameters()} tooltip={t("preview.parameters.resetAll.tooltip")}
        >{t("preview.parameters.resetAll.label")}</TipButton
      >
    </div>
    <div class="filter">
      <TextField
        type="search"
        placeholder={t("preview.parameters.filter.placeholder")}
        aria-label={t("preview.parameters.filter.aria")}
        bind:value={filter}
      />
    </div>
    {#if doc.parameters.length === 0}
      <p class="ed-hint">{t("preview.parameters.empty")}</p>
    {/if}
    {#each shown as section (section.id ?? "")}
      <Section title={section.name} level={4}>
        {#snippet actions()}<span class="muted">{section.parameters.length}</span>{/snippet}
        {#each section.parameters as p (p.id)}
          <!-- 値は毎フレーム変わる (呼吸・物理) が、Slider は利用者の操作のときだけ onchange を呼ぶ (T0910)。 -->
          <div class="row">
            <Slider
              label={p.name || p.id}
              min={p.min}
              max={p.max}
              step={sliderStep(p)}
              value={preview.values[p.id] ?? p.default}
              defaultValue={p.default}
              resetLabel={t("preview.parameters.reset.aria", {
                name: p.name || p.id,
                value: formatValue(p.default),
              })}
              onchange={(v) => preview.setParameter(p.id, v)}
            />
          </div>
        {/each}
      </Section>
    {/each}
  </section>
</div>

<style>
.preview-panel {
  display: flex;
  flex-direction: column;
  gap: var(--ugoe-space-3);
  padding-bottom: var(--ugoe-space-3);
}

.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ugoe-space-2);
}

h3 {
  margin: var(--ugoe-space-2) 0;
}

section {
  border-top: 1px solid var(--ugoe-color-border);
  padding-top: var(--ugoe-space-2);
}

.error {
  margin: 0;
  color: var(--ugoe-color-danger);
  overflow-wrap: anywhere;
}

.muted {
  color: var(--ugoe-color-text-muted);
}

/* ライセンスの口は控えめな文字の色 (形は app.css の .ed-link) */
.licenses {
  margin-top: var(--ugoe-space-2);
  color: var(--ugoe-color-text-muted);
  font-size: var(--ugoe-font-size-sm);
}

.stage {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ugoe-space-3);
  min-height: 26px;
}

.level {
  padding-left: var(--ugoe-space-5);
}

.list {
  margin: 0;
}

.list li {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ugoe-space-3);
  margin-bottom: var(--ugoe-space-1);
}

.list li :global(.play) {
  flex: 1;
  min-width: 0;
  justify-content: flex-start;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.list li .muted {
  white-space: nowrap;
}

.events {
  margin: var(--ugoe-space-2) 0 0;
  overflow-wrap: anywhere;
}

.filter {
  margin-bottom: var(--ugoe-space-2);
}

.row {
  padding: var(--ugoe-space-1) 0 var(--ugoe-space-2) var(--ugoe-space-3);
}
</style>
