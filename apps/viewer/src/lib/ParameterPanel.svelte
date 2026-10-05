<script lang="ts">
// パラメータのスライダーの一覧 (グループごと)。中継用 (internal) のパラメータは既定で隠し、切り替えて出せる。
import { formatValue, internalCount, type Player, sliderStep, visibleSections } from "@ugoe/player";
import { Button, Section, Slider, TextField, Toggle } from "@ugoe/ui";
import { t } from "./i18n";

let { player }: { player: Player } = $props();

let filter = $state("");
let showInternal = $state(false);

const internalHintId = $props.id();

let hidden = $derived(internalCount(player.sections));
let sections = $derived(visibleSections(player.sections, filter, showInternal));
</script>

<section class="parameters" aria-labelledby="parameters-heading">
  <header>
    <h2 id="parameters-heading">{t("parameters.title")}</h2>
    <Button small onclick={() => player.resetAllParameters()} disabled={player.status !== "ready"}
      >{t("parameters.resetAll.label")}</Button
    >
  </header>
  {#if player.sections.length === 0}
    <p class="empty">
      {player.status === "ready" ? t("parameters.empty") : t("parameters.noModel")}
    </p>
  {:else}
    <div class="filter">
      <TextField
        type="search"
        aria-label={t("parameters.filter.aria")}
        placeholder={t("parameters.filter.placeholder")}
        bind:value={filter}
      />
    </div>
    {#if hidden > 0}
      <div class="internal-toggle">
        <Toggle
          label={t("parameters.showInternal.label", { count: hidden })}
          aria-describedby={internalHintId}
          bind:checked={showInternal}
        />
        <p class="hint" id={internalHintId}>{t("parameters.showInternal.description")}</p>
      </div>
    {/if}
    <div class="list">
      {#each sections as section (section.id ?? "")}
        <Section title={section.id === null ? t("parameters.ungrouped") : section.name}>
          {#snippet actions()}
            <span class="count">{section.parameters.length}</span>
          {/snippet}
          {#each section.parameters as p (p.id)}
            {@const value = player.values[p.id] ?? p.default}
            {@const driven = player.driven.has(p.id)}
            {@const name = p.name || p.id}
            <!-- ダブルクリックでも既定値に戻す (動かしているドライバが無いとき)。 -->
            <div
              class="row"
              class:internal={p.internal}
              data-parameter={p.id}
              role="presentation"
              ondblclick={() => {
                if (!driven) player.resetParameter(p.id);
              }}
            >
              <Slider
                label={driven ? t("parameters.driven", { name }) : name}
                {value}
                min={p.min}
                max={p.max}
                step={sliderStep(p)}
                defaultValue={p.default}
                resetLabel={t("parameters.reset.aria", { name, value: formatValue(p.default) })}
                disabled={driven}
                onchange={(v) => player.setParameter(p.id, v)}
              />
            </div>
          {/each}
        </Section>
      {/each}
    </div>
  {/if}
</section>

<style>
  .parameters {
    flex: 1;
    min-height: 12rem;
    display: flex;
    flex-direction: column;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--ugoe-space-2);
  }
  h2 {
    margin: 0;
    font-size: var(--ugoe-font-size-md);
    font-weight: var(--ugoe-font-weight-strong);
  }
  .empty,
  .hint,
  .count {
    color: var(--ugoe-color-text-muted);
  }
  .empty {
    font-size: var(--ugoe-font-size-sm);
  }
  .hint {
    margin: var(--ugoe-space-1) 0 0;
    font-size: var(--ugoe-font-size-xs);
  }
  .count {
    font-size: var(--ugoe-font-size-xs);
    font-variant-numeric: tabular-nums;
  }
  .filter {
    margin: var(--ugoe-space-3) 0 var(--ugoe-space-2);
  }
  .internal-toggle {
    margin-bottom: var(--ugoe-space-2);
    font-size: var(--ugoe-font-size-sm);
  }
  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  .row {
    padding: var(--ugoe-space-1) 0 var(--ugoe-space-2) var(--ugoe-space-2);
  }
  .row :global(.ugoe-slider) {
    font-size: var(--ugoe-font-size-sm);
  }
  .row :global(.ugoe-slider__label) {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row.internal :global(.ugoe-slider__label) {
    font-style: italic;
    color: var(--ugoe-color-text-muted);
  }
</style>
