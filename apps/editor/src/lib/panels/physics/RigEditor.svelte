<!--
  物理のリグ 1 つの編集 (docs/editor.md#物理の画面での操作): 名前・重力の基準・衝突する衝突体・粒子の鎖と係数・
  入力・出力・正規化の範囲。変更は入力欄の確定 (Enter・フォーカスを外す) ごとに 1 つのコマンドを送る。
  入力・出力・粒子は ID を持たないので、並びごと置き換える (set_physics_inputs など)。
-->
<script lang="ts">
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
  outputBlend,
  type ParticleField,
  removeParticle,
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
  send: (command: Command) => Promise<boolean>;
  busy: boolean;
}

let { rig, model, settings, send, busy }: Props = $props();

const SOURCES: PhysicsSource[] = ["x", "y", "angle"];

const PARTICLE_FIELDS: { field: ParticleField; label: string; title: string; step: number }[] = [
  { field: "radius", label: "Length", title: "Distance from the previous particle (px)", step: 1 },
  {
    field: "mobility",
    label: "Sway",
    title: "Mobility: velocity kept per 1/30 s (0..1)",
    step: 0.05,
  },
  {
    field: "delay",
    label: "Speed",
    title: "Delay: time scale (1 = normal, larger reacts faster)",
    step: 0.05,
  },
  {
    field: "acceleration",
    label: "Settle",
    title: "Acceleration: gravity scale (larger settles faster)",
    step: 0.1,
  },
  { field: "collision_radius", label: "Hit r", title: "Collision radius (px)", step: 1 },
  { field: "x", label: "x", title: "Initial position x (relative to the root, px)", step: 1 },
  { field: "y", label: "y", title: "Initial position y (relative to the root, px)", step: 1 },
];

const deformers = $derived(model.deformers);

function rename(name: string): void {
  if (name !== rig.name) void send({ op: "rename_physics_rig", id: rig.id, name });
}

function setFrame(frame: string): void {
  void send({
    op: "set_physics_frame",
    rig: rig.id,
    ...(frame ? { frame } : {}),
  });
}

function setColliders(id: string, on: boolean): void {
  const next = on ? [...rig.colliders, id] : rig.colliders.filter((c) => c !== id);
  void send({ op: "set_physics_rig_colliders", rig: rig.id, colliders: next });
}

function setParticles(particles: PhysicsParticle[]): void {
  void send({ op: "set_physics_particles", rig: rig.id, particles });
}

function setParticle(index: number, field: ParticleField, raw: number): void {
  const value = finiteValue(raw);
  if (value !== null) setParticles(setParticleField(rig.particles, index, field, value));
}

function particleValue(p: PhysicsParticle, field: ParticleField): number {
  if (field === "x") return p.position[0];
  if (field === "y") return p.position[1];
  return p[field];
}

function setInputs(inputs: PhysicsInput[]): void {
  void send({ op: "set_physics_inputs", rig: rig.id, inputs });
}

function patchInput(index: number, patch: Partial<PhysicsInput>): void {
  setInputs(rig.inputs.map((input, i) => (i === index ? { ...input, ...patch } : input)));
}

function addInput(): void {
  const parameter = model.parameters[0];
  if (!parameter) return;
  setInputs([...rig.inputs, { parameter: parameter.id, source: "x", weight: 100, reflect: false }]);
}

function setOutputs(outputs: PhysicsOutput[]): void {
  void send({ op: "set_physics_outputs", rig: rig.id, outputs });
}

function patchOutput(index: number, patch: Partial<PhysicsOutput>): void {
  setOutputs(rig.outputs.map((o, i) => (i === index ? { ...o, ...patch } : o)));
}

function setOutputKind(index: number, kind: TargetKind): void {
  const output = rig.outputs[index];
  const first = targetCandidates(model, kind)[0];
  if (!output || !first) return;
  patchOutput(index, { target: makeTarget(model, kind, first.id) });
}

function setOutputRef(index: number, id: string): void {
  const output = rig.outputs[index];
  if (output)
    patchOutput(index, { target: makeTarget(model, targetKind(output.target), id, output.target) });
}

function addOutput(): void {
  const output = newOutput(model, "deformer_angle", 1) ?? newOutput(model, "parameter", 1);
  if (output) setOutputs([...rig.outputs, output]);
}

function setNormalization(
  which: "position" | "angle",
  field: keyof NormalizationRange,
  raw: number,
): void {
  const value = finiteValue(raw);
  if (value === null) return;
  const normalization = {
    ...rig.normalization,
    [which]: { ...rig.normalization[which], [field]: value },
  };
  void send({ op: "set_physics_normalization", rig: rig.id, normalization });
}

