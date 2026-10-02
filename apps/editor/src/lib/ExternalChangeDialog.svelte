<!--
  開いているプロジェクトがエディタの外で書き換えられたときの確認 (docs/editor.md#外での書き換え)。
  FileMenu が `change` を渡すと開く。選べること:
  - 未保存の変更がない: 開き直す・今のまま (後で)
  - 未保存の変更がある (または保存しようとした): 開き直す (自分の変更を捨てる)・自分の版で上書き・別名で保存・後で
  - プロジェクトが消された: その場所に保存し直す・別名で保存・後で
  Esc・閉じるボタンは「後で」。
-->
<script lang="ts">
import { RichText } from "@ugoe/i18n";
import { Button, Checkbox, Dialog } from "@ugoe/ui";
import { describeFiles } from "./externalChange";
import { t } from "./i18n";
import type { ExternalChange } from "./ipc";
import TipButton from "./ui/TipButton.svelte";

interface Props {
  change: ExternalChange | null;
  /** 未保存の変更があるか。 */
  modified: boolean;
  /** 保存しようとして見つかったか (上書きを選べるようにする)。 */
  saving: boolean;
  /** 未保存の変更がなければ自動で開き直す設定。 */
  autoReload: boolean;
  onautoreload: (enabled: boolean) => void;
  onreload: () => void;
  onoverwrite: () => void;
  onsaveas: () => void;
  ondismiss: () => void;
}

let {
  change,
  modified,
  saving,
  autoReload,
  onautoreload,
  onreload,
  onoverwrite,
  onsaveas,
  ondismiss,
}: Props = $props();

const canOverwrite = $derived(modified || saving);
</script>

<Dialog
  open={change !== null}
  title={change?.missing ? t("externalChange.missing.title") : t("externalChange.changed.title")}
  onclose={ondismiss}
>
  {#if change}
    {#if change.missing}
      <p>{t("externalChange.missing.description")}</p>
    {:else}
      <p>
        <RichText text={t("externalChange.changed.description", { files: describeFiles(change.files) })}>
          {#snippet code(text: string)}<code>{text}</code>{/snippet}
        </RichText>
      </p>
      {#if modified}
        <p class="warn">{t("externalChange.changed.modifiedWarning")}</p>
      {:else if saving}
        <p class="warn">{t("externalChange.changed.savingWarning")}</p>
      {/if}
      <Checkbox
        checked={autoReload}
        label={t("externalChange.autoReload.label")}
        onchange={onautoreload}
      />
    {/if}
  {/if}
  {#snippet actions()}
    <TipButton tooltip={t("externalChange.later.tooltip")} onclick={ondismiss}>
      {t("externalChange.later.label")}
    </TipButton>
    {#if canOverwrite || change?.missing}
      <Button onclick={onsaveas}>{t("externalChange.saveAs.label")}</Button>
    {/if}
    {#if change?.missing}
      <Button variant="primary" onclick={onoverwrite}>{t("externalChange.saveHere.label")}</Button>
    {:else}
      {#if canOverwrite}
        <Button variant="danger" onclick={onoverwrite}>{t("externalChange.overwrite.label")}</Button>
      {/if}
      {#if modified}
        <Button variant="danger" onclick={onreload}>{t("externalChange.reloadDiscard.label")}</Button>
      {:else}
        <Button variant="primary" onclick={onreload}>{t("externalChange.reload.label")}</Button>
      {/if}
    {/if}
  {/snippet}
</Dialog>

<style>
  p {
    margin: 0;
  }
  code {
    overflow-wrap: anywhere;
  }
  .warn {
    color: var(--ugoe-color-warning);
  }
</style>
