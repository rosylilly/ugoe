<!--
  「いまの状態」の印 1 つ (docs/studio.md#操作ウィンドウ、T0817)。色だけで伝えないよう、印の形 (動いている = 丸にチェック・切 = 中の空いた丸・
  注意 = 三角・動かせない = 八角) と短い文を並べ、意味は Tooltip で出す (キーボードでも出せるよう、印をフォーカスできるようにする)。
  中身 (文と説明) は lib/status.ts。
-->
<script lang="ts">
import { Icon, Tooltip } from "@ugoe/ui";
import type { Snippet } from "svelte";
import type { StatusBadgeView } from "./lib/status";

interface Props {
  view: StatusBadgeView;
  /** 文の後ろに足すもの (口パクの音量のメーターなど)。 */
  children?: Snippet;
}

let { view, children }: Props = $props();
</script>

<Tooltip text={view.tooltip}>
  {#snippet trigger(attrs)}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <span {...attrs} class="status-badge" data-tone={view.tone} data-status={view.id} tabindex="0">
      <span class="status-badge__icon">
        {#if view.tone === "ok"}
          <Icon name="success" />
        {:else if view.tone === "warning"}
          <Icon name="warning" />
        {:else if view.tone === "danger"}
          <Icon name="danger" />
        {:else}
          <!-- 切: 中の空いた丸に横線 (Icon に無い 1 か所だけの印。docs/ui.md#アイコン) -->
          <svg class="ugoe-icon" viewBox="0 0 16 16" aria-hidden="true">
            <circle cx="8" cy="8" r="6.25" /><path d="M5.25 8h5.5" />
          </svg>
        {/if}
      </span>
      <span class="status-badge__label">{view.label}</span>
      {@render children?.()}
    </span>
  {/snippet}
</Tooltip>

<style>
  .status-badge {
    display: inline-flex;
    align-items: center;
    gap: var(--ugoe-space-1);
    min-height: var(--ugoe-target-min);
    box-sizing: border-box;
    padding: 0 var(--ugoe-space-2);
    border: 1px solid var(--ugoe-color-border);
    border-radius: var(--ugoe-radius-md);
    background: var(--ugoe-color-surface);
    color: var(--ugoe-color-text);
    font-size: var(--ugoe-font-size-sm);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .status-badge:focus-visible {
    outline: var(--ugoe-focus-ring);
    outline-offset: var(--ugoe-focus-offset);
  }

  .status-badge__icon {
    display: inline-flex;
    color: var(--ugoe-color-text-muted);
  }

  .status-badge[data-tone="ok"] .status-badge__icon {
    color: var(--ugoe-color-success);
  }

  .status-badge[data-tone="warning"] {
    border-color: var(--ugoe-color-warning);
    background: var(--ugoe-color-warning-soft);
  }

  .status-badge[data-tone="warning"] .status-badge__icon {
    color: var(--ugoe-color-warning);
  }

  .status-badge[data-tone="danger"] {
    border-color: var(--ugoe-color-danger);
    background: var(--ugoe-color-danger-soft);
  }

  .status-badge[data-tone="danger"] .status-badge__icon {
    color: var(--ugoe-color-danger);
  }

  .status-badge[data-tone="off"] .status-badge__label {
    color: var(--ugoe-color-text-muted);
  }
</style>
