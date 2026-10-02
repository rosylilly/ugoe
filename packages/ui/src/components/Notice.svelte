<!--
  Notice: 画面に残る知らせ (読み込みの警告など) (docs/ui.md#部品の一覧)。自作。
  - `role="status"` (エラーの `danger` は `role="alert"`)。描いた時点の中身が読み上げられる。
  - 種類は色に加えて印と読み上げの言葉 (「警告:」など) で示す (色だけで伝えない)。
  - エラー (`danger`) のときだけ、見出しと本文 (`.ugoe-notice__body`) を選択できる (`.ugoe-selectable`)。
  - `ondismiss` を渡すと閉じるボタンを出す。消すのは呼び手 (Notice を描かなくする)。
-->
<script lang="ts">
import type { Snippet } from "svelte";
import { t } from "../i18n";
import type { Tone } from "../types";
import Icon from "./Icon.svelte";
import IconButton from "./IconButton.svelte";

interface Props {
  tone?: Tone;
  /** 太字の 1 行目 (省ける)。 */
  title?: string;
  /** 本文。 */
  children?: Snippet;
  /** 本文の後ろの操作のボタン (「読み直す」など)。 */
  actions?: Snippet;
  /** 閉じるボタンを押したとき。渡さなければ閉じるボタンを出さない。 */
  ondismiss?: () => void;
  /** 閉じるボタンの名前。省くと @ugoe/ui のカタログの「閉じる」(`common.close`)。 */
  dismissLabel?: string;
}

let { tone = "info", title, children, actions, ondismiss, dismissLabel }: Props = $props();

/** エラーの文は写して報告に貼れるよう、選択できるようにする (docs/ui.md#文字の選択)。 */
const selectable = $derived(tone === "danger");
</script>

<div class="ugoe-notice" data-tone={tone} role={tone === "danger" ? "alert" : "status"}>
  <span class="ugoe-notice__icon"><Icon name={tone} /></span>
  <div class="ugoe-notice__body" class:ugoe-selectable={selectable}>
    <span class="ugoe-visually-hidden">{t(`notice.tone.${tone}`)}</span>
    {#if title}<p class="ugoe-notice__title">{title}</p>{/if}
    {#if children}<div class="ugoe-notice__text">{@render children()}</div>{/if}
    {#if actions}<div class="ugoe-notice__actions">{@render actions()}</div>{/if}
  </div>
  {#if ondismiss}
    <IconButton label={dismissLabel ?? t("common.close")} small onclick={ondismiss}>
      <Icon name="close" />
    </IconButton>
  {/if}
</div>

<style>
  :global {
    .ugoe-notice {
      display: flex;
      align-items: flex-start;
      gap: var(--ugoe-space-3);
      box-sizing: border-box;
      padding: var(--ugoe-space-3) var(--ugoe-space-3) var(--ugoe-space-3) var(--ugoe-space-4);
      border: 1px solid var(--ugoe-notice-color);
      border-left-width: 4px;
      border-radius: var(--ugoe-radius-lg);
      background: var(--ugoe-notice-soft);
      color: var(--ugoe-color-text);
      font-family: var(--ugoe-font-sans);
      font-size: var(--ugoe-font-size-md);
      line-height: var(--ugoe-line-height);
    }

    .ugoe-notice[data-tone="info"] {
      --ugoe-notice-color: var(--ugoe-color-accent);
      --ugoe-notice-soft: var(--ugoe-color-accent-soft);
    }

    .ugoe-notice[data-tone="success"] {
      --ugoe-notice-color: var(--ugoe-color-success);
      --ugoe-notice-soft: var(--ugoe-color-success-soft);
    }

    .ugoe-notice[data-tone="warning"] {
      --ugoe-notice-color: var(--ugoe-color-warning);
      --ugoe-notice-soft: var(--ugoe-color-warning-soft);
    }

    .ugoe-notice[data-tone="danger"] {
      --ugoe-notice-color: var(--ugoe-color-danger);
      --ugoe-notice-soft: var(--ugoe-color-danger-soft);
    }

    .ugoe-notice__icon {
      display: inline-flex;
      /* 1 行目の文字の高さにそろえる */
      height: calc(1em * var(--ugoe-line-height));
      align-items: center;
      color: var(--ugoe-notice-color);
    }

    .ugoe-notice__body {
      display: flex;
      flex: 1;
      flex-direction: column;
      gap: var(--ugoe-space-1);
      min-width: 0;
      overflow-wrap: anywhere;
    }

    .ugoe-notice__title {
      margin: 0;
      font-weight: var(--ugoe-font-weight-strong);
    }

    .ugoe-notice__text > :global(p) {
      margin: 0;
    }

    .ugoe-notice__actions {
      display: flex;
      flex-wrap: wrap;
      gap: var(--ugoe-space-2);
      margin-top: var(--ugoe-space-2);
    }
  }
</style>
