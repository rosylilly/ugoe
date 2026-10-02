<!--
  モジュールのプロジェクトの本体のファイルが、開いた (読み直した) 後で変わったときの確認 (docs/editor.md#本体の変化)。
  FileMenu が `change` を渡すと開く。
  - 本体が変わった: 本体を読み直す (保存していないモジュールの編集は保つ。Undo の履歴は消える)・後で。
    「Reload automatically」(外での書き換えと同じ設定) を入れると、次からは尋ねずに読み直す。
  - 本体が見つからない: 本体を選び直す (開き直さないので、保存していない編集は残る)・後で
  Esc・閉じるボタンは「後で」。
-->
<script lang="ts">
import { RichText } from "@ugoe/i18n";
import { Button, Checkbox, Dialog } from "@ugoe/ui";
import { describeFiles } from "./externalChange";
import { t } from "./i18n";
import type { ExternalChange } from "./ipc";
import { baseFileName } from "./moduleProject";
import TipButton from "./ui/TipButton.svelte";

interface Props {
  change: ExternalChange | null;
  /** 本体のファイルの場所。 */
  base: string;
  /** 外での書き換えと同じ「自動で読み直す」設定。 */
  autoReload: boolean;
  onautoreload: (enabled: boolean) => void;
  onreload: () => void;
  /** 本体を選び直す (見つからないとき)。 */
  onlocate: (kind: "project" | "bundle") => void;
  ondismiss: () => void;
}

let { change, base, autoReload, onautoreload, onreload, onlocate, ondismiss }: Props = $props();
</script>

<Dialog
  open={change !== null}
  title={change?.missing ? t("baseChange.missing.title") : t("baseChange.changed.title")}
  onclose={ondismiss}
>
  {#if change}
    {#if change.missing}
      <p>
        <RichText text={t("baseChange.missing.description", { path: base })}>
          {#snippet code(text: string)}<code>{text}</code>{/snippet}
        </RichText>
      </p>
    {:else}
      <p>
        {change.files.length > 0
          ? t("baseChange.changed.descriptionFiles", {
              name: baseFileName(base),
              files: describeFiles(change.files),
            })
          : t("baseChange.changed.description", { name: baseFileName(base) })}
      </p>
      <p class="muted">{t("baseChange.changed.hint")}</p>
      <Checkbox
        checked={autoReload}
        label={t("baseChange.autoReload.label")}
        onchange={onautoreload}
      />
    {/if}
  {/if}
  {#snippet actions()}
    <TipButton tooltip={t("baseChange.later.tooltip")} onclick={ondismiss}>
      {t("baseChange.later.label")}
    </TipButton>
    {#if change?.missing}
      <Button onclick={() => onlocate("bundle")}>{t("baseChange.locateBundle.label")}</Button>
      <Button variant="primary" onclick={() => onlocate("project")}>
        {t("baseChange.locateProject.label")}
      </Button>
    {:else}
      <Button variant="primary" onclick={onreload}>{t("baseChange.reload.label")}</Button>
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
  .muted {
    color: var(--ugoe-color-text-muted);
  }
</style>
