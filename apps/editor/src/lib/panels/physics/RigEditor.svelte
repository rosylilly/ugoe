<!--
  物理のリグ 1 つの編集 (docs/editor.md#物理の画面での操作): 名前・重力の基準・衝突する衝突体・粒子の鎖と係数・
  入力・出力・正規化の範囲。変更は入力欄の確定 (Enter・フォーカスを外す) ごとに 1 つのコマンドを送る。
  入力・出力・粒子は ID を持たないので、並びごと置き換える (set_physics_inputs など)。並びは送る直前のリグから
  組み立てる (sendRig。前の編集を読み直す前の写しから組み立てると、その編集を戻してしまう)。
-->
<script lang="ts">
import { Button, Checkbox, Icon, IconButton, NumberField, Select, TextField } from "@ugoe/ui";
import { t } from "../../i18n";
import type {
  Command,
  Model,
  NormalizationRange,
  PhysicsBlend,
  PhysicsInput,
  PhysicsOutput,
  PhysicsParticle,
  PhysicsRig,
  PhysicsSettings,
  PhysicsSource,
} from "../../ipc";
import { elementLabel } from "../../labels";
import {
  appendParticle,
  BLENDS,
  finiteValue,
  hasBlend,
  makeTarget,
  newOutput,
  normalizationBounds,
  outputBlend,
  PARTICLE_BOUNDS,
  type ParticleField,
  removeParticle,
  rigOf,
  type SendPhysics,
  setParticleField,
  TARGET_KINDS,
  type TargetKind,
  targetCandidates,
  targetKind,
  targetRef,
} from "./physics";

interface Props {
  rig: PhysicsRig;
  model: Model;
  settings: PhysicsSettings | null;
  send: SendPhysics;
  busy: boolean;
}

let { rig, model, settings, send, busy }: Props = $props();

const SOURCES: PhysicsSource[] = ["x", "y", "angle"];

/** 粒子の欄 (名前と説明は読むたびに t() を通す)。 */
const PARTICLE_FIELDS: {
  field: ParticleField;
  label: () => string;
  title: () => string;
  step: number;
}[] = [
  {
    field: "radius",
    label: () => t("physics.particles.fields.radius.label"),
    title: () => t("physics.particles.fields.radius.tooltip"),
    step: 1,
  },
  {
    field: "mobility",
    label: () => t("physics.particles.fields.mobility.label"),
    title: () => t("physics.particles.fields.mobility.tooltip"),
    step: 0.05,
  },
  {
    field: "delay",
    label: () => t("physics.particles.fields.delay.label"),
    title: () => t("physics.particles.fields.delay.tooltip"),
    step: 0.05,
  },
  {
    field: "acceleration",
    label: () => t("physics.particles.fields.acceleration.label"),
    title: () => t("physics.particles.fields.acceleration.tooltip"),
    step: 0.1,
  },
  {
    field: "collision_radius",
    label: () => t("physics.particles.fields.collisionRadius.label"),
    title: () => t("physics.particles.fields.collisionRadius.tooltip"),
    step: 1,
  },
  {
    field: "x",
    label: () => t("physics.particles.fields.x.label"),
    title: () => t("physics.particles.fields.x.tooltip"),
    step: 1,
  },
  {
    field: "y",
    label: () => t("physics.particles.fields.y.label"),
    title: () => t("physics.particles.fields.y.tooltip"),
    step: 1,
  },
];

const deformers = $derived(model.deformers);

/** 送る直前のこのリグからコマンドを組み立てて送る (リグが消えていれば送らない)。 */
function sendRig(build: (current: PhysicsRig) => Command | null): void {
  const id = rig.id;
  void send((settings) => {
    const current = rigOf(settings, id);
    return current ? build(current) : null;
  });
}

function rename(name: string): void {
  sendRig((r) => (name !== r.name ? { op: "rename_physics_rig", id: r.id, name } : null));
}

function setFrame(frame: string): void {
  sendRig((r) => ({
    op: "set_physics_frame",
    rig: r.id,
    ...(frame ? { frame } : {}),
  }));
}

function setColliders(id: string, on: boolean): void {
  sendRig((r) => {
    const rest = r.colliders.filter((c) => c !== id);
    return { op: "set_physics_rig_colliders", rig: r.id, colliders: on ? [...rest, id] : rest };
  });
}

function updateParticles(change: (particles: PhysicsParticle[]) => PhysicsParticle[]): void {
  sendRig((r) => ({ op: "set_physics_particles", rig: r.id, particles: change(r.particles) }));
}

function setParticle(index: number, field: ParticleField, raw: number): void {
  const value = finiteValue(raw);
  if (value !== null) updateParticles((ps) => setParticleField(ps, index, field, value));
}

function particleValue(p: PhysicsParticle, field: ParticleField): number {
  if (field === "x") return p.position[0];
  if (field === "y") return p.position[1];
  return p[field];
}

function updateInputs(change: (inputs: PhysicsInput[]) => PhysicsInput[]): void {
  sendRig((r) => ({ op: "set_physics_inputs", rig: r.id, inputs: change(r.inputs) }));
}

function patchInput(index: number, patch: Partial<PhysicsInput>): void {
  updateInputs((inputs) =>
    inputs.map((input, i) => (i === index ? { ...input, ...patch } : input)),
  );
}

function addInput(): void {
  const parameter = model.parameters[0];
  if (!parameter) return;
  updateInputs((inputs) => [
    ...inputs,
    { parameter: parameter.id, source: "x", weight: 100, reflect: false },
  ]);
}

function updateOutputs(change: (outputs: PhysicsOutput[]) => PhysicsOutput[]): void {
  sendRig((r) => ({ op: "set_physics_outputs", rig: r.id, outputs: change(r.outputs) }));
}

/** `index` 番目の出力を変える (`patch` が関数なら、送る直前の出力から作る)。 */
function patchOutput(
  index: number,
  patch: Partial<PhysicsOutput> | ((output: PhysicsOutput) => Partial<PhysicsOutput>),
): void {
  updateOutputs((outputs) =>
    outputs.map((o, i) =>
      i === index ? { ...o, ...(typeof patch === "function" ? patch(o) : patch) } : o,
    ),
  );
}

function setOutputKind(index: number, kind: TargetKind): void {
  const first = targetCandidates(model, kind)[0];
  if (first) patchOutput(index, { target: makeTarget(model, kind, first.id) });
}

function setOutputRef(index: number, id: string): void {
  patchOutput(index, (output) => ({
    target: makeTarget(model, targetKind(output.target), id, output.target),
  }));
}

function addOutput(): void {
  const output = newOutput(model, "deformer_angle", 1) ?? newOutput(model, "parameter", 1);
  if (output) updateOutputs((outputs) => [...outputs, output]);
}

function setNormalization(
  which: "position" | "angle",
  field: keyof NormalizationRange,
  raw: number,
): void {
  const value = finiteValue(raw);
  if (value === null) return;
  sendRig((r) => ({
    op: "set_physics_normalization",
    rig: r.id,
    normalization: {
      ...r.normalization,
      [which]: { ...r.normalization[which], [field]: value },
    },
  }));
}

const ids = $props.id();

/** 重力の基準の選択肢の「キャンバス」の値。デフォーマの ID は空にならない。 */
const CANVAS = "\u0000canvas";
const frameOptions = $derived([
  { value: CANVAS, label: t("physics.rig.frame.canvas") },
  ...deformers.map((d) => ({ value: d.id, label: elementLabel(d.name, d.id) })),
]);
const parameterOptions = $derived(
  model.parameters.map((p) => ({ value: p.id, label: elementLabel(p.name, p.id) })),
);
// i18n-ignore: 成分の名前は値そのもの (x・y・angle) を出す
const sourceOptions = SOURCES.map((s) => ({ value: s, label: s }));
const kindOptions = $derived(
  TARGET_KINDS.map((k) => ({
    value: k.kind,
    label: k.label,
    disabled: targetCandidates(model, k.kind).length === 0,
  })),
);
const blendOptions = $derived(BLENDS.map((b) => ({ value: b.blend, label: b.label })));
const rangeFields = $derived([
  { field: "min" as const, label: t("physics.range.min") },
  { field: "default" as const, label: t("physics.range.default") },
  { field: "max" as const, label: t("physics.range.max") },
]);
</script>

<section>
  <h3>{t("physics.rig.title")}</h3>
  <div class="ed-field">
    <label for="{ids}-name">{t("physics.rig.name.label")}</label>
    <span class="pair">
      <TextField id="{ids}-name" value={rig.name} onchange={rename} />
      <span class="muted id">{rig.id}</span>
    </span>
  </div>
  <div class="ed-field">
    <span id="{ids}-frame">{t("physics.rig.frame.label")}</span>
    <Select aria-labelledby="{ids}-frame" value={rig.frame ?? CANVAS} options={frameOptions}
      tooltip={t("physics.rig.frame.tooltip")} onchange={(v) => setFrame(v === CANVAS ? "" : v)} />
  </div>
  <div class="ed-buttons row">
    <Button disabled={busy} onclick={() => sendRig((r) => ({ op: "remove_physics_rig", id: r.id }))}>
      {t("physics.rig.delete")}
    </Button>
  </div>
</section>

<section>
  <h3>{t("physics.particles.title")}</h3>
  <p class="ed-hint">{t("physics.particles.hint")}</p>
  {#each rig.particles as p, i (i)}
    <div class="card">
      <span class="index" id="{ids}-p{i}" aria-label={t("physics.particles.aria", { index: i })}>#{i}</span>
      {#each PARTICLE_FIELDS as f (f.field)}
        {#if i > 0 || f.field === "x" || f.field === "y"}
          <span class="labelled" title={f.title()}>
            <span id="{ids}-p{i}-{f.field}">{f.label()}</span>
            <NumberField value={particleValue(p, f.field)} step={f.step} decimals={4} size={4}
              {...PARTICLE_BOUNDS[f.field]}
              aria-labelledby="{ids}-p{i} {ids}-p{i}-{f.field}"
              onchange={(v) => setParticle(i, f.field, v)} />
          </span>
        {/if}
      {/each}
      {#if i > 0}
        <span class="remove">
          <IconButton small disabled={busy} label={t("physics.particles.remove", { index: i })}
            onclick={() => updateParticles((ps) => removeParticle(ps, i))}><Icon name="close" /></IconButton
          >
        </span>
      {/if}
    </div>
  {/each}
  <div class="ed-buttons row">
    <Button disabled={busy} onclick={() => updateParticles(appendParticle)}
      >{t("physics.particles.add")}</Button
    >
  </div>
</section>

<section>
  <h3>{t("physics.inputs.title")}</h3>
  <span hidden id="{ids}-in-parameter">{t("physics.inputs.parameter")}</span>
  <span hidden id="{ids}-in-source">{t("physics.inputs.source")}</span>
  <span hidden id="{ids}-in-weight">{t("physics.inputs.weight.label")}</span>
  {#each rig.inputs as input, i (i)}
    <div class="card">
      <span hidden id="{ids}-in{i}">{t("physics.inputs.aria", { index: i })}</span>
      <Select aria-labelledby="{ids}-in{i} {ids}-in-parameter" value={input.parameter} options={parameterOptions}
        onchange={(v) => patchInput(i, { parameter: v })} />
      <Select aria-labelledby="{ids}-in{i} {ids}-in-source" value={input.source} options={sourceOptions}
        onchange={(v) => patchInput(i, { source: v })} />
      <span title={t("physics.inputs.weight.tooltip")}>
        <NumberField value={input.weight} step={5} decimals={4} size={4} unit="%"
          aria-labelledby="{ids}-in{i} {ids}-in-weight"
          onchange={(v) => {
            const w = finiteValue(v);
            if (w !== null) patchInput(i, { weight: w });
          }} />
      </span>
      <Checkbox checked={input.reflect} label={t("physics.inputs.flip")}
        onchange={(on) => patchInput(i, { reflect: on })} />
      <span class="remove">
        <IconButton small disabled={busy} label={t("physics.inputs.remove", { index: i })}
          onclick={() => updateInputs((inputs) => inputs.filter((_, k) => k !== i))}><Icon name="close" /></IconButton
        >
      </span>
    </div>
  {:else}
    <p class="ed-hint">{t("physics.inputs.empty")}</p>
  {/each}
  <div class="ed-buttons row">
    <Button disabled={busy || model.parameters.length === 0} onclick={addInput}>{t("physics.inputs.add")}</Button>
  </div>
</section>

<section>
  <h3>{t("physics.outputs.title")}</h3>
  <span hidden id="{ids}-out-kind">{t("physics.outputs.kind")}</span>
  <span hidden id="{ids}-out-target">{t("physics.outputs.target")}</span>
  <span hidden id="{ids}-out-particle">{t("physics.outputs.particle.aria")}</span>
  <span hidden id="{ids}-out-source">{t("physics.outputs.source")}</span>
  <span hidden id="{ids}-out-scale">{t("physics.outputs.scale.label")}</span>
  <span hidden id="{ids}-out-weight">{t("physics.outputs.weight.label")}</span>
  <span hidden id="{ids}-out-blend">{t("physics.outputs.blend.label")}</span>
  {#each rig.outputs as output, i (i)}
    {@const kind = targetKind(output.target)}
    <div class="card">
      <span hidden id="{ids}-out{i}">{t("physics.outputs.aria", { index: i })}</span>
      <Select aria-labelledby="{ids}-out{i} {ids}-out-kind" value={kind} options={kindOptions}
        onchange={(v) => setOutputKind(i, v)} />
      <Select aria-labelledby="{ids}-out{i} {ids}-out-target" value={targetRef(output.target)}
        options={targetCandidates(model, kind).map((c) => ({ value: c.id, label: elementLabel(c.name, c.id) }))}
        onchange={(v) => setOutputRef(i, v)} />
      <span class="labelled" title={t("physics.outputs.particle.tooltip")}>
        <span aria-hidden="true">{t("physics.outputs.particle.label")}</span>
        <NumberField value={output.particle} min={1} max={rig.particles.length - 1} step={1} size={3}
          aria-labelledby="{ids}-out{i} {ids}-out-particle"
          onchange={(v) => {
            const n = finiteValue(v);
            if (n !== null && n >= 1) patchOutput(i, { particle: Math.round(n) });
          }} />
      </span>
      <Select aria-labelledby="{ids}-out{i} {ids}-out-source" value={output.source} options={sourceOptions}
        onchange={(v) => patchOutput(i, { source: v })} />
      <span class="labelled" title={t("physics.outputs.scale.tooltip")}>
        <span aria-hidden="true">×</span>
        <NumberField value={output.scale} step={0.1} decimals={4} size={4}
          aria-labelledby="{ids}-out{i} {ids}-out-scale"
          onchange={(v) => {
            const n = finiteValue(v);
            if (n !== null) patchOutput(i, { scale: n });
          }} />
      </span>
      <span title={t("physics.outputs.weight.tooltip")}>
        <NumberField value={output.weight} step={5} decimals={4} size={4} unit="%"
          aria-labelledby="{ids}-out{i} {ids}-out-weight"
          onchange={(v) => {
            const n = finiteValue(v);
            if (n !== null) patchOutput(i, { weight: n });
          }} />
      </span>
      <Checkbox checked={output.reflect} label={t("physics.outputs.flip")}
        onchange={(on) => patchOutput(i, { reflect: on })} />
      {#if hasBlend(output)}
        <span title={t("physics.outputs.blend.tooltip")}>
          <Select aria-labelledby="{ids}-out{i} {ids}-out-blend" value={outputBlend(output)} options={blendOptions}
            onchange={(v) => patchOutput(i, { blend: v })} />
        </span>
      {/if}
      {#if output.target.type === "vertices"}
        <span class="muted">{t("physics.outputs.weights", { count: output.target.weights.length })}</span>
      {/if}
      <span class="remove">
        <IconButton small disabled={busy} label={t("physics.outputs.remove", { index: i })}
          onclick={() => updateOutputs((outputs) => outputs.filter((_, k) => k !== i))}><Icon name="close" /></IconButton
        >
      </span>
    </div>
  {:else}
    <p class="ed-hint">{t("physics.outputs.empty")}</p>
  {/each}
  <div class="ed-buttons row">
    <Button disabled={busy} onclick={addOutput}>{t("physics.outputs.add")}</Button>
  </div>
</section>

<section>
  <h3>{t("physics.collides.title")}</h3>
  {#if settings && settings.colliders.length > 0}
    <div class="checks">
      {#each settings.colliders as c (c.id)}
        <Checkbox checked={rig.colliders.includes(c.id)} disabled={busy} label={elementLabel(c.name, c.id)}
          onchange={(on) => setColliders(c.id, on)} />
      {/each}
    </div>
  {:else}
    <p class="ed-hint">{t("physics.collides.empty")}</p>
  {/if}
</section>

<section>
  <h3>{t("physics.range.title")}</h3>
  {#each rangeFields as f (f.field)}
    <span hidden id="{ids}-range-{f.field}">{f.label}</span>
  {/each}
  {#each ["position", "angle"] as const as which (which)}
    <div class="ed-field">
      <span id="{ids}-range-{which}">{which === "position" ? t("physics.range.position") : t("physics.range.angle")}</span>
      <span class="pair">
        {#each rangeFields as f (f.field)}
          <span title={f.label}>
            <NumberField value={rig.normalization[which][f.field]} step={1} decimals={4} size={4}
              {...normalizationBounds(rig.normalization[which], f.field)}
              aria-labelledby="{ids}-range-{which} {ids}-range-{f.field}"
              onchange={(v) => setNormalization(which, f.field, v)} />
          </span>
        {/each}
      </span>
    </div>
  {/each}
</section>

<style>
.row {
  margin: var(--ugoe-space-2) 0;
}

.pair {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ugoe-space-2);
  align-items: center;
  min-width: 0;
}

.card {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ugoe-space-2) var(--ugoe-space-3);
  align-items: center;
  padding: var(--ugoe-space-2) 0;
  border-bottom: 1px solid var(--ugoe-color-border);
}

.labelled {
  display: inline-flex;
  align-items: center;
  gap: var(--ugoe-space-1);
}

.labelled > span:first-child {
  font-size: var(--ugoe-font-size-xs);
  color: var(--ugoe-color-text-muted);
}

.pair :global(.ugoe-text-field) {
  flex: 1;
}

.index {
  font-weight: var(--ugoe-font-weight-strong);
  min-width: 2em;
}

.remove {
  margin-left: auto;
}

.checks {
  display: flex;
  flex-direction: column;
  gap: var(--ugoe-space-2);
}

.id {
  font-size: var(--ugoe-font-size-xs);
}
</style>
