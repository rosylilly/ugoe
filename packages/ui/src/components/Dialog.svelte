<!--
  Dialog: 作業を止めて確認・入力させるモーダル (docs/ui.md#部品の一覧)。土台は Bits UI の Dialog
  (中でフォーカスを回す・Esc と外のクリックで閉じる・閉じたら開く前にフォーカスがあった要素へ戻す)。
  取り消せない操作の確認は AlertDialog を使う。利用者の判断が要らない知らせには使わない (Notice)。

  - 開くとフォーカスは中の最初の操作できる要素へ移る。閉じるボタンは右上に出すが、並びの最後に置く
    (最初の入力欄や決定のボタンに先にフォーカスが行くように)。
  - `onclose` は利用者が閉じたとき (Esc・外のクリック・閉じるボタン) に呼ぶ。`open` を外から false にしたときは呼ばない。
-->
<script lang="ts">
import { Dialog } from "bits-ui";
import type { Snippet } from "svelte";
import { t } from "../i18n";
import Icon from "./Icon.svelte";
import IconButton from "./IconButton.svelte";

interface Props {
  /** 開いているか。 */
  open?: boolean;
  /** 見出し (翻訳を通した文言)。読み上げの名前にもなる。 */
  title: string;
  /** 見出しの下の説明。読み上げの説明 (aria-describedby) になる。 */
  description?: string;
  /** 幅。既定は `md`。 */
  size?: "sm" | "md" | "lg";
  /** Esc・外のクリック・閉じるボタンで閉じられるか。既定は true。false なら中のボタンで閉じる。 */
  dismissible?: boolean;
  /** 閉じるボタンの名前。省くと @ugoe/ui のカタログの「閉じる」(`common.close`)。 */
  closeLabel?: string;
  /** 利用者が閉じたとき。 */
  onclose?: () => void;
  children?: Snippet;
  /** 下に並べる操作のボタン (決定・取り消し)。決定は右端に置く。 */
  actions?: Snippet;
}

let {
  open = $bindable(false),
  title,
  description,
  size = "md",
  dismissible = true,
  closeLabel,
  onclose,
  children,
  actions,
}: Props = $props();
</script>

<Dialog.Root
  {open}
  onOpenChange={(v) => {
    open = v;
    if (!v) onclose?.();
  }}
>
  <Dialog.Portal>
    <Dialog.Overlay class="ugoe-dialog__overlay" />
    <Dialog.Content
      class={`ugoe-dialog ugoe-dialog--${size}${dismissible ? " ugoe-dialog--dismissible" : ""}`}
      escapeKeydownBehavior={dismissible ? "close" : "ignore"}
      interactOutsideBehavior={dismissible ? "close" : "ignore"}
    >
      <Dialog.Title class="ugoe-dialog__title" level={2}>{title}</Dialog.Title>
      {#if description}
        <Dialog.Description class="ugoe-dialog__description">{description}</Dialog.Description>
      {/if}
      {#if children}
        <div class="ugoe-dialog__body">{@render children()}</div>
      {/if}
      {#if actions}
        <div class="ugoe-dialog__actions">{@render actions()}</div>
      {/if}
      {#if dismissible}
        <IconButton
          label={closeLabel ?? t("common.close")}
          tooltip={false}
          small
          class="ugoe-dialog__close"
          onclick={() => {
            open = false;
            onclose?.();
          }}
        >
          <Icon name="close" />
        </IconButton>
      {/if}
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global {
    .ugoe-dialog__overlay {
      position: fixed;
      inset: 0;
      z-index: var(--ugoe-z-popover);
      background: var(--ugoe-color-backdrop);
      animation: ugoe-fade-in var(--ugoe-duration-normal) ease-out;
    }

    .ugoe-dialog {
      position: fixed;
      top: 50%;
      left: 50%;
      z-index: var(--ugoe-z-popover);
      display: flex;
      flex-direction: column;
      gap: var(--ugoe-space-3);
      box-sizing: border-box;
      width: min(var(--ugoe-dialog-width), calc(100vw - 2 * var(--ugoe-space-4)));
      max-height: calc(100vh - 2 * var(--ugoe-space-4));
      padding: var(--ugoe-space-5);
      overflow: auto;
      border: 1px solid var(--ugoe-color-border);
      border-radius: var(--ugoe-radius-lg);
      background: var(--ugoe-color-surface);
      box-shadow: var(--ugoe-shadow-overlay);
      color: var(--ugoe-color-text);
      font-family: var(--ugoe-font-sans);
      font-size: var(--ugoe-font-size-md);
      line-height: var(--ugoe-line-height);
      transform: translate(-50%, -50%);
      animation: ugoe-fade-in var(--ugoe-duration-normal) ease-out;
    }

    .ugoe-dialog--sm {
      --ugoe-dialog-width: 360px;
    }

    .ugoe-dialog--md {
      --ugoe-dialog-width: 480px;
    }

    .ugoe-dialog--lg {
      --ugoe-dialog-width: 720px;
    }

    .ugoe-dialog:focus-visible {
      outline: var(--ugoe-focus-ring);
      outline-offset: var(--ugoe-focus-offset);
    }

    .ugoe-dialog__title {
      margin: 0;
      font-size: var(--ugoe-font-size-lg);
      font-weight: var(--ugoe-font-weight-strong);
      line-height: var(--ugoe-line-height-control);
    }

    /* 閉じるボタンと重ならないように */
    .ugoe-dialog--dismissible .ugoe-dialog__title {
      padding-right: calc(var(--ugoe-target-min) + var(--ugoe-space-2));
    }

    .ugoe-dialog__description {
      margin: 0;
      color: var(--ugoe-color-text-muted);
    }

    .ugoe-dialog__actions {
      display: flex;
      flex-wrap: wrap;
      justify-content: flex-end;
      gap: var(--ugoe-space-2);
      margin-top: var(--ugoe-space-2);
    }

    .ugoe-dialog .ugoe-dialog__close {
      position: absolute;
      top: var(--ugoe-space-4);
      right: var(--ugoe-space-4);
    }
  }
</style>
