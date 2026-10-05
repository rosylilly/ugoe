<!--
  シーンの一覧 (docs/studio.md#操作ウィンドウ の左)。押すとそのシーンに切り替え、編集の対象にする。
  追加・複製・並べ替え・削除。配信しているシーンと、読み込んでいるシーンに印を付け、切り替えのホットキーを並べて示す。
  操作のボタンは狭い列でも 1 行に収まるよう、追加のほかはアイコンにする (名前は Tooltip。T0817)。削除は確かめてから行う。
-->
<script lang="ts">
import { AlertDialog, Button, Icon, IconButton, Kbd } from "@ugoe/ui";
import { displayKeys } from "./lib/hotkeys";
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
  /** シーンの ID → 切り替えのホットキー (`Ctrl+Shift+Digit1` の形。割り当てたものだけ)。 */
  hotkeys?: ReadonlyMap<string, string>;
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
  hotkeys = new Map(),
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
            <span class="scenes__row">
              <span class="scenes__name">{sceneLabel(scene, i)}</span>
              {#if scene.id === loading?.scene}
                <span class="scenes__loading">{t("scenes.loading")}</span>
              {:else if scene.id === live}
                <span class="scenes__live">{t("scenes.live")}</span>
              {/if}
            </span>
            <span class="scenes__row">
              <span class="scenes__model">
                {scene.model === null ? t("scenes.noModel") : fileName(scene.model)}
              </span>
              {#if hotkeys.has(scene.id)}
                <span class="scenes__key"><Kbd keys={displayKeys(hotkeys.get(scene.id) ?? "")} /></span>
              {/if}
            </span>
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
    <Button {disabled} onclick={onadd}><Icon name="plus" />{t("scenes.add")}</Button>
    <span class="scenes__spacer"></span>
    <IconButton
      label={t("scenes.duplicate")}
      disabled={disabled || current === null}
      onclick={() => current && onduplicate(current.id)}
    >
      <!-- 複製: 重ねた 2 枚 (Icon に無い 1 か所だけの印) -->
      <svg class="ugoe-icon" viewBox="0 0 16 16" aria-hidden="true">
        <rect x="5.5" y="5.5" width="8" height="8" rx="1" /><path d="M10.5 3.5v-.75a.75.75 0 0 0-.75-.75h-6.5a.75.75 0 0 0-.75.75v6.5c0 .41.34.75.75.75h.75" />
      </svg>
    </IconButton>
    <IconButton
      label={t("scenes.moveUp")}
      disabled={disabled || current === null || index <= 0}
      onclick={() => current && onmove(current.id, index - 1)}
    >
      <Icon name="arrowUp" />
    </IconButton>
    <IconButton
      label={t("scenes.moveDown")}
      disabled={disabled || current === null || index >= scenes.length - 1}
      onclick={() => current && onmove(current.id, index + 1)}
    >
      <Icon name="arrowDown" />
    </IconButton>
    <IconButton
      label={t("scenes.removeLabel")}
      variant="danger"
      disabled={disabled || current === null}
      onclick={() => (removing = true)}
    >
      <!-- 削除: ごみ箱 (Icon に無い 1 か所だけの印) -->
      <svg class="ugoe-icon" viewBox="0 0 16 16" aria-hidden="true">
        <path d="M2.75 4.25h10.5M6.25 4.25V2.75h3.5v1.5M4.25 4.25l.6 9h6.3l.6-9M6.75 6.75v4M9.25 6.75v4" />
      </svg>
    </IconButton>
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
    display: flex;
    flex-direction: column;
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
    background-image: var(--ugoe-overlay-hover);
  }

  .scenes__item:active:not(:disabled) {
    background-image: var(--ugoe-overlay-active);
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

  /* 1 行目は名前と配信中の印、2 行目はモデルとホットキー。名前とモデルを省略し、印とキーは縮めない */
  .scenes__row {
    display: flex;
    align-items: center;
    gap: var(--ugoe-space-2);
    min-width: 0;
  }

  .scenes__name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: var(--ugoe-font-weight-strong);
  }

  .scenes__model {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--ugoe-color-text-muted);
    font-size: var(--ugoe-font-size-sm);
  }

  .scenes__key {
    flex: none;
    color: var(--ugoe-color-text-muted);
    font-size: var(--ugoe-font-size-sm);
  }

  .scenes__live {
    flex: none;
    padding: 0 var(--ugoe-space-2);
    border-radius: var(--ugoe-radius-sm);
    background: var(--ugoe-color-accent);
    color: var(--ugoe-color-on-accent);
    font-size: var(--ugoe-font-size-sm);
  }

  .scenes__loading {
    flex: none;
    padding: 0 var(--ugoe-space-2);
    border: 1px solid var(--ugoe-color-control-border);
    border-radius: var(--ugoe-radius-sm);
    color: var(--ugoe-color-text-muted);
    font-size: var(--ugoe-font-size-sm);
  }

  .scenes__actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--ugoe-space-1);
  }

  .scenes__spacer {
    flex: 1;
  }

  .scenes__empty,
  .scenes__hint {
    margin: 0;
    color: var(--ugoe-color-text-muted);
    font-size: var(--ugoe-font-size-sm);
  }
</style>
