// バックエンドが持つドキュメント (モデルの正となる状態) の、フロントエンド側の写し。
// 編集は必ず IPC の Command で行い、その後 refresh() で読み直す (docs/editor.md)。
import { isEmptyModel } from "./emptyState";
import { type Command, ipc, type Model, type Parameter, type RederiveWarning } from "./ipc";
import { mergeWarnings } from "./panels/inspector/inference/inference";
import type { ViewportModel } from "./viewport/controller.svelte";

/** パネルが使うパラメータの定義 (ugoe_model::Parameter)。 */
export type ParameterDef = Parameter;

export class EditorDocument {
  /** ビューポートに渡すモデル (JSON と、テクスチャ付きの .ugoe バンドル)。refresh のたびに作り直す。 */
  model = $state.raw<ViewportModel | null>(null);
  parameters = $state<ParameterDef[]>([]);
  error = $state<string | null>(null);
  /**
   * 適用したコマンドの自動の作り直しで失敗したこと (ミラーのやり直しなど。docs/editor.md#推定とミラー)。
   * 利用者が閉じるまで残す。同じ格子とミラー元の組は新しいもので置き換える。
   */
  warnings = $state<RederiveWarning[]>([]);
  /** パーツ・メッシュなどが 1 つもない (空のプロジェクト)。ビューポートとツリーが案内を出す。 */
  empty = $state(false);
  /** refresh が成功するたびに増える (ファイルメニューの状態の取り直しなどに使う)。 */
  revision = $state(0);

  async refresh(): Promise<void> {
    try {
      const json = await ipc.getModelJson();
      // バンドルに書けない状態 (テクスチャの欠けなど) でも表示は続ける (JSON なら白いテクスチャで描く)。
      const bundle = await ipc.getModelBundle().catch((e) => {
        console.warn("ugoe: showing the model without textures:", e);
        return null;
      });
      this.model = { json, bundle };
      const parsed = JSON.parse(json) as Model;
      this.parameters = parsed.parameters;
      this.empty = isEmptyModel(parsed);
      this.error = null;
      this.revision += 1;
    } catch (e) {
      this.error = String(e);
    }
  }

  async apply(command: Command): Promise<void> {
    await this.#run(async () => {
      const report = await ipc.applyCommand(command);
      if (report.warnings.length > 0) {
        this.warnings = mergeWarnings(this.warnings, report.warnings);
      }
    });
  }

  dismissWarnings(): void {
    this.warnings = [];
  }

  async undo(): Promise<void> {
    await this.#run(ipc.undo);
  }

  async redo(): Promise<void> {
    await this.#run(ipc.redo);
  }

  async #run(action: () => Promise<unknown>): Promise<void> {
    try {
      await action();
    } catch (e) {
      this.error = String(e);
      return;
    }
    await this.refresh();
  }
}
