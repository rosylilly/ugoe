// 書き出しの設定の欄 (ExportPanel.svelte) の状態 (docs/editor.md#書き出し)。exportSettings.test.ts で確かめる。
// 変えた値はすぐ欄に出し、プロジェクトに残す (`set_export_settings`) のは 1 つずつ順に行う。それぞれの変更は、
// その前の変更まで残した設定に重ねて送る (続けて変えても、前の変更が終わる前の古い設定を元に送って戻さない)。
import { errorText } from "./errors";
import { MAX_ATLAS_PADDING } from "./exportSummary";
import { t } from "./i18n";
import { type ExportSettings, ipc, type ProjectStatus } from "./ipc";

export class ExportSettingsForm {
  /** 欄に出す設定 (残した設定に、まだ残していない変更を重ねたもの)。読めるまで null。 */
  settings = $state<ExportSettings | null>(null);
  error = $state<string | null>(null);
  /** プロジェクトに残した設定。 */
  #saved: ExportSettings | null = null;
  /** まだ残していない変更 (送る順)。 */
  #pending: Partial<ExportSettings>[] = [];
  /** 送る変更の列 (前の変更が終わってから次を送る)。 */
  #queue: Promise<void> = Promise.resolve();
  #onstatus: (status: ProjectStatus) => void;

  /** `onstatus` は設定を残した後のプロジェクトの状態 (未保存の印)。 */
  constructor(onstatus: (status: ProjectStatus) => void) {
    this.#onstatus = onstatus;
  }

  async load(): Promise<void> {
    try {
      const settings = await ipc.getExportSettings();
      this.#saved = settings;
      this.settings = this.#shown();
    } catch (e) {
      this.error = errorText(e);
    }
  }

  /** 設定を変える。返す Promise は、この変更を残し終えたら (失敗しても) 解決する。 */
  update(patch: Partial<ExportSettings>): Promise<void> {
    if (!this.#saved) return Promise.resolve();
    // 前の失敗の文は、次に変えたときに消す (続けた変更が通っても、断られたことは見せる)
    this.error = null;
    this.#pending.push(patch);
    this.settings = this.#shown();
    this.#queue = this.#queue.then(() => this.#send(patch));
    return this.#queue;
  }

  /** 余白の欄: 範囲の外なら送らずに知らせる。 */
  setPadding(value: number): Promise<void> {
    if (Number.isInteger(value) && value >= 0 && value <= MAX_ATLAS_PADDING) {
      return this.update({ atlas_padding: value });
    }
    this.error = t("export.padding.invalid", { max: MAX_ATLAS_PADDING });
    return Promise.resolve();
  }

  async #send(patch: Partial<ExportSettings>): Promise<void> {
    if (!this.#saved) return;
    const next = { ...this.#saved, ...patch };
    try {
      const status = await ipc.setExportSettings(next);
      this.#saved = next;
      this.#onstatus(status);
    } catch (e) {
      // 断られた変更は欄から戻す (後に続く変更は残す)
      this.error = errorText(e);
    } finally {
      this.#pending.shift();
      this.settings = this.#shown();
    }
  }

  #shown(): ExportSettings | null {
    if (!this.#saved) return null;
    return Object.assign({ ...this.#saved }, ...this.#pending);
  }
}
