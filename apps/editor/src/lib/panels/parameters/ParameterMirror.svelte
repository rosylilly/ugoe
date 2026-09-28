<!--
  パラメータの編集欄の「Mirror」: 左右の対 (Parameter::mirror) を選ぶと、その場で set_parameter_mirror を送る
  (対は双方向に張られる)。ミラー (docs/spec/keyform-inference.md#ミラー) で、格子の軸を相手のパラメータ
  (自分自身なら符号を反転した値) に対応させるのに使う。標準の命名 (EyeLOpen ↔ EyeROpen など) から対を提案する。
-->
<script lang="ts">
import type { EditorDocument, ParameterDef } from "../../document.svelte";
import { labelOf } from "../../labels";
import { mirrorOptions, setMirrorCommand, suggestedMirror } from "./mirror";

interface Props {
  doc: EditorDocument;
  parameter: ParameterDef;
  parameters: ParameterDef[];
}

let { doc, parameter, parameters }: Props = $props();

const options = $derived(mirrorOptions(parameters, parameter.id));
const current = $derived(parameter.mirror ?? "");
const suggestion = $derived(
  current === ""
    ? suggestedMirror(
        parameter.id,
        parameters.map((p) => p.id),
      )
    : null,
);

function suggestionLabel(id: string): string {
  return id === parameter.id ? "itself" : labelOf(parameters, id);
}

function set(value: string): void {
  if (value !== current) doc.apply(setMirrorCommand(parameter.id, value));
}
</script>

<label class="mirror">
  <span>Mirror</span>
  <select
    value={current}
    onchange={(e) => set((e.currentTarget as HTMLSelectElement).value)}
    title="The parameter on the other side (used when mirroring keyforms). Pairs are set on both parameters."
  >
    {#each options as option (option.value)}
      <option value={option.value}>{option.label}</option>
    {/each}
  </select>
</label>
{#if suggestion}
  <button class="suggest" onclick={() => set(suggestion)}>Pair with {suggestionLabel(suggestion)}</button>
{/if}

<style>
.mirror {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  align-items: center;
  gap: 4px;
}

.mirror select {
  min-width: 0;
}

.suggest {
  align-self: flex-start;
  font-size: 11px;
}
</style>
