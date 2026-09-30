<!--
  物理の設定のパネル (docs/editor.md#物理の画面での操作)。右の列の「Physics」タブ。
  操作はすべて物理のコマンド (set_physics_* など) を送るだけで、組み立ては physics.ts にある。

  - 「Preview」で物理のプレビューを始める (ViewportController.startPhysicsPreview)。パラメータや頭を動かすと揺れる。
    「Stop」で止めると編集中の姿に戻る。
  - リグの一覧・テンプレート (TemplateSection)・選んだリグの編集 (RigEditor)・衝突体 (ColliderSection)。
  - 静止姿勢で衝突体に当たっている鎖があれば、上に警告を出す (RestPoseWarnings)。行を押すとそのリグと衝突体を選ぶ。
  - リグは上から順に処理される。▲▼ で並べ替える (reorder_physics_rig)。前のリグの出力を読むリグには「relay」と出す
    (docs/spec/physics.md#リグの順序と中継)。
  - パネルを開いている間、ビューポートに粒子と衝突体を重ねる (viewport/tools/physics/PhysicsOverlay.svelte)。
-->
<script lang="ts">
import BaseLock from "../../BaseLock.svelte";
import type { EditorDocument } from "../../document.svelte";
import type { Command, Model } from "../../ipc";
import { elementLabel } from "../../labels";
import { isBaseRig } from "../../moduleProject";
import type { Selection } from "../../selection.svelte";
import type { ViewportController } from "../../viewport/controller.svelte";
import ColliderSection from "./ColliderSection.svelte";
import {
  chainCommand,
  chainOrder,
  DEFAULT_SETTINGS,
  finiteValue,
  isDynamicSettings,
  moveRigCommand,
  newRig,
  relayInputs,
  worldCommand,
} from "./physics";
import type { PhysicsView } from "./physicsView.svelte";
import RestPoseWarnings from "./RestPoseWarnings.svelte";
import RigEditor from "./RigEditor.svelte";
import TemplateSection from "./TemplateSection.svelte";

interface Props {
  doc: EditorDocument;
  selection: Selection;
  viewport: ViewportController;
  view: PhysicsView;
}

let { doc, selection, viewport, view }: Props = $props();

const model = $derived.by((): Model | null => {
  try {
    return doc.model ? (JSON.parse(doc.model.json) as Model) : null;
  } catch {
    return null;
  }
});
const settings = $derived(doc.physics);
const world = $derived(settings ?? DEFAULT_SETTINGS);
const rig = $derived(settings?.rigs.find((r) => r.id === view.rig) ?? null);

// 選んでいたリグが消えたら (Undo など) 選択を外す。
$effect(() => {
  if (view.rig !== null && !rig) view.rig = settings?.rigs[0]?.id ?? null;
});

/** 選択中の回転デフォーマ (親から子の順)。「From selected deformers」で鎖にする。 */
const chain = $derived(
  model
    ? chainOrder(
        model,
        selection.items.filter((r) => r.kind === "deformer").map((r) => r.id),
      )
    : [],
);

let busy = $state(false);

async function send(command: Command): Promise<boolean> {
  if (busy) return false;
  busy = true;
  try {
    return (await doc.apply(command)) !== null;
  } finally {
    busy = false;
  }
}

async function addRig(): Promise<void> {
  const created = newRig(settings, `Rig ${(settings?.rigs.length ?? 0) + 1}`);
  if (await send({ op: "add_physics_rig", rig: created })) view.rig = created.id;
}

async function addChain(): Promise<void> {
  const before = new Set(settings?.rigs.map((r) => r.id));
  if (await send(chainCommand(chain))) {
    view.rig = doc.physics?.rigs.find((r) => !before.has(r.id))?.id ?? view.rig;
  }
}

function setWorld(field: "gx" | "gy" | "wx" | "wy" | "fps", raw: number): void {
  const value = finiteValue(raw);
  if (value === null) return;
  const [gx, gy] = world.gravity;
  const [wx, wy] = world.wind;
  const patch =
    field === "fps"
      ? { fps: value }
      : field === "gx" || field === "gy"
        ? { gravity: (field === "gx" ? [value, gy] : [gx, value]) as [number, number] }
        : { wind: (field === "wx" ? [value, wy] : [wx, value]) as [number, number] };
  void send(worldCommand(settings, patch));
}

function moveRig(id: string, delta: -1 | 1): void {
  const command = moveRigCommand(settings, id, delta);
  if (command) void send(command);
}

function togglePreview(): void {
  if (viewport.previewingPhysics) viewport.stopPhysicsPreview();
  else viewport.startPhysicsPreview();
}
</script>

<h2>Physics</h2>

<div class="row">
  <button
    class:active={viewport.previewingPhysics}
    disabled={!settings || settings.rigs.length === 0}
    onclick={togglePreview}
    title="Run the physics. Move parameters (or the head) to see it swing; stop to return to the edit pose."
  >
    {viewport.previewingPhysics ? "Stop preview" : "Preview"}
  </button>
  {#if viewport.previewingPhysics}<span class="muted">Previewing…</span>{/if}
</div>
{#if settings && !isDynamicSettings(settings) && settings.rigs.length > 0}
  <p class="muted hint">
    These rigs only output to parameters, so their particles are not in canvas space and are not
    drawn in the viewport.
  </p>
{/if}
<RestPoseWarnings
  {doc}
  selected={{ rig: view.rig, collider: view.collider }}
  onSelect={(rig, collider) => ((view.rig = rig), (view.collider = collider))}
/>

<section>
  <h3>World</h3>
  <BaseLock
    locked={doc.module !== null}
    hint="Gravity, wind and steps belong to the base model and can't be edited here. Edit them in the base project."
  >
  <div class="grid">
    <span>Gravity</span>
    <span class="row">
      <input type="number" step="0.1" value={world.gravity[0]} aria-label="Gravity x"
        onchange={(e) => setWorld("gx", e.currentTarget.valueAsNumber)} />
      <input type="number" step="0.1" value={world.gravity[1]} aria-label="Gravity y"
        onchange={(e) => setWorld("gy", e.currentTarget.valueAsNumber)} />
    </span>
    <span>Wind</span>
    <span class="row">
      <input type="number" step="0.1" value={world.wind[0]} aria-label="Wind x"
        onchange={(e) => setWorld("wx", e.currentTarget.valueAsNumber)} />
      <input type="number" step="0.1" value={world.wind[1]} aria-label="Wind y"
        onchange={(e) => setWorld("wy", e.currentTarget.valueAsNumber)} />
    </span>
    <span>Steps / s</span>
    <span class="row">
      <input type="number" step="1" min="1" value={world.fps} aria-label="Physics steps per second"
        onchange={(e) => setWorld("fps", e.currentTarget.valueAsNumber)} />
    </span>
  </div>
  </BaseLock>
</section>

<section>
  <h3>Rigs</h3>
  {#if settings && settings.rigs.length > 0}
    <p class="muted hint">
      Rigs run top to bottom. A rig that reads a parameter written by a rig above it gets that
      value in the same step (relay).
    </p>
    <ul class="list">
      {#each settings.rigs as r, i (r.id)}
        {@const relay = relayInputs(settings, i)}
        <li class="rig">
          <button class="item" class:selected={r.id === view.rig} onclick={() => (view.rig = r.id)}>
            {elementLabel(r.name, r.id)}
            <span class="muted">· {r.particles.length} particles · {r.outputs.length} outputs</span>
            {#if relay.length > 0}
              <span class="relay" title="Reads {relay.join(', ')} from the rigs above in the same step"
                >relay</span
              >
            {/if}
          </button>
          <span class="order">
            <button
              disabled={busy || i === 0}
              onclick={() => moveRig(r.id, -1)}
              title="Move up (run earlier)"
              aria-label="Move {elementLabel(r.name, r.id)} up">▲</button
            >
            <button
              disabled={busy || i === settings.rigs.length - 1}
              onclick={() => moveRig(r.id, 1)}
              title="Move down (run later)"
              aria-label="Move {elementLabel(r.name, r.id)} down">▼</button
            >
          </span>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="muted">No physics rigs.</p>
  {/if}
  <div class="row">
    <button disabled={busy} onclick={addRig}>+ Rig</button>
    <button
      disabled={busy || chain.length === 0}
      onclick={addChain}
      title="Select rotation deformers (root to tip, e.g. a hair strand's deformers) in the tree, then create a rig whose particles follow them and swing them"
    >
      + From selected deformers{chain.length > 0 ? ` (${chain.length})` : ""}
    </button>
  </div>
</section>

{#if model}
  <TemplateSection
    {model}
    {send}
    {busy}
    onApplied={(rigs) => (view.rig = rigs.find((id) => doc.physics?.rigs.some((r) => r.id === id)) ?? view.rig)}
  />
{/if}

{#if rig && model}
  {#key rig.id}
    <BaseLock locked={isBaseRig(doc.module, rig.id)}>
      <RigEditor {rig} {model} {settings} {send} {busy} />
    </BaseLock>
  {/key}
{/if}

<ColliderSection {doc} {model} {selection} {viewport} {view} {send} {busy} />

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

input[type="number"] {
  width: 4.5em;
  min-width: 0;
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

.list {
  list-style: none;
  margin: 0;
  padding: 0;
}

.item {
  width: 100%;
  text-align: left;
  border: 1px solid transparent;
  background: none;
  overflow-wrap: anywhere;
}

.item.selected {
  border-color: var(--accent);
}

.rig {
  display: flex;
  align-items: center;
  gap: 2px;
}

.rig .item {
  flex: 1;
  min-width: 0;
}

.order {
  display: flex;
  flex: none;
}

.order button {
  padding: 0 4px;
  font-size: 10px;
}

.relay {
  margin-left: 4px;
  padding: 0 4px;
  border: 1px solid var(--accent);
  border-radius: 3px;
  color: var(--accent);
  font-size: 10px;
}

button.active {
  border-color: var(--accent);
  color: var(--accent);
}

.hint {
  font-size: 11px;
}
</style>
