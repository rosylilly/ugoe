// バックエンドが持つドキュメント (モデルの正となる状態) の、フロントエンド側の写し。
// 編集は必ず IPC の Command で行い、その後 refresh() で読み直す (docs/editor.md)。
import { type EditCommand, ipc } from "./ipc";
import type { ViewportModel } from "./viewport/controller.svelte";

/** パネルが使うパラメータの定義 (ugoe_model::Parameter の一部)。 */
export interface ParameterDef {
  id: string;
  name: string;
  min: number;
  max: number;
  default: number;
}

export class EditorDocument {
  /** ビューポートに渡すモデル (JSON と、テクスチャ付きの .ugoe バンドル)。refresh のたびに作り直す。 */
  model = $state.raw<ViewportModel | null>(null);
  parameters = $state<ParameterDef[]>([]);
  error = $state<string | null>(null);
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
      this.parameters = JSON.parse(json).parameters;
      this.error = null;
      this.revision += 1;
    } catch (e) {
      this.error = String(e);
    }
  }

  async apply(command: EditCommand): Promise<void> {
    await this.#run(() => ipc.applyCommand(command));
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