function number(e: Event): number {
  return (e.currentTarget as HTMLInputElement).valueAsNumber;
}
</script>

<section>
  <h3>Rig</h3>
  <div class="grid">
    <span>Name</span>
    <span class="row">
      <input type="text" value={rig.name} aria-label="Rig name"
        onchange={(e) => rename(e.currentTarget.value)} />
      <span class="muted id">{rig.id}</span>
    </span>
    <span title="Gravity and wind are measured in this deformer's space (tilting the head tilts the hanging direction)">Gravity frame</span>
    <select value={rig.frame ?? ""} aria-label="Gravity frame" onchange={(e) => setFrame(e.currentTarget.value)}>
      <option value="">Canvas</option>
      {#each deformers as d (d.id)}
        <option value={d.id}>{elementLabel(d.name, d.id)}</option>
      {/each}
    </select>
  </div>
  <div class="row">
    <button disabled={busy} onclick={() => send({ op: "remove_physics_rig", id: rig.id })}>
      Delete rig
    </button>
  </div>
</section>

<section>
  <h3>Particles</h3>
  <p class="muted hint">Particle 0 is the root. It follows the first deformer output (or the gravity frame).</p>
  {#each rig.particles as p, i (i)}
    <div class="card">
      <span class="index">#{i}</span>
      {#each PARTICLE_FIELDS as f (f.field)}
        {#if i > 0 || f.field === "x" || f.field === "y"}
          <label title={f.title}>
            <span>{f.label}</span>
            <input type="number" step={f.step} value={particleValue(p, f.field)}
              aria-label="Particle {i} {f.label}"
              onchange={(e) => setParticle(i, f.field, number(e))} />
          </label>
        {/if}
      {/each}
      {#if i > 0}
        <button class="remove" disabled={busy} aria-label="Remove particle {i}"
          onclick={() => setParticles(removeParticle(rig.particles, i))}>×</button>
      {/if}
    </div>
  {/each}
  <div class="row">
    <button disabled={busy} onclick={() => setParticles(appendParticle(rig.particles))}>+ Particle</button>
  </div>
</section>

<section>
  <h3>Inputs</h3>
  {#each rig.inputs as input, i (i)}
    <div class="card">
      <select value={input.parameter} aria-label="Input {i} parameter"
        onchange={(e) => patchInput(i, { parameter: e.currentTarget.value })}>
        {#each model.parameters as p (p.id)}
          <option value={p.id}>{elementLabel(p.name, p.id)}</option>
        {/each}
      </select>
      <select value={input.source} aria-label="Input {i} source"
        onchange={(e) => patchInput(i, { source: e.currentTarget.value as PhysicsSource })}>
        {#each SOURCES as s (s)}<option value={s}>{s}</option>{/each}
      </select>
      <label title="Weight (%)"><span>%</span>
        <input type="number" step="5" value={input.weight} aria-label="Input {i} weight"
          onchange={(e) => {
            const w = finiteValue(number(e));
            if (w !== null) patchInput(i, { weight: w });
          }} />
      </label>
      <label class="check"><input type="checkbox" checked={input.reflect}
        onchange={(e) => patchInput(i, { reflect: e.currentTarget.checked })} />Flip</label>
      <button class="remove" disabled={busy} aria-label="Remove input {i}"
        onclick={() => setInputs(rig.inputs.filter((_, k) => k !== i))}>×</button>
    </div>
  {:else}
    <p class="muted hint">No inputs. The root still follows its deformer.</p>
  {/each}
  <div class="row">
    <button disabled={busy || model.parameters.length === 0} onclick={addInput}>+ Input</button>
  </div>
</section>

<section>
  <h3>Outputs</h3>
  {#each rig.outputs as output, i (i)}
    {@const kind = targetKind(output.target)}
    <div class="card">
      <select value={kind} aria-label="Output {i} kind"
        onchange={(e) => setOutputKind(i, e.currentTarget.value as TargetKind)}>
        {#each TARGET_KINDS as k (k.kind)}
          <option value={k.kind} disabled={targetCandidates(model, k.kind).length === 0}>{k.label}</option>
        {/each}
      </select>
      <select value={targetRef(output.target)} aria-label="Output {i} target"
        onchange={(e) => setOutputRef(i, e.currentTarget.value)}>
        {#each targetCandidates(model, kind) as c (c.id)}
          <option value={c.id}>{elementLabel(c.name, c.id)}</option>
        {/each}
      </select>
      <label title="Particle index (1 = end of the first segment)"><span>Particle</span>
        <input type="number" min="1" max={rig.particles.length - 1} step="1" value={output.particle}
          aria-label="Output {i} particle"
          onchange={(e) => {
            const v = finiteValue(number(e));
            if (v !== null && v >= 1) patchOutput(i, { particle: Math.round(v) });
          }} />
      </label>
      <select value={output.source} aria-label="Output {i} source"
        onchange={(e) => patchOutput(i, { source: e.currentTarget.value as PhysicsSource })}>
        {#each SOURCES as s (s)}<option value={s}>{s}</option>{/each}
      </select>
      <label title="Scale"><span>×</span>
        <input type="number" step="0.1" value={output.scale} aria-label="Output {i} scale"
          onchange={(e) => {
            const v = finiteValue(number(e));
            if (v !== null) patchOutput(i, { scale: v });
          }} />
      </label>
      <label title="Weight (%)"><span>%</span>
        <input type="number" step="5" value={output.weight} aria-label="Output {i} weight"
          onchange={(e) => {
            const v = finiteValue(number(e));
            if (v !== null) patchOutput(i, { weight: v });
          }} />
      </label>
      <label class="check"><input type="checkbox" checked={output.reflect}
        onchange={(e) => patchOutput(i, { reflect: e.currentTarget.checked })} />Flip</label>
      {#if hasBlend(output)}
        <select value={outputBlend(output)} aria-label="Output {i} blend"
          title="How the output combines with the parameter value (Add keeps tracking and adds the sway)"
          onchange={(e) => patchOutput(i, { blend: e.currentTarget.value as PhysicsBlend })}>
          {#each BLENDS as b (b.blend)}<option value={b.blend} title={b.title}>{b.label}</option>{/each}
        </select>
      {/if}
      {#if output.target.type === "vertices"}
        <span class="muted">{output.target.weights.length} weights</span>
      {/if}
      <button class="remove" disabled={busy} aria-label="Remove output {i}"
        onclick={() => setOutputs(rig.outputs.filter((_, k) => k !== i))}>×</button>
    </div>
  {:else}
    <p class="muted hint">No outputs. Add one to make the rig move something.</p>
  {/each}
  <div class="row">
    <button disabled={busy} onclick={addOutput}>+ Output</button>
  </div>
</section>

<section>
  <h3>Collides with</h3>
  {#if settings && settings.colliders.length > 0}
    {#each settings.colliders as c (c.id)}
      <label class="check">
        <input type="checkbox" checked={rig.colliders.includes(c.id)} disabled={busy}
          onchange={(e) => setColliders(c.id, e.currentTarget.checked)} />
        {elementLabel(c.name, c.id)}
      </label>
    {/each}
  {:else}
    <p class="muted hint">Add colliders below, then tick the ones this rig collides with.</p>
  {/if}
</section>

<section>
  <h3>Input range</h3>
  <div class="grid">
    {#each ["position", "angle"] as const as which (which)}
      <span>{which === "position" ? "Position" : "Angle"}</span>
      <span class="row">
        {#each ["min", "default", "max"] as const as field (field)}
          <input type="number" step="1" value={rig.normalization[which][field]}
            aria-label="{which} {field}" title={field}
            onchange={(e) => setNormalization(which, field, number(e))} />
        {/each}
      </span>
    {/each}
  </div>
</section>

<style>
h3 {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
}

.grid {
  display: grid;
  grid-template-columns: 6em 1fr;
  gap: 4px;
  align-items: center;
}

.row {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  align-items: center;
  margin: 4px 0;
}

.grid .row {
  margin: 0;
}

.card {
  display: flex;
  flex-wrap: wrap;
  gap: 4px 6px;
  align-items: center;
  padding: 4px 0;
  border-bottom: 1px solid var(--border);
}

.card label {
  display: inline-flex;
  align-items: center;
  gap: 2px;
}

.card label span {
  font-size: 11px;
  color: var(--muted);
}

input[type="number"] {
  width: 4.2em;
  min-width: 0;
}

input[type="text"] {
  min-width: 0;
  flex: 1;
}

select {
  max-width: 100%;
  min-width: 0;
}

.index {
  font-weight: 600;
  min-width: 2em;
}

.remove {
  margin-left: auto;
}

.check {
  display: flex;
  align-items: center;
  gap: 4px;
}

.id {
  font-size: 11px;
}

.hint {
  font-size: 11px;
  margin: 2px 0;
}
</style>
