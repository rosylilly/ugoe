<script lang="ts">
// パラメータのスライダーの一覧 (グループごと)。中継用 (internal) のパラメータは既定で隠し、切り替えて出せる。
import { formatValue, internalCount, type Player, sliderStep, visibleSections } from "@ugoe/player";

let { player }: { player: Player } = $props();

let filter = $state("");
let showInternal = $state(false);

let hidden = $derived(internalCount(player.sections));
let sections = $derived(visibleSections(player.sections, filter, showInternal));
</script>

<section class="panel parameters">
  <header>
    <h2>パラメータ</h2>
    <button
      type="button"
      onclick={() => player.resetAllParameters()}
      disabled={player.status !== "ready"}
      title="すべてのパラメータを既定値に戻す">すべて戻す</button
    >
  </header>
  {#if player.sections.length === 0}
    <p class="empty">
      {player.status === "ready" ? "このモデルにはパラメータがありません" : "モデルを開くと表示されます"}
    </p>
  {:else}
    <input class="filter" type="search" placeholder="絞り込み (名前・ID)" bind:value={filter} />
    {#if hidden > 0}
      <label
        class="internal-toggle"
        title="物理のリグの間で値を受け渡す中継用のパラメータ。ふつうは動かしません"
      >
        <input type="checkbox" bind:checked={showInternal} />
        中継用のパラメータも表示 ({hidden})
      </label>
    {/if}
    <div class="list">
      {#each sections as section (section.id ?? "")}
        <details open>
          <summary>{section.name} <span class="count">{section.parameters.length}</span></summary>
          {#each section.parameters as p (p.id)}
            {@const value = player.values[p.id] ?? p.default}
            {@const driven = player.driven.has(p.id)}
            <div class="row" class:driven class:internal={p.internal} title={p.id}>
              <label for="param-{p.id}">{p.name || p.id}</label>
              <span class="value">{formatValue(value)}</span>
              <input
                id="param-{p.id}"
                type="range"
                min={p.min}
                max={p.max}
                step={sliderStep(p)}
                {value}
                disabled={driven}
                oninput={(e) => player.setParameter(p.id, e.currentTarget.valueAsNumber)}
                ondblclick={() => player.resetParameter(p.id)}
              />
              <button
                type="button"
                class="reset"
                onclick={() => player.resetParameter(p.id)}
                disabled={driven || Math.abs(value - p.default) < 1e-6}
                title="既定値 ({formatValue(p.default)}) に戻す"
                aria-label="{p.name || p.id} を既定値に戻す">↺</button
              >
            </div>
          {/each}
        </details>
      {/each}
    </div>
  {/if}
</section>

<style>
  .parameters {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }
  h2 {
    margin: 0;
    font-size: 0.85rem;
    font-weight: 600;
  }
  .empty {
    color: var(--muted);
    font-size: 0.85rem;
  }
  .filter {
    margin: 0.5rem 0;
  }
  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  summary {
    cursor: pointer;
    padding: 0.35rem 0;
    font-weight: 600;
    font-size: 0.8rem;
  }
  .count {
    color: var(--muted);
    font-weight: normal;
  }
  .row {
    display: grid;
    grid-template-columns: 1fr auto auto;
    grid-template-areas:
      "label value reset"
      "slider slider reset";
    align-items: center;
    column-gap: 0.4rem;
    padding: 0.2rem 0 0.35rem 0.5rem;
    font-size: 0.8rem;
  }
  .row label {
    grid-area: label;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row .value {
    grid-area: value;
    font-variant-numeric: tabular-nums;
    color: var(--muted);
  }
  .row input {
    grid-area: slider;
    width: 100%;
    margin: 0;
  }
  .row .reset {
    grid-area: reset;
    padding: 0.1rem 0.4rem;
  }
  .internal-toggle {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    margin-bottom: 0.4rem;
    font-size: 0.75rem;
    color: var(--muted);
  }
  .row.internal label {
    font-style: italic;
    color: var(--muted);
  }
  .row.driven label::after {
    content: " (自動)";
    color: var(--accent);
  }
</style>
