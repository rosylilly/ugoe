<!--
  AlertDialog: 取り消せない操作の確認 (docs/ui.md#使い分けの約束)。土台は Bits UI の AlertDialog
  (外のクリックでは閉じない。Esc と取り消しのボタンで閉じる)。
  開いたときのフォーカスは取り消し側に置く (うっかり Enter で決定しないように)。

  - 決定のボタンを押すと `onconfirm` を呼んで閉じる。
  - 取り消し (ボタン・Esc) のときは `oncancel` を呼んで閉じる。
-->
<script lang="ts">
import { AlertDialog } from "bits-ui";
import type { Snippet } from "svelte";
import { t } from "../i18n";
import Button from "./Button.svelte";

interface Props {
  open?: boolean;
  /** 見出し (翻訳を通した文言。何をするかを問う形)。 */
  title: string;
  /** 何が起きるか・取り消せないこと。 */
  description: string;
  /** 決定のボタンの文言 (動詞。「すべて忘れる」など)。 */
  confirmLabel: string;
  /** 取り消しのボタンの文言。省くと @ugoe/ui のカタログの「キャンセル」(`common.cancel`)。 */
  cancelLabel?: string;
  /** 決定を `danger` のボタンにする。既定は true (取り消せない操作の確認なので)。 */
  danger?: boolean;
  onconfirm: () => void;
  oncancel?: () => void;
  /** 説明の下に足す中身 (消えるものの一覧など)。 */
  children?: Snippet;
}

let {
  open = $bindable(false),
  title,
  description,
  confirmLabel,
  cancelLabel,
  danger = true,
  onconfirm,
  oncancel,
  children,
}: Props = $props();

const id = $props.id();
</script>

<AlertDialog.Root
  {open}
  onOpenChange={(v) => {
    open = v;
    if (!v) oncancel?.();
  }}
>
  <AlertDialog.Portal>
    <AlertDialog.Overlay class="ugoe-dialog__overlay" />
    <AlertDialog.Content
      class="ugoe-dialog ugoe-dialog--sm"
      onOpenAutoFocus={(event) => {
        event.preventDefault();
        document.getElementById(`${id}-cancel`)?.focus();
      }}
    >
      <AlertDialog.Title class="ugoe-dialog__title" level={2}>{title}</AlertDialog.Title>
      <AlertDialog.Description class="ugoe-dialog__description">{description}</AlertDialog.Description>
      {#if children}
        <div class="ugoe-dialog__body">{@render children()}</div>
      {/if}
      <div class="ugoe-dialog__actions">
        <AlertDialog.Cancel id={`${id}-cancel`}>
          {#snippet child({ props })}
            <Button {...props}>{cancelLabel ?? t("common.cancel")}</Button>
          {/snippet}
        </AlertDialog.Cancel>
        <Button
          variant={danger ? "danger" : "primary"}
          onclick={() => {
            open = false;
            onconfirm();
          }}>{confirmLabel}</Button
        >
      </div>
    </AlertDialog.Content>
  </AlertDialog.Portal>
</AlertDialog.Root>
