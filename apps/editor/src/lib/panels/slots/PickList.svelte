<!--
  スロットのパネルの、要素を複数選ぶ欄 (置き換えるパーツ・公開するパラメータなど)。
  畳んだ見出しに選んだ数を出し、開くとチェックの一覧になる。チェックを変えるたびに onchange で新しい一覧を渡す。
-->
<script lang="ts">
import { formatList } from "@ugoe/i18n";
import { Checkbox } from "@ugoe/ui";
import { t } from "../../i18n";
import { toggled } from "./slots";

interface Props {
  title: string;
  /** 何のための設定かの短い説明。 */
  help: string;
  options: { id: string; label: string }[];
  selected: readonly string[];
  /** 選べるものがないときの説明。 */
  empty: string;
  disabled?: boolean;
  onchange: (next: string[]) => void;
}

let { title, help, options, selected, empty, disabled = false, onchange }: Props = $props();

/** 一覧にない (消えた要素などの) 選択も、外せるように出す。 */
const missing = $derived(selected.filter((id) => !options.some((o) => o.id === id)));
</script>

<details class="pick">
  <summary>
    <span>{title}</span>
    <span class="count" class:none={selected.length === 0}>
      {selected.length === 0 ? t("slots.pickList.none") : formatList(selected)}
    </span>
  </summary>
  <p class="help">{help}</p>
  {#if options.length === 0 && missing.length === 0}
    <p class="help empty">{empty}</p>
  {:else}
    <div class="options">
      {#each options as option (option.id)}
        <Checkbox
          label={option.label}
          {disabled}
          checked={selected.includes(option.id)}
          onchange={(checked) => onchange(toggled(selected, option.id, checked))}
        />
      {/each}
      {#each missing as id (id)}
        <span class="missing">
          <Checkbox
            label={t("slots.pickList.missing", { id })}
            {disabled}
            checked
            onchange={() => onchange(toggled(selected, id, false))}
          />
        </span>
      {/each}
    </div>
  {/if}
</details>

<style>
.pick {
  margin: var(--ugoe-space-2) 0;
  border: 1px solid var(--ugoe-color-border);
  border-radius: var(--ugoe-radius-md);
  padding: var(--ugoe-space-1) var(--ugoe-space-2);
}

summary {
  display: flex;
  gap: var(--ugoe-space-3);
  align-items: baseline;
  cursor: pointer;
}

summary > span:first-child {
  flex: none;
}

.count {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--ugoe-color-accent);
}

.count.none {
  color: var(--ugoe-color-text-muted);
}

.help {
  margin: var(--ugoe-space-1) 0 var(--ugoe-space-2);
  color: var(--ugoe-color-text-muted);
  font-size: var(--ugoe-font-size-sm);
}

.options {
  display: flex;
  flex-direction: column;
  gap: var(--ugoe-space-1);
  max-height: 11em;
  overflow-y: auto;
  overflow-wrap: anywhere;
}

.missing :global(.ugoe-checkbox__label) {
  color: var(--ugoe-color-danger);
}
</style>
