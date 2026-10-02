<!--
  パラメータの編集欄の「Mirror」: 左右の対 (Parameter::mirror) を選ぶと、その場で set_parameter_mirror を送る
  (対は双方向に張られる)。ミラー (docs/spec/keyform-inference.md#ミラー) で、格子の軸を相手のパラメータ
  (自分自身なら符号を反転した値) に対応させるのに使う。標準の命名 (EyeLOpen ↔ EyeROpen など) から対を提案する。
-->
<script lang="ts">
import { Button, Select, Tooltip } from "@ugoe/ui";
import type { EditorDocument, ParameterDef } from "../../document.svelte";
import { t } from "../../i18n";
import { labelOf } from "../../labels";
import { mirrorOptions, setMirrorCommand, suggestedMirror } from "./mirror";

interface Props {
  doc: EditorDocument;
  parameter: ParameterDef;
  parameters: ParameterDef[];
}

let { doc, parameter, parameters }: Props = $props();

/** 「対なし」の選択肢の値。Select の空文字列は「何も選んでいない」なので、別の値に置き換える。 */
const NONE = "\u0000none";

const options = $derived(
  mirrorOptions(parameters, parameter.id).map((o) => ({ ...o, value: o.value || NONE })),
);
const current = $derived(parameter.mirror ?? "");
const suggestion = $derived(
  current === ""
    ? suggestedMirror(
        parameter.id,
        parameters.map((p) => p.id),
      )
    : null,
);
const id = $props.id();

function suggestionLabel(id: string): string {
  return id === parameter.id ? t("parameters.mirror.self") : labelOf(parameters, id);
}

function set(value: string): void {
  if (value !== current) doc.apply(setMirrorCommand(parameter.id, value));
}
</script>

<Tooltip text={t("parameters.mirror.tooltip")}>
  {#snippet children(attrs)}
    <span class="mirror" {...attrs}>
      <span id="{id}-label">{t("parameters.mirror.label")}</span>
      <Select
        labelledby="{id}-label"
        value={current || NONE}
        {options}
        onchange={(v) => set(v === NONE ? "" : v)}
      />
    </span>
  {/snippet}
</Tooltip>
{#if suggestion}
  <span class="suggest">
    <Button small onclick={() => set(suggestion)}
      >{t("parameters.mirror.suggest", { name: suggestionLabel(suggestion) })}</Button
    >
  </span>
{/if}

<style>
.mirror {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  align-items: center;
  gap: var(--ugoe-space-2);
}

.mirror :global(.ugoe-select) {
  min-width: 0;
}

.suggest {
  align-self: flex-start;
}
</style>
