<script lang="ts">
import { Button, Checkbox, NumberField, Select } from "@ugoe/ui";
// 書き出しの設定 (アトラス化の有無・最大の大きさ・余白) と、最後の書き出しの結果 (docs/editor.md#書き出し)。
// 設定は変えるたびにプロジェクトに残す (保存で project.json の editor.export に書く。続けて変えても重ねない。
// 状態は exportSettings.svelte.ts の ExportSettingsForm)。
// .ugoe の書き出しと、モジュールのプロジェクトの .ugoemod の書き出し (T0654) で同じ設定を使う。
import { ExportSettingsForm } from "./exportSettings.svelte";
import { ATLAS_SIZES, MAX_ATLAS_PADDING } from "./exportSummary";
import { t } from "./i18n";
import type { ProjectStatus } from "./ipc";

interface Props {
  /** ダイアログを開いているか、その後の処理の途中か。 */
  busy: boolean;
  /** 最後の書き出しの結果の要約の行 (exportSummary・moduleExportSummary)。 */
  summary: string[] | null;
  /** 書き出すボタンの文言。 */
  label?: string;
  /** 書き出す (保存のダイアログを開く)。 */
  onexport: () => void;
  /** 設定を残した後のプロジェクトの状態 (未保存の印)。 */
  onstatus: (status: ProjectStatus) => void;
}

let { busy, summary, label, onexport, onstatus }: Props = $props();

const form = new ExportSettingsForm((status) => onstatus(status));

/** 最大の大きさの選択肢 (今の値が一覧に無ければ足す)。 */
const sizeOptions = $derived.by(() => {
  const sizes: number[] = [...ATLAS_SIZES];
  const settings = form.settings;
  if (settings && !sizes.includes(settings.atlas_max_size)) sizes.push(settings.atlas_max_size);
  return sizes.map((size) => ({ value: String(size), label: t("export.px", { value: size }) }));
});

form.load();
</script>

<div class="export-panel">
  {#if form.settings}
    <Checkbox
      label={t("export.atlas.label")}
      checked={form.settings.atlas}
      disabled={busy}
      onchange={(atlas) => form.update({ atlas })}
    />
    <Select
      label={t("export.maxSize.label")}
      value={String(form.settings.atlas_max_size)}
      options={sizeOptions}
      disabled={busy || !form.settings.atlas}
      onchange={(v) => form.update({ atlas_max_size: Number(v) })}
    />
    <NumberField
      label={t("export.padding.label")}
      unit={t("export.padding.unit")}
      min={0}
      max={MAX_ATLAS_PADDING}
      step={1}
      size={4}
      value={form.settings.atlas_padding}
      disabled={busy || !form.settings.atlas}
      onchange={(value) => form.setPadding(value)}
    />
    <p class="ed-hint">{t("export.hint")}</p>
  {/if}
  <div class="ed-buttons">
    <Button variant="primary" onclick={onexport} disabled={busy}>{label ?? t("fileMenu.export.bundle")}</Button>
  </div>
  {#if form.error}<p class="error ugoe-selectable">{form.error}</p>{/if}
  {#if summary}
    <ul class="summary" aria-label={t("export.summary.aria")}>
      {#each summary as line, i (i)}
        <li>{line}</li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  /* 枠 (影・角丸・読み上げの名前) は FileMenu の Popover が付ける */
  .export-panel {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-2);
    min-width: 280px;
    max-width: 360px;
  }
  .error {
    margin: 0;
    color: var(--ugoe-color-danger);
  }
  .summary {
    margin: 0;
    padding: var(--ugoe-space-2) 0 0 18px;
    border-top: 1px solid var(--ugoe-color-border);
  }
</style>
