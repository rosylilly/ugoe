<!--
  右パネル: 選択中の要素のプロパティとキーフォーム。
  プロパティは T0606 / T0607、キーの格子とキーフォームの編集は T0608、推定の操作は T0613、深度は T0617、描画用の複製は T0633、
  何も選んでいないときのアバタープロファイルは T0631、リグを提案し直す知らせは T0523 で追加する。
  それぞれ lib/panels/inspector/ にコンポーネントを置き、ここではそれを並べるだけにする。
-->
<script lang="ts">
import BaseLock from "../BaseLock.svelte";
import type { EditorDocument } from "../document.svelte";
import { t } from "../i18n";
import { labelOf } from "../labels";
import { baseElements, isBaseElement } from "../moduleProject";
import type { ElementKind, Selection } from "../selection.svelte";
import type { ViewportController } from "../viewport/controller.svelte";
import AvatarSection from "./inspector/avatar/AvatarSection.svelte";
import DeformerProperties from "./inspector/DeformerProperties.svelte";
import DepthSection from "./inspector/depth/DepthSection.svelte";
import DrawCloneSection from "./inspector/drawClone/DrawCloneSection.svelte";
import InferenceSection from "./inspector/inference/InferenceSection.svelte";
import RederiveNotice from "./inspector/inference/RederiveNotice.svelte";
import KeyformInspector from "./inspector/keyforms/KeyformInspector.svelte";
import MeshProperties from "./inspector/MeshProperties.svelte";
import StaleRigNotice from "./inspector/StaleRigNotice.svelte";

interface Props {
  doc: EditorDocument;
  selection: Selection;
  viewport: ViewportController;
  /** 本体の変形を写した後に形を編集した衣装のメッシュ (`ProposalState.stale_rigs`)。 */
  staleRigs?: readonly { mesh: string; name: string }[];
  /** AI タブを開く (知らせから)。 */
  onopenai?: () => void;
}

let { doc, selection, viewport, staleRigs = [], onopenai = () => {} }: Props = $props();

/** モジュールのプロジェクトで、主な選択が本体の要素 (読み取り専用) か。 */
const primaryIsBase = $derived(
  selection.primary !== null && isBaseElement(baseElements(doc.module), selection.primary),
);

/** 要素の種類の名前。 */
function kindLabel(kind: ElementKind): string {
  return {
    part: t("inspector.kind.part"),
    deformer: t("inspector.kind.deformer"),
    mesh: t("inspector.kind.mesh"),
    parameter: t("inspector.kind.parameter"),
  }[kind];
}

/** 主な選択の表記 (表示名と ID)。 */
const primaryLabel = $derived.by(() => {
  const primary = selection.primary;
  const model = doc.parsedModel;
  if (!primary || !model) return primary?.id ?? "";
  const items = {
    part: model.parts,
    deformer: model.deformers,
    mesh: model.meshes,
    parameter: model.parameters,
  }[primary.kind];
  return labelOf(items, primary.id);
});
</script>

<!-- 右の列のタブと同じ名前なので見た目では隠す (読み上げと見出しの移動に残す。App.svelte) -->
<h2 class="ugoe-visually-hidden">{t("inspector.title")}</h2>
<RederiveNotice {doc} />
{#if selection.primary}
  <p class="heading">
    {t("inspector.heading", { kind: kindLabel(selection.primary.kind), label: primaryLabel })}{#if primaryIsBase}<span
        class="badge">{t("inspector.baseBadge")}</span
      >{/if}
  </p>
  <StaleRigNotice {selection} {staleRigs} onopen={onopenai} />
  <BaseLock locked={primaryIsBase}>
    <MeshProperties {doc} {selection} />
    <DrawCloneSection {doc} {selection} {viewport} />
    <DeformerProperties {doc} {selection} />
    <KeyformInspector {doc} {selection} {viewport} />
    <InferenceSection {doc} {selection} {viewport} />
    <DepthSection {doc} {selection} {viewport} />
  </BaseLock>
{:else}
  <p class="muted">{t("inspector.empty")}</p>
  <BaseLock
    locked={doc.module !== null}
    hint={t("inspector.avatarBaseHint")}
  >
    <AvatarSection {doc} />
  </BaseLock>
{/if}

<style>
.heading {
  overflow-wrap: anywhere;
}
.badge {
  margin-left: var(--ugoe-space-2);
  padding: 0 var(--ugoe-space-2);
  border: 1px solid var(--ugoe-color-border);
  border-radius: var(--ugoe-radius-sm);
  font-size: var(--ugoe-font-size-xs);
  color: var(--ugoe-color-text-muted);
}
</style>
