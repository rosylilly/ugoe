<!--
  物理のテンプレート (docs/editor.md#物理のテンプレート)。体の追従・弾み・首の追従を、
  apply_physics_template を 1 つ送って作る (1 回の Undo で戻る)。リグの中身は Rust (physics_template.rs) が作る。
  モジュールのプロジェクトでは、本体の要素を選ばせない: 首の候補から本体のデフォーマを除き、パラメータは作らず、
  本体にあるリグを作り直すテンプレートと、本体にない中継パラメータ (BodyBounce) を作るテンプレートは使えない
  (T0651、docs/editor.md#本体のものの表示)。
-->
<script lang="ts">
import { Checkbox, Select } from "@ugoe/ui";
import { t } from "../../i18n";
import type { Model, ModuleStatus, PhysicsBlend, PhysicsTemplate } from "../../ipc";
import { elementLabel } from "../../labels";
import { baseElements, isBaseRig } from "../../moduleProject";
import TipButton from "../../ui/TipButton.svelte";
import { BLENDS, type SendPhysics } from "./physics";
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
  send: SendPhysics;
  busy: boolean;
  /** 適用したら、テンプレートが作るリグの ID の列で呼ぶ (作られた最初のものを一覧で選ぶ)。 */
  onApplied: (rigs: string[]) => void;
}

let { model, module = null, send, busy, onApplied }: Props = $props();

let createMissingChoice = $state(true);
/** モジュールはパラメータを足せない。 */
const ids = $props.id();
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

const blendOptions = $derived(BLENDS.map((b) => ({ value: b.blend, label: b.label })));
const neckOptions = $derived(
  necks.map((d) => ({ value: d.id, label: elementLabel(d.name, d.id) })),
);
const neckPlaceholder = $derived(
  necks.length > 0
    ? t("physics.templates.neck.choose")
    : module
      ? t("physics.templates.neck.noneModule")
      : t("physics.templates.neck.none"),
);

async function apply(template: PhysicsTemplate): Promise<void> {
  const info = TEMPLATES.find((item) => item.template === template);
  if ((await send(() => templateCommand(template, createMissing, blend, neck))) && info) {
    onApplied(info.rigs);
  }
}
</script>

<section>
  <h3>{t("physics.templates.title")}</h3>
  <p class="ed-hint">{t("physics.templates.hint")}</p>
  <div class="row">
    <Checkbox checked={createMissing} disabled={module !== null} label={t("physics.templates.createMissing.label")}
      aria-describedby={module ? `${ids}-create-missing` : undefined}
      onchange={(on) => (createMissingChoice = on)} />
    {#if module}
      <!-- 押せない理由は近くに見える文で書く (docs/ui.md#部品の一覧 の使い分け) -->
      <p class="ed-hint" id="{ids}-create-missing">{t("physics.templates.createMissing.description")}</p>
    {/if}
  </div>
  <div class="row">
    <Select label={t("physics.templates.blend.label")} value={blend} options={blendOptions}
      tooltip={t("physics.templates.blend.tooltip")} onchange={(v) => (blend = v)} />
  </div>
  <div class="row">
    <Select
      label={t("physics.templates.neck.label")}
      value={neck ?? ""}
      options={neckOptions}
      placeholder={neckPlaceholder}
      disabled={necks.length === 0}
      onchange={(v) => (neckChoice = v || null)}
    />
  </div>
  {#each TEMPLATES as tpl (tpl.template)}
    {@const missing = missingParameters(model, tpl.template)}
    {@const inBase = baseRigsOf(tpl.rigs)}
    {@const relays = module ? missingRelays(model, tpl.template) : []}
    <div class="ed-buttons row">
      <TipButton
        disabled={busy || (tpl.template === "neck_follow" && !neck) || inBase.length > 0 || relays.length > 0}
        tooltip={tpl.title}
        onclick={() => apply(tpl.template)}>{t("physics.templates.apply", { template: tpl.label })}</TipButton
      >
      {#if inBase.length > 0}
        <span class="ed-hint">{t("physics.templates.inBase", { rigs: inBase.join(", ") })}</span>
      {:else if relays.length > 0}
        <span class="ed-hint">{t("physics.templates.needsRelays", { parameters: relays.join(", ") })}</span>
      {:else if missing.length > 0}
        <span class="ed-hint"
          >{createMissing
            ? t("physics.templates.creates", { parameters: missing.join(", ") })
            : t("physics.templates.skips", { parameters: missing.join(", ") })}</span
        >
      {/if}
    </div>
  {/each}
</section>

<style>
.row {
  margin: var(--ugoe-space-2) 0;
}
</style>
