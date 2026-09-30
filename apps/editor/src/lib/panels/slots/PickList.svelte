<!--
  スロットのパネルの、要素を複数選ぶ欄 (置き換えるパーツ・公開するパラメータなど)。
  畳んだ見出しに選んだ数を出し、開くとチェックの一覧になる。チェックを変えるたびに onchange で新しい一覧を渡す。
-->
<script lang="ts">
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
      {selected.length === 0 ? "none" : selected.join(", ")}
    </span>
  </summary>
  <p class="help">{help}</p>
  {#if options.length === 0 && missing.length === 0}
    <p class="help empty">{empty}</p>
  {:else}
    <div class="options">
      {#each options as option (option.id)}
        <label>
          <input
            type="checkbox"
            {disabled}
            checked={selected.includes(option.id)}
            onchange={(e) => onchange(toggled(selected, option.id, e.currentTarget.checked))}
          />
          <span>{option.label}</span>
        </label>
      {/each}
      {#each missing as id (id)}
        <label class="missing">
          <input
            type="checkbox"
            {disabled}
            checked
            onchange={() => onchange(toggled(selected, id, false))}
          />
          <span>{id} (missing)</span>
        </label>
      {/each}
    </div>
  {/if}
</details>

<style>
.pick {
  margin: 6px 0;
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 2px 6px;
}

summary {
  display: flex;
  gap: 8px;
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
  color: var(--accent);
}

.count.none {
  color: var(--muted);
}

.help {
  margin: 2px 0 4px;
  color: var(--muted);
  font-size: 11px;
}

.options {
  max-height: 11em;
  overflow-y: auto;
}

.options label {
  display: flex;
  align-items: center;
  gap: 6px;
  overflow-wrap: anywhere;
}

.missing {
  color: #dc2626;
}
</style>
