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

/** 開いているか。閉じている間は中身を描かない (パーツ 200 個の一覧を、閉じた欄の数だけ持たない)。 */
let open = $state(false);

/** 一覧にない (消えた要素などの) 選択も、外せるように出す。 */
const missing = $derived(selected.filter((id) => !options.some((o) => o.id === id)));
</script>

<details class="pick" bind:open>
  <summary>
    <span>{title}</span>
    <span class="count" class:none={selected.length === 0}>
      {selected.length === 0
        ? t("slots.pickList.none")
        : formatList(selected.map((id) => options.find((o) => o.id === id)?.label ?? id))}
    </span>
  </summary>
  {#if open}
    <p class="ed-help">{help}</p>
    {#if options.length === 0 && missing.length === 0}
      <p class="ed-help empty">{empty}</p>
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
  align-items: center;
  min-height: var(--ugoe-target-min);
  cursor: pointer;
}

summary:focus-visible {
  outline: var(--ugoe-focus-ring);
  outline-offset: var(--ugoe-focus-offset);
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

.options {
  display: flex;
  flex-direction: column;
  gap: var(--ugoe-space-1);
  /* 選択肢が多いときは一覧の中で送る。行の途中で切って、送れることを見せる */
  max-height: calc(var(--ugoe-target-min) * 7.5);
  padding-inline-end: var(--ugoe-scrollbar-gap);
  overflow-y: auto;
  overflow-wrap: anywhere;
}

.missing :global(.ugoe-checkbox__label) {
  color: var(--ugoe-color-danger);
}
</style>
