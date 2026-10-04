<!--
  プレビューモードの右の列 (docs/editor.md#プレビューモード)。段 (物理・目パチ・呼吸・リップシンク・ポーズ) の切り替え、
  ポーズの表示する要素の切り替え (T0660。着せ替えで付けたモジュールのグループも。T0661)、着せ替え (DressUpSection、T0649)、フェイストラッキング (@ugoe/face-tracking、T0636)、プロジェクトのモーション・表情の再生、すべてのパラメータの
  スライダー (グループごと)。
  状態と WebPuppet の操作は PreviewMode (preview.svelte.ts)。コマンドは送らない。
-->
<script lang="ts">
import { TrackingPanel } from "@ugoe/face-tracking";
import { Button, Notice, Section, Select, Slider, Toggle } from "@ugoe/ui";
import type { EditorDocument } from "../document.svelte";
import { t } from "../i18n";
import type { DifferenceItem } from "../ipc";
import { labelOf } from "../labels";
import type { LightingView } from "../panels/lighting/lightingView.svelte";
import PreviewLightsSection from "../panels/lighting/PreviewLightsSection.svelte";
import { formatValue } from "../panels/parameterValue";
import { shownEntry } from "../panels/poses/poses";
import TipButton from "../ui/TipButton.svelte";
import type { ViewportController } from "../viewport/controller.svelte";
import DressUpSection from "./DressUpSection.svelte";
import LicenseDialog from "./LicenseDialog.svelte";
import {
  filterSections,
  groupParameters,
  modulePoseLabels,
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
const moduleLabels = $derived(modulePoseLabels(preview.modulePoses));
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
      {#each poseGroups as group, gi (gi)}
        {@const current = group.entries[shownEntry(group, preview.poseShown)]?.part ?? ""}
        <div class="pose">
          <Select
            label={t("preview.pose.group", { n: gi + 1 })}
            value={current}
            options={group.entries.map((entry) => ({ value: entry.part, label: labelOf(model?.parts, entry.part) }))}
            disabled={!preview.stages.pose}
            onchange={(v) => preview.showPose(group, v)}
          />
        </div>
      {/each}
      {#each preview.modulePoses as { module, group }, mi (mi)}
        {@const current = group.entries[shownEntry(group, preview.modulePoseShown)]?.part ?? ""}
        {@const label = moduleLabels[mi] ?? t("preview.pose.moduleGroupUnnamed", { n: mi + 1 })}
        <div class="pose">
          <Select
            {label}
            value={current}
            options={group.entries.map((entry) => ({
              value: entry.part,
              label: entry.part.startsWith(`${module}/`) ? entry.part.slice(module.length + 1) : entry.part,
            }))}
            disabled={!preview.stages.pose}
            onchange={(v) => preview.showModulePose(group, v)}
          />
        </div>
      {/each}
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
    />
    <button class="link" type="button" onclick={() => void licenses?.open()}
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
              >{current ? "■" : "▶"} {motion.name}</Button
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
        ariaLabel={t("preview.expressions.title")}
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
    <input
      class="filter"
      type="search"
      placeholder={t("preview.parameters.filter.placeholder")}
      aria-label={t("preview.parameters.filter.aria")}
      bind:value={filter}
    />
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

/* トラッキングのパネルは <details> で開閉する。見出しを他の節に揃える。 */
.tracking :global(details.tracking > summary h2) {
  margin: var(--ugoe-space-2) 0;
  font-size: var(--ugoe-font-size-md);
  font-weight: var(--ugoe-font-weight-strong);
  color: var(--ugoe-color-text);
}

/* エディタの label は縦に並べる (app.css) ので、チェックボックスと文言を横に並べ直す。 */
.tracking :global(details.tracking .options label) {
  display: flex;
  align-items: center;
  gap: var(--ugoe-space-2);
}

.link {
  margin-top: var(--ugoe-space-2);
  padding: 0;
  border: none;
  background: none;
  color: var(--ugoe-color-text-muted);
  font: inherit;
  font-size: var(--ugoe-font-size-sm);
  text-decoration: underline;
  cursor: pointer;
}

.link:focus-visible {
  outline: var(--ugoe-focus-ring);
  outline-offset: 1px;
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

.pose {
  margin: var(--ugoe-space-1) 0;
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
  width: 100%;
  min-height: var(--ugoe-control-height);
  margin-bottom: var(--ugoe-space-2);
  padding: 0 var(--ugoe-space-2);
  border: 1px solid var(--ugoe-color-control-border);
  border-radius: var(--ugoe-radius-md);
  background: var(--ugoe-color-surface);
  color: var(--ugoe-color-text);
  font: inherit;
}

.filter:focus-visible {
  outline: var(--ugoe-focus-ring);
  outline-offset: 1px;
}

.row {
  padding: var(--ugoe-space-1) 0 var(--ugoe-space-2) var(--ugoe-space-3);
}
</style>
