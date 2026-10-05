<!--
  Section: 折りたためるまとまり (パネルの中。docs/ui.md#部品の一覧)。土台は Bits UI の Collapsible
  (見出しの中のボタンで開閉する。Enter・Space)。
-->
<script lang="ts">
import { Collapsible } from "bits-ui";
import type { Snippet } from "svelte";

interface Props {
  /** 見出し。翻訳を通した文言。 */
  title: string;
  /** 開いているか。既定は true。 */
  open?: boolean;
  /** 見出しの段 (h2〜h6)。既定は 3。 */
  level?: 2 | 3 | 4 | 5 | 6;
  /** 見出しの右に出すもの (ボタンなど。開閉のボタンの外に置く)。 */
  actions?: Snippet;
  children: Snippet;
  onchange?: (open: boolean) => void;
}

let { title, open = $bindable(true), level = 3, actions, children, onchange }: Props = $props();
</script>

<Collapsible.Root
  class="ugoe-section"
  bind:open
  onOpenChange={(v) => {
    onchange?.(v);
  }}
>
  <div class="ugoe-section__header">
    <svelte:element this={`h${level}`} class="ugoe-section__heading">
      <Collapsible.Trigger class="ugoe-section__trigger">
        <svg class="ugoe-icon ugoe-section__chevron" viewBox="0 0 16 16" aria-hidden="true">
          <path d="M6 4l4 4-4 4" />
        </svg>
        <span>{title}</span>
      </Collapsible.Trigger>
    </svelte:element>
    {#if actions}
      <div class="ugoe-section__actions">{@render actions()}</div>
    {/if}
  </div>
  <Collapsible.Content class="ugoe-section__content">
    {@render children()}
  </Collapsible.Content>
</Collapsible.Root>

<style>
  :global {
    .ugoe-section {
      color: var(--ugoe-color-text);
      font-family: var(--ugoe-font-sans);
      font-size: var(--ugoe-font-size-md);
      line-height: var(--ugoe-line-height);
    }

    .ugoe-section__header {
      display: flex;
      align-items: center;
      gap: var(--ugoe-space-2);
    }

    .ugoe-section__heading {
      flex: 1;
      min-width: 0;
      margin: 0;
      font-size: var(--ugoe-font-size-sm);
      font-weight: var(--ugoe-font-weight-strong);
    }

    .ugoe-section__trigger {
      display: flex;
      align-items: center;
      gap: var(--ugoe-space-1);
      box-sizing: border-box;
      width: 100%;
      min-height: max(var(--ugoe-control-height), var(--ugoe-target-min));
      padding: 0;
      border: 0;
      border-radius: var(--ugoe-radius-sm);
      background: transparent;
      color: var(--ugoe-color-text);
      font: inherit;
      text-align: left;
      cursor: pointer;
    }

    .ugoe-section__trigger:hover {
      background-image: var(--ugoe-overlay-hover);
    }

    .ugoe-section__trigger:active {
      background-image: var(--ugoe-overlay-active);
    }

    .ugoe-section__trigger:focus-visible {
      outline: var(--ugoe-focus-ring);
      outline-offset: var(--ugoe-focus-offset);
    }

    .ugoe-section__chevron {
      color: var(--ugoe-color-text-muted);
      transition: transform var(--ugoe-duration-fast);
    }

    .ugoe-section__trigger[data-state="open"] .ugoe-section__chevron {
      transform: rotate(90deg);
    }

    .ugoe-section__actions {
      display: flex;
      align-items: center;
      gap: var(--ugoe-space-1);
    }

    .ugoe-section__content {
      padding: var(--ugoe-space-2) 0 var(--ugoe-space-3);
    }
  }
</style>
