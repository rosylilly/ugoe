<!--
  パラメータの編集欄の「Group」: パラメータを入れるグループ (Parameter::group) を選ぶと、その場で set_parameter_group を送る
  (docs/editor.md#パラメータのグループ)。グループは同じパネルの「Groups」で作る。
-->
<script lang="ts">
import { Select } from "@ugoe/ui";
import type { EditorDocument, ParameterDef } from "../../document.svelte";
import type { ParameterGroup } from "../../generated/types";
import { t } from "../../i18n";
import { groupOptions, setGroupCommand } from "./groups";

interface Props {
  doc: EditorDocument;
  parameter: ParameterDef;
  groups: readonly ParameterGroup[];
}

let { doc, parameter, groups }: Props = $props();

/** 「グループなし」の選択肢の値。Select の空文字列は「何も選んでいない」なので、別の値に置き換える。 */
const NONE = "\u0000none";

const options = $derived(groupOptions(groups).map((o) => ({ ...o, value: o.value || NONE })));
const current = $derived(parameter.group ?? "");
const id = $props.id();

function set(value: string): void {
  if (value !== current) doc.apply(setGroupCommand(parameter.id, value));
}
</script>

<span class="group">
  <span id="{id}-label">{t("parameters.group.label")}</span>
  <Select
    aria-labelledby="{id}-label"
    value={current || NONE}
    {options}
    onchange={(v) => set(v === NONE ? "" : v)}
  />
</span>

<style>
.group {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  align-items: center;
  gap: var(--ugoe-space-2);
}

.group :global(.ugoe-select) {
  min-width: 0;
}
</style>
