<!--
  物理の設定のパネル (docs/editor.md#物理の画面での操作)。右の列の「Physics」タブ。
  操作はすべて物理のコマンド (set_physics_* など) を送るだけで、組み立ては physics.ts にある。

  - 「Preview」で物理のプレビューを始める (ViewportController.startPhysicsPreview)。パラメータや頭を動かすと揺れる。
    「Stop」で止めると編集中の姿に戻る。
  - リグの一覧・選んだリグの編集 (RigEditor)・衝突体 (ColliderSection)。
  - パネルを開いている間、ビューポートに粒子と衝突体を重ねる (viewport/tools/physics/PhysicsOverlay.svelte)。
-->
<script lang="ts">
import type { EditorDocument } from "../../document.svelte";
import type { Command, Model } from "../../ipc";
import { elementLabel } from "../../labels";
import type { Selection } from "../../selection.svelte";
import type { ViewportController } from "../../viewport/controller.svelte";
import ColliderSection from "./ColliderSection.svelte";
import {
  chainCommand,
  chainOrder,
  DEFAULT_SETTINGS,
  finiteValue,
  isDynamicSettings,
  newRig,
  worldCommand,
} from "./physics";
import type { PhysicsView } from "./physicsView.svelte";
import RigEditor from "./RigEditor.svelte";

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

<section>
  <h3>World</h3>
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
</section>

<section>
  <h3>Rigs</h3>
  {#if settings && settings.rigs.length > 0}
    <ul class="list">
      {#each settings.rigs as r (r.id)}
        <li>
          <button class="item" class:selected={r.id === view.rig} onclick={() => (view.rig = r.id)}>
            {elementLabel(r.name, r.id)}
            <span class="muted">· {r.particles.length} particles · {r.outputs.length} outputs</span>
          </button>
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

{#if rig && model}
  {#key rig.id}
    <RigEditor {rig} {model} {settings} {send} {busy} />
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

button.active {
  border-color: var(--accent);
  color: var(--accent);
}

.hint {
  font-size: 11px;
}
</style>
