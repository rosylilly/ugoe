<!--
  プレビューモードの右の列 (docs/editor.md#プレビューモード)。段 (物理・目パチ・呼吸・リップシンク・ポーズ) の切り替え、
  フェイストラッキング (@ugoe/face-tracking、T0636)、プロジェクトのモーション・表情の再生、すべてのパラメータの
  スライダー (グループごと)。
  状態と WebPuppet の操作は PreviewMode (preview.svelte.ts)。コマンドは送らない。
-->
<script lang="ts">
import { TrackingPanel } from "@ugoe/face-tracking";
import type { EditorDocument } from "../document.svelte";
import type { Model } from "../ipc";
import { formatValue, parseValue } from "../panels/parameterValue";
import type { ViewportController } from "../viewport/controller.svelte";
import LicenseDialog from "./LicenseDialog.svelte";
import { filterSections, groupParameters, STAGES, sliderStep, stageAvailability } from "./preview";
import type { PreviewMode } from "./preview.svelte";

interface Props {
  preview: PreviewMode;
  doc: EditorDocument;
  viewport: ViewportController;
}

let { preview, doc, viewport }: Props = $props();

let filter = $state("");
let licenses = $state<LicenseDialog | null>(null);
/** 値の欄の入力の誤り (パラメータごとに 1 つ)。 */
let valueError = $state<{ id: string; message: string } | null>(null);
/**
 * 入力中の値の欄 (ID と入り始めの表示)。値は毎フレーム変わる (呼吸・物理) ので、入力中は表示を書き換えない。
 */
let typing = $state<{ id: string; text: string } | null>(null);

const model = $derived(doc.model ? (JSON.parse(doc.model.json) as Model) : null);
const sections = $derived(groupParameters(doc.parameters, model?.parameter_groups ?? [], "Other"));
const shown = $derived(filterSections(sections, filter));
const available = $derived(stageAvailability(model, doc.physics !== null));

function onValueChange(id: string, e: Event) {
  const input = e.currentTarget as HTMLInputElement;
  const p = doc.parameters.find((q) => q.id === id);
  if (!p) return;
  const parsed = parseValue(input.value, p);
  if (!parsed.ok) {
    valueError = { id, message: parsed.error };
  } else {
    valueError = null;
    preview.setParameter(id, parsed.value);
  }
  typing = null;
  input.value = formatValue(preview.values[id] ?? p.default);
}

function onValueKeyDown(e: KeyboardEvent) {
  if (e.key === "Enter") (e.currentTarget as HTMLInputElement).blur();
}
</script>

<div class="preview-panel">
  <h2>Preview</h2>
  {#if preview.error}<p class="error">{preview.error}</p>{/if}
  {#if viewport.status !== "ready"}<p class="muted">{viewport.message ?? "Loading viewport…"}</p>{/if}

  <section aria-label="Runtime stages">
    <h3>Runtime</h3>
    {#each STAGES as stage (stage.id)}
      <div class="stage">
        <label class="check" title={available[stage.id] ? "" : "This model has no settings for this stage"}>
          <input
            type="checkbox"
            checked={preview.stages[stage.id]}
            onchange={(e) => preview.setStage(stage.id, e.currentTarget.checked)}
          />
          {stage.label}
          {#if !available[stage.id]}<span class="muted">(not set up)</span>{/if}
        </label>
        {#if stage.id === "physics"}
          <button
            onclick={() => preview.resetPhysics()}
            disabled={!preview.stages.physics || !available.physics}
            title="Put the particles back to their rest pose">Reset</button
          >
        {:else if stage.id === "lip_sync"}
          <input
            class="level"
            type="range"
            min="0"
            max="1"
            step="0.01"
            value={preview.lipSyncLevel}
            disabled={!preview.stages.lip_sync}
            aria-label="Lip sync level"
            title="Mouth opening from the voice level (0 to 1)"
            oninput={(e) => preview.setLipSyncLevel(e.currentTarget.valueAsNumber)}
          />
        {/if}
      </div>
    {/each}
  </section>

  <section aria-label="Face tracking" class="tracking">
    <TrackingPanel
      tracker={preview.tracker}
      parameterIds={preview.trackingParameterIds}
      values={preview.values}
      open={false}
    />
    <button class="link" type="button" onclick={() => void licenses?.open()}
      >Third-party licenses</button
    >
  </section>
  <LicenseDialog bind:this={licenses} />

  <section aria-label="Motions">
    <div class="head">
      <h3>Motions</h3>
      <button onclick={() => preview.stopMotions()} disabled={!preview.motionPlaying}>Stop</button>
    </div>
    {#if doc.motions.length === 0}
      <p class="muted">No motions in this project.</p>
    {:else}
      <ul class="list">
        {#each doc.motions as motion (motion.name)}
          {@const current = preview.motionPlaying && preview.motion === motion.name}
          <li>
            <button class="play" class:current onclick={() => (current ? preview.stopMotions() : preview.playMotion(motion))}
              title={current ? `Stop ${motion.name}` : `Play ${motion.name}`}
              >{current ? "■" : "▶"} {motion.name}</button
            >
            <span class="muted">{formatValue(motion.duration)} s{motion.looping ? " · loop" : ""}</span>
          </li>
        {/each}
      </ul>
    {/if}
    {#if preview.events.length > 0}
      <p class="muted events">Events: {preview.events.join(", ")}</p>
    {/if}
  </section>

  <section aria-label="Expressions">
    <h3>Expression</h3>
    {#if doc.expressions.length === 0}
      <p class="muted">No expressions in this project.</p>
    {:else}
      <select
        value={preview.expression ?? ""}
        aria-label="Expression"
        onchange={(e) => preview.setExpression(e.currentTarget.value || null)}
      >
        <option value="">(none)</option>
        {#each doc.expressions as expression (expression.name)}
          <option value={expression.name}>{expression.name}</option>
        {/each}
      </select>
    {/if}
  </section>

  <section aria-label="Parameters" class="parameters">
    <div class="head">
      <h3>Parameters</h3>
      <button onclick={() => preview.resetAllParameters()} title="Set every parameter to its default"
        >Reset all</button
      >
    </div>
    <input class="filter" type="search" placeholder="Filter (name or ID)" bind:value={filter} />
    {#if doc.parameters.length === 0}
      <p class="muted">This model has no parameters.</p>
    {/if}
    {#each shown as section (section.id ?? "")}
      <details open>
        <summary>{section.name} <span class="muted">{section.parameters.length}</span></summary>
        {#each section.parameters as p (p.id)}
          {@const value = preview.values[p.id] ?? p.default}
          <div class="row" title={p.id}>
            <span class="name">{p.name || p.id}</span>
            <input
              class="value"
              type="text"
              inputmode="decimal"
              value={typing?.id === p.id ? typing.text : formatValue(value)}
              aria-label="{p.name} value"
              title="Type a value between {p.min} and {p.max}, then press Enter"
              onfocus={(e) => (typing = { id: p.id, text: e.currentTarget.value })}
              onblur={() => (typing = null)}
              onchange={(e) => onValueChange(p.id, e)}
              onkeydown={onValueKeyDown}
            />
            <button
              class="reset"
              onclick={() => preview.resetParameter(p.id)}
              disabled={Math.abs(value - p.default) < 1e-6}
              title="Back to the default ({formatValue(p.default)})"
              aria-label="Reset {p.name}">↺</button
            >
            <input
              class="slider"
              type="range"
              min={p.min}
              max={p.max}
              step={sliderStep(p)}
              {value}
              aria-label={p.name}
              oninput={(e) => preview.setParameter(p.id, e.currentTarget.valueAsNumber)}
              ondblclick={() => preview.resetParameter(p.id)}
            />
          </div>
          {#if valueError?.id === p.id}<p class="error">{valueError.message}</p>{/if}
        {/each}
      </details>
    {/each}
  </section>
</div>

<style>
.preview-panel {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding-bottom: 8px;
}

.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 4px;
}

h3 {
  margin: 4px 0;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
}

section {
  border-top: 1px solid var(--border);
  padding-top: 4px;
}

/* トラッキングのパネルは <details> で開閉する。見出しを他の節に揃える。 */
.tracking :global(details.tracking > summary h2) {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
}

/* エディタの label は縦に並べる (app.css) ので、チェックボックスと文言を横に並べ直す。 */
.tracking :global(details.tracking .options label) {
  display: flex;
  align-items: center;
  gap: 4px;
}

.link {
  margin-top: 4px;
  padding: 0;
  border: none;
  background: none;
  color: var(--muted);
  font-size: 11px;
  text-decoration: underline;
  cursor: pointer;
}

.stage {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  min-height: 26px;
}

.check {
  display: flex;
  align-items: center;
  gap: 6px;
}

.level {
  width: 120px;
}

.list {
  margin: 0;
}

.list li {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 2px;
}

.play {
  flex: 1;
  min-width: 0;
  text-align: left;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.play.current {
  color: var(--accent);
  border-color: var(--accent);
}

.events {
  margin: 4px 0 0;
  overflow-wrap: anywhere;
}

select,
.filter {
  width: 100%;
}

summary {
  cursor: pointer;
  padding: 4px 0;
  font-weight: 600;
}

.row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto auto;
  grid-template-areas:
    "name value reset"
    "slider slider slider";
  align-items: center;
  column-gap: 4px;
  padding: 2px 0 6px 8px;
}

.name {
  grid-area: name;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.value {
  grid-area: value;
  width: 5em;
  text-align: right;
}

.reset {
  grid-area: reset;
  padding: 0 6px;
}

.slider {
  grid-area: slider;
  width: 100%;
  margin: 0;
}
</style>
