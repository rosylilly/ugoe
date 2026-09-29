<!--
  右パネル: 選択中の要素のプロパティとキーフォーム。
  プロパティは T0606 / T0607、キーの格子とキーフォームの編集は T0608、推定の操作は T0613、深度は T0617、描画用の複製は T0633、
  何も選んでいないときのアバタープロファイルは T0631 で追加する。
  それぞれ lib/panels/inspector/ にコンポーネントを置き、ここではそれを並べるだけにする。
-->
<script lang="ts">
import type { EditorDocument } from "../document.svelte";
import type { Model } from "../ipc";
import { labelOf } from "../labels";
import type { Selection } from "../selection.svelte";
import type { ViewportController } from "../viewport/controller.svelte";
import AvatarSection from "./inspector/avatar/AvatarSection.svelte";
import DeformerProperties from "./inspector/DeformerProperties.svelte";
import DepthSection from "./inspector/depth/DepthSection.svelte";
import DrawCloneSection from "./inspector/drawClone/DrawCloneSection.svelte";
import InferenceSection from "./inspector/inference/InferenceSection.svelte";
import RederiveNotice from "./inspector/inference/RederiveNotice.svelte";
import KeyformInspector from "./inspector/keyforms/KeyformInspector.svelte";
import { parseModel } from "./inspector/keyforms/keyforms";
import MeshProperties from "./inspector/MeshProperties.svelte";

interface Props {
  doc: EditorDocument;
  selection: Selection;
  viewport: ViewportController;
}

let { doc, selection, viewport }: Props = $props();

/** 主な選択の表記 (表示名と ID)。 */
const primaryLabel = $derived.by(() => {
  const primary = selection.primary;
  if (!primary || !doc.model) return primary?.id ?? "";
  // 名前も含むモデル全体 (parseModel は同じ JSON の解析を使い回す)。
  const model = parseModel(doc.model.json) as unknown as Model;
  const items = {
    part: model.parts,
    deformer: model.deformers,
    mesh: model.meshes,
    parameter: model.parameters,
  }[primary.kind];
  return labelOf(items, primary.id);
});
</script>

<h2>Inspector</h2>
<RederiveNotice {doc} />
{#if selection.primary}
  <p class="heading">{selection.primary.kind}: {primaryLabel}</p>
  <MeshProperties {doc} {selection} />
  <DrawCloneSection {doc} {selection} {viewport} />
  <DeformerProperties {doc} {selection} />
  <KeyformInspector {doc} {selection} {viewport} />
  <InferenceSection {doc} {selection} {viewport} />
  <DepthSection {doc} {selection} {viewport} />
{:else}
  <p class="muted">Nothing selected (rev {doc.revision})</p>
  <AvatarSection {doc} />
{/if}

<style>
.heading {
  overflow-wrap: anywhere;
}
</style>
