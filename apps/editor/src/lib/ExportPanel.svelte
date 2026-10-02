<script lang="ts">
import { Button, Checkbox, NumberField, Select } from "@ugoe/ui";
import { errorText } from "./errors";
// 書き出しの設定 (アトラス化の有無・最大の大きさ・余白) と、最後の書き出しの結果 (docs/editor.md#書き出し)。
// 設定は変えるたびにプロジェクトに残す (保存で project.json の editor.export に書く)。
// .ugoe の書き出しと、モジュールのプロジェクトの .ugoemod の書き出し (T0654) で同じ設定を使う。
import { ATLAS_SIZES, MAX_ATLAS_PADDING } from "./exportSummary";
import { t } from "./i18n";
import { type ExportSettings, ipc, type ProjectStatus } from "./ipc";

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

/** 最大の大きさの選択肢 (今の値が一覧に無ければ足す)。 */
const sizeOptions = $derived.by(() => {
  const sizes: number[] = [...ATLAS_SIZES];
  if (settings && !sizes.includes(settings.atlas_max_size)) sizes.push(settings.atlas_max_size);
  return sizes.map((size) => ({ value: String(size), label: t("export.px", { value: size }) }));
});

let settings = $state<ExportSettings | null>(null);
let error = $state<string | null>(null);

async function load() {
  try {
    settings = await ipc.getExportSettings();
  } catch (e) {
    error = errorText(e);
  }
}

async function update(patch: Partial<ExportSettings>) {
  if (!settings) return;
  const next = { ...settings, ...patch };
  try {
    onstatus(await ipc.setExportSettings(next));
    settings = next;
    error = null;
  } catch (e) {
    error = errorText(e);
  }
}

function setPadding(value: number) {
  if (Number.isInteger(value) && value >= 0 && value <= MAX_ATLAS_PADDING) {
    update({ atlas_padding: value });
  } else {
    error = t("export.padding.invalid", { max: MAX_ATLAS_PADDING });
  }
}

load();
</script>

<div class="export-panel" role="dialog" aria-label={t("export.aria")}>
  {#if settings}
    <Checkbox
      label={t("export.atlas.label")}
      checked={settings.atlas}
      disabled={busy}
      onchange={(atlas) => update({ atlas })}
    />
    <Select
      label={t("export.maxSize.label")}
      value={String(settings.atlas_max_size)}
      options={sizeOptions}
      disabled={busy || !settings.atlas}
      onchange={(v) => update({ atlas_max_size: Number(v) })}
    />
    <NumberField
      label={t("export.padding.label")}
      unit={t("export.padding.unit")}
      min={0}
      max={MAX_ATLAS_PADDING}
      step={1}
      size={4}
      value={settings.atlas_padding}
      disabled={busy || !settings.atlas}
      onchange={setPadding}
    />
    <p class="ed-hint">{t("export.hint")}</p>
  {/if}
  <div class="ed-buttons">
    <Button variant="primary" onclick={onexport} disabled={busy}>{label ?? t("fileMenu.export.bundle")}</Button>
  </div>
  {#if error}<p class="error ugoe-selectable">{error}</p>{/if}
  {#if summary}
    <ul class="summary" aria-label={t("export.summary.aria")}>
      {#each summary as line, i (i)}
        <li>{line}</li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .export-panel {
    position: absolute;
    z-index: 10;
    top: 100%;
    margin-top: var(--ugoe-space-2);
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-2);
    min-width: 280px;
    padding: var(--ugoe-space-3);
    background: var(--ugoe-color-surface);
    border: 1px solid var(--ugoe-color-border);
    border-radius: var(--ugoe-radius-md);
    box-shadow: var(--ugoe-shadow-overlay);
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
