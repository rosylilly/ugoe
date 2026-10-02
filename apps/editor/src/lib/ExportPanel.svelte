<script lang="ts">
import { errorText } from "./errors";
// 書き出しの設定 (アトラス化の有無・最大の大きさ・余白) と、最後の書き出しの結果 (docs/editor.md#書き出し)。
// 設定は変えるたびにプロジェクトに残す (保存で project.json の editor.export に書く)。
// .ugoe の書き出しと、モジュールのプロジェクトの .ugoemod の書き出し (T0654) で同じ設定を使う。
import { ATLAS_SIZES, MAX_ATLAS_PADDING } from "./exportSummary";
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

let { busy, summary, label = "Export .ugoe…", onexport, onstatus }: Props = $props();

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
    error = `Padding must be an integer from 0 to ${MAX_ATLAS_PADDING}`;
  }
}

load();
</script>

<div class="export-panel" role="dialog" aria-label="Export settings">
  {#if settings}
    <label class="check">
      <input
        type="checkbox"
        checked={settings.atlas}
        disabled={busy}
        onchange={(e) => update({ atlas: e.currentTarget.checked })}
      />
      Pack textures into atlases
    </label>
    <label class="field">
      Max size
      <select
        value={settings.atlas_max_size}
        disabled={busy || !settings.atlas}
        onchange={(e) => update({ atlas_max_size: Number(e.currentTarget.value) })}
      >
        {#each ATLAS_SIZES as size (size)}
          <option value={size}>{size} px</option>
        {/each}
        {#if !ATLAS_SIZES.some((s) => s === settings?.atlas_max_size)}
          <option value={settings.atlas_max_size}>{settings.atlas_max_size} px</option>
        {/if}
      </select>
    </label>
    <label class="field">
      Padding
      <input
        type="number"
        min="0"
        max={MAX_ATLAS_PADDING}
        step="1"
        value={settings.atlas_padding}
        disabled={busy || !settings.atlas}
        onchange={(e) => setPadding(e.currentTarget.valueAsNumber)}
      />
      px
    </label>
    <p class="hint">Settings are kept in the project. Identical images are packed once.</p>
  {/if}
  <button class="primary" onclick={onexport} disabled={busy}>{label}</button>
  {#if error}<p class="error">{error}</p>{/if}
  {#if summary}
    <ul class="summary" aria-label="Last export">
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
    margin-top: 4px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 280px;
    padding: 8px 10px;
    background: var(--panel);
    border: 1px solid var(--border);
    box-shadow: 0 4px 12px rgb(0 0 0 / 0.2);
  }
  .check,
  .field {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .check input {
    width: auto;
    margin: 0;
  }
  .field input {
    width: 5em;
  }
  .hint {
    margin: 0;
    opacity: 0.7;
    font-size: 0.9em;
  }
  .primary {
    align-self: flex-start;
  }
  .error {
    margin: 0;
    color: #dc2626;
  }
  .summary {
    margin: 0;
    padding: 6px 0 0 18px;
    border-top: 1px solid var(--border);
  }
</style>
