<!--
  物理のテンプレート (docs/editor.md#物理のテンプレート)。体の追従・弾み・首の追従を、
  apply_physics_template を 1 つ送って作る (1 回の Undo で戻る)。リグの中身は Rust (physics_template.rs) が作る。
  モジュールのプロジェクトでは、本体の要素を選ばせない: 首の候補から本体のデフォーマを除き、パラメータは作らず、
  本体にあるリグを作り直すテンプレートと、本体にない中継パラメータ (BodyBounce) を作るテンプレートは使えない
  (T0651、docs/editor.md#本体のものの表示)。
-->
<script lang="ts">
import type { Command, Model, ModuleStatus, PhysicsBlend, PhysicsTemplate } from "../../ipc";
import { elementLabel } from "../../labels";
import { baseElements, isBaseRig } from "../../moduleProject";
import { BLENDS } from "./physics";
import {
  defaultNeck,
  missingParameters,
  missingRelays,
  neckCandidates,
  TEMPLATES,
  templateCommand,
} from "./templates";

interface Props {
  model: Model;
  /** モジュールのプロジェクトなら、その情報 (本体の要素を選ばせない)。 */
  module?: ModuleStatus | null;
  send: (command: Command) => Promise<boolean>;
  busy: boolean;
  /** 適用したら、テンプレートが作るリグの ID の列で呼ぶ (作られた最初のものを一覧で選ぶ)。 */
  onApplied: (rigs: string[]) => void;
}

let { model, module = null, send, busy, onApplied }: Props = $props();

let createMissingChoice = $state(true);
/** モジュールはパラメータを足せない。 */
const createMissing = $derived(module === null && createMissingChoice);
const base = $derived(baseElements(module));
let blend = $state<PhysicsBlend>("add");
/** 選んだ首のデフォーマ。null なら既定 (名前に neck か「首」を含むもの)。 */
let neckChoice = $state<string | null>(null);

const necks = $derived(neckCandidates(model).filter((d) => !base.has(`deformer:${d.id}`)));
const neck = $derived(
  neckChoice && necks.some((d) => d.id === neckChoice) ? neckChoice : defaultNeck(model, necks),
);

/** テンプレートが作るリグのうち本体のもの (作り直すと本体を変えるので、モジュールでは使えない)。 */
function baseRigsOf(rigs: readonly string[]): string[] {
  return rigs.filter((id) => isBaseRig(module, id));
}

async function apply(template: PhysicsTemplate): Promise<void> {
  const info = TEMPLATES.find((t) => t.template === template);
  if ((await send(templateCommand(template, createMissing, blend, neck))) && info) {
    onApplied(info.rigs);
  }
}
</script>

<section>
  <h3>Templates</h3>
  <p class="muted hint">
    Common rigs on the standard parameters. Applying one again resets its rigs. One Undo removes
    everything a template made.
  </p>
  <label class="row" title={module ? "A module can't add parameters. Add them in the base project." : undefined}>
    <input type="checkbox" checked={createMissing} disabled={module !== null}
      onchange={(e) => (createMissingChoice = e.currentTarget.checked)} />
    Create missing parameters
  </label>
  <label class="row" title="How the body rigs write BodyAngle*: Add keeps tracking and motion values and adds the lag">
    Body blend
    <select bind:value={blend} aria-label="Template blend">
      {#each BLENDS as b (b.blend)}
        <option value={b.blend} title={b.title}>{b.label}</option>
      {/each}
    </select>
  </label>
  <label class="row">
    Neck
    <select
      value={neck ?? ""}
      disabled={necks.length === 0}
      aria-label="Neck deformer"
      onchange={(e) => (neckChoice = e.currentTarget.value || null)}
    >
      {#if !neck}<option value=""
          >{necks.length > 0 ? "(choose)" : module ? "(no rotation deformer of the module)" : "(no rotation deformer)"}</option
        >{/if}
      {#each necks as d (d.id)}
        <option value={d.id}>{elementLabel(d.name, d.id)}</option>
      {/each}
    </select>
  </label>
  {#each TEMPLATES as t (t.template)}
    {@const missing = missingParameters(model, t.template)}
    {@const inBase = baseRigsOf(t.rigs)}
    {@const relays = module ? missingRelays(model, t.template) : []}
    <div class="row">
      <button
        disabled={busy || (t.template === "neck_follow" && !neck) || inBase.length > 0 || relays.length > 0}
        title={t.title}
        onclick={() => apply(t.template)}>+ {t.label}</button
      >
      {#if inBase.length > 0}
        <span class="muted hint">the base model has {inBase.join(", ")}: edit it in the base project</span>
      {:else if relays.length > 0}
        <span class="muted hint">needs {relays.join(", ")} in the base model (a module can't add parameters)</span>
      {:else if missing.length > 0}
        <span class="muted hint"
          >{createMissing ? "creates" : "skips rigs using"} {missing.join(", ")}</span
        >
      {/if}
    </div>
  {/each}
</section>

<style>
h3 {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
}

.row {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  align-items: center;
  margin: 4px 0;
}

select {
  min-width: 0;
  max-width: 14em;
}

.hint {
  font-size: 11px;
}
</style>
