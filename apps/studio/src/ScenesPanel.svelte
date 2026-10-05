<!--
  シーンの一覧 (docs/studio.md#操作ウィンドウ の左)。押すとそのシーンに切り替え、編集の対象にする。
  追加・複製・並べ替え・削除。配信しているシーンと、読み込んでいるシーンに印を付ける。
-->
<script lang="ts">
import { AlertDialog, Button } from "@ugoe/ui";
import { t } from "./lib/i18n";
import { sceneLabel } from "./lib/labels";
import { fileName, type SceneSettings } from "./lib/profile";

interface Props {
  scenes: SceneSettings[];
  /** 配信しているシーンの ID。 */
  live: string | null;
  /** 編集しているシーンの ID。 */
  selected: string | null;
  /** 切り替えを読み込んでいるか (`scene` は読み込んでいるシーンの ID。分からなければ `null`)。 */
  loading?: { scene: string | null } | null;
  disabled?: boolean;
  onactivate: (id: string) => void;
  onadd: () => void;
  onduplicate: (id: string) => void;
  onmove: (id: string, index: number) => void;
  onremove: (id: string) => void;
}

let {
  scenes,
  live,
  selected,
  loading = null,
  disabled = false,
  onactivate,
  onadd,
  onduplicate,
  onmove,
  onremove,
}: Props = $props();

let removing = $state(false);

const index = $derived(scenes.findIndex((s) => s.id === selected));
const current = $derived(index < 0 ? null : (scenes[index] ?? null));
</script>

<section class="scenes" aria-labelledby="scenes-heading">
  <h2 id="scenes-heading">{t("scenes.heading")}</h2>
  {#if scenes.length === 0}
    <p class="scenes__empty">{t("scenes.empty")}</p>
  {:else}
    <ul class="scenes__list" aria-label={t("scenes.listLabel")}>
      {#each scenes as scene, i (scene.id)}
        <li>
          <button
            type="button"
            class="scenes__item"
            class:scenes__item--selected={scene.id === selected}
            class:scenes__item--live={scene.id === live}
            aria-current={scene.id === live ? "true" : undefined}
            aria-busy={scene.id === loading?.scene ? "true" : undefined}
            {disabled}
            onclick={() => onactivate(scene.id)}
          >
            <span class="scenes__name">{sceneLabel(scene, i)}</span>
            <span class="scenes__model">
              {scene.model === null ? t("scenes.noModel") : fileName(scene.model)}
            </span>
            {#if scene.id === loading?.scene}
              <span class="scenes__loading">{t("scenes.loading")}</span>
            {:else if scene.id === live}
              <span class="scenes__live">{t("scenes.live")}</span>
            {/if}
          </button>
        </li>
      {/each}
    </ul>
    <!-- 読み込んでいる間は、切り替えの案内の代わりに読み込み中を出す (読み上げにも知らせる)。 -->
    <p class="scenes__hint" role="status">
      {loading === null ? t("scenes.switchHint") : t("scenes.loadingStatus")}
    </p>
  {/if}
  <div class="scenes__actions">
    <Button small {disabled} onclick={onadd}>{t("scenes.add")}</Button>
    <Button small disabled={disabled || current === null} onclick={() => current && onduplicate(current.id)}>
      {t("scenes.duplicate")}
    </Button>
    <Button
      small
      disabled={disabled || current === null || index <= 0}
      onclick={() => current && onmove(current.id, index - 1)}
    >
      {t("scenes.moveUp")}
    </Button>
    <Button
      small
      disabled={disabled || current === null || index >= scenes.length - 1}
      onclick={() => current && onmove(current.id, index + 1)}
    >
      {t("scenes.moveDown")}
    </Button>
    <Button small variant="danger" disabled={disabled || current === null} onclick={() => (removing = true)}>
      {t("scenes.remove")}
    </Button>
  </div>
</section>

{#if current !== null}
  <AlertDialog
    bind:open={removing}
    title={t("scenes.removeTitle", { name: sceneLabel(current, index) })}
    description={t("scenes.removeBody")}
    confirmLabel={t("scenes.remove")}
    danger
    onconfirm={() => {
      removing = false;
      onremove(current.id);
    }}
  />
{/if}

<style>
  .scenes {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-3);
    padding: var(--ugoe-space-4) var(--ugoe-space-3);
    overflow: auto;
    background: var(--ugoe-color-surface);
    border-right: 1px solid var(--ugoe-color-border);
  }

  .scenes h2 {
    margin: 0;
    font-size: var(--ugoe-font-size-lg);
    font-weight: var(--ugoe-font-weight-strong);
  }

  .scenes__list {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-1);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .scenes__item {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 0 var(--ugoe-space-2);
    width: 100%;
    padding: var(--ugoe-space-2) var(--ugoe-space-3);
    border: 1px solid transparent;
    border-radius: var(--ugoe-radius-md);
    background: transparent;
    color: var(--ugoe-color-text);
    font: inherit;
    text-align: start;
    cursor: pointer;
  }

  .scenes__item:hover:not(:disabled) {
    background: var(--ugoe-color-sunken);
  }

  .scenes__item:focus-visible {
    outline: var(--ugoe-focus-ring);
    outline-offset: 2px;
  }

  .scenes__item--selected {
    border-color: var(--ugoe-color-control-border);
    background: var(--ugoe-color-accent-soft);
  }

  .scenes__item--live {
    border-color: var(--ugoe-color-accent);
  }

  .scenes__name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: var(--ugoe-font-weight-strong);
  }

  .scenes__model {
    grid-column: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--ugoe-color-text-muted);
    font-size: var(--ugoe-font-size-sm);
  }

  .scenes__live {
    grid-column: 2;
    grid-row: 1 / span 2;
    align-self: center;
    padding: 0 var(--ugoe-space-2);
    border-radius: var(--ugoe-radius-sm);
    background: var(--ugoe-color-accent);
    color: var(--ugoe-color-on-accent);
    font-size: var(--ugoe-font-size-sm);
  }

  .scenes__loading {
    grid-column: 2;
    grid-row: 1 / span 2;
    align-self: center;
    padding: 0 var(--ugoe-space-2);
    border: 1px solid var(--ugoe-color-control-border);
    border-radius: var(--ugoe-radius-sm);
    color: var(--ugoe-color-text-muted);
    font-size: var(--ugoe-font-size-sm);
  }

  .scenes__actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--ugoe-space-2);
  }

  .scenes__empty,
  .scenes__hint {
    margin: 0;
    color: var(--ugoe-color-text-muted);
    font-size: var(--ugoe-font-size-sm);
  }
</style>
