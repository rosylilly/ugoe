// バックエンドが持つドキュメント (モデルの正となる状態) の、フロントエンド側の写し。
// 編集は必ず IPC の Command で行い、その後 refresh() で読み直す (docs/editor.md)。
// 編集 (send・apply・applyWith・undo・redo) は 1 本の列に並べ、前の適用と読み直しが終わってから次を始める。

import { tick } from "svelte";
import { isEmptyModel } from "./emptyState";
import { errorText } from "./errors";
import {
  type ApplyReport,
  type Command,
  type Expression,
  ipc,
  type Model,
  type ModuleStatus,
  type Motion,
  type Parameter,
  type PhysicsSettings,
  type RederiveWarning,
} from "./ipc";
import { setHiddenIdPrefix } from "./labels";
import { mergeWarnings } from "./panels/inspector/inference/inference";
import type { ViewportModel } from "./viewport/controller.svelte";

/** パネルが使うパラメータの定義 (ugoe_model::Parameter)。 */
export type ParameterDef = Parameter;

export class EditorDocument {
  /** ビューポートに渡すモデル (JSON と、テクスチャ付きの .ugoe バンドル)。refresh のたびに作り直す。 */
  model = $state.raw<ViewportModel | null>(null);
  parameters = $state<ParameterDef[]>([]);
  /** モーションと表情 (保存・書き出しの順)。refresh のたびに読み直す。 */
  motions = $state.raw<Motion[]>([]);
  expressions = $state.raw<Expression[]>([]);
  /**
   * モジュールのプロジェクトなら、本体とスロットと本体の要素 (読み取り専用の下敷き)。本体のプロジェクトなら null。
   * docs/editor.md#モジュールのプロジェクト。
   */
  module = $state.raw<ModuleStatus | null>(null);
  /** 物理の設定 (なければ null)。物理のパネルとビューポートの表示が読む。 */
  physics = $state.raw<PhysicsSettings | null>(null);
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
  /**
   * 列に並べてまだ終わっていない編集の数 (`send`・`apply`・`applyWith`・`undo`・`redo`)。
   * パネルは並べ替え・削除のように番号や今の姿に頼るボタンを、これが 0 でない間は無効にする。
   */
  pending = $state(0);
  /** 始めた読み直しの数。重なった読み直しは、最後に始めたものだけが結果を置く。 */
  #generation = 0;
  /** 編集の列の末尾 (失敗しても次へ進むよう、拒否を飲み込んだもの)。 */
  #queue: Promise<void> = Promise.resolve();

  async refresh(): Promise<void> {
    const generation = ++this.#generation;
    try {
      const [json, bundle, animations, physics, module] = await Promise.all([
        ipc.getModelJson(),
        // バンドルに書けない状態 (テクスチャの欠けなど) でも表示は続ける (JSON なら白いテクスチャで描く)。
        ipc.getModelBundle().catch((e) => {
          console.warn("ugoe: showing the model without textures:", e);
          return null;
        }),
        ipc.getAnimations(),
        ipc.getPhysics(),
        ipc.moduleStatus(),
      ]);
      // 後から始めた読み直しがあれば、古い結果で上書きしない (Undo の連打・エージェントの変更の知らせなど)。
      if (generation !== this.#generation) return;
      this.physics = physics;
      this.module = module;
      // 表記から合成の接頭辞を隠す (モデルを差し替える前に。表記はモデルから作り直される)
      setHiddenIdPrefix(this.module ? `${this.module.name}/` : null);
      this.model = { json, bundle };
      this.motions = animations.motions;
      this.expressions = animations.expressions;
      const parsed = JSON.parse(json) as Model;
      this.parameters = parsed.parameters;
      this.empty = isEmptyModel(parsed);
      this.error = null;
      this.revision += 1;
    } catch (e) {
      if (generation === this.#generation) this.error = errorText(e);
    }
  }

  /**
   * 前に並べた編集の適用と読み直しが終わるのを待ってから、`build` でコマンドを組み立てて適用し、読み直す。
   * `build` は送る直前に呼ぶので、その時点の最新の写し (`physics`・`model` など) から組み立てる
   * (画面が持つ古い写しから組み立てると、前の編集を戻してしまう)。null を返したら何も送らない。
   * 適用の報告を返し、送らなかったとき・失敗したときは null (失敗の理由は `error`)。
   */
  async send(build: () => Command | null): Promise<ApplyReport | null> {
    return this.#enqueue(async () => {
      // 読み直した値を、画面の写し ($derived・each の要素) にも行き渡らせてから組み立てる。
      await tick();
      let command: Command | null;
      try {
        command = build();
      } catch (e) {
        this.error = errorText(e);
        return null;
      }
      return command ? this.#applyNow(() => ipc.applyCommand(command)) : null;
    });
  }

  /**
   * コマンドを適用して読み直す (前に並べた編集の後に)。適用の報告 (ミラーの結果など) を返し、失敗したら null
   * (`error` に理由)。今の値から組み立てるコマンドは `send` で送る。
   */
  async apply(command: Command): Promise<ApplyReport | null> {
    return this.applyWith(() => ipc.applyCommand(command));
  }

  /**
   * 適用の報告を返す IPC (AI の提案の採用など) を、前に並べた編集の後に呼んで読み直す。`apply` と同じく失敗したら null。
   * `rethrow` なら、失敗を `error` に置かずに例外のまま返す (ダイアログが自分のエラーとして出すとき)。
   */
  async applyWith(
    action: () => Promise<ApplyReport>,
    options: { rethrow?: boolean } = {},
  ): Promise<ApplyReport | null> {
    return this.#enqueue(() => this.#applyNow(action, options.rethrow));
  }

  dismissWarnings(): void {
    this.warnings = [];
  }

  async undo(): Promise<void> {
    await this.#enqueue(() => this.#run(ipc.undo));
  }

  async redo(): Promise<void> {
    await this.#enqueue(() => this.#run(ipc.redo));
  }

  /** `task` を列の末尾に並べる。前のものが失敗しても次へ進む。 */
  #enqueue<T>(task: () => Promise<T>): Promise<T> {
    this.pending += 1;
    const result = this.#queue.then(task).finally(() => {
      this.pending -= 1;
    });
    this.#queue = result.then(
      () => undefined,
      () => undefined,
    );
    return result;
  }

  /** 適用して読み直す (列の中で呼ぶ)。 */
  async #applyNow(
    action: () => Promise<ApplyReport>,
    rethrow = false,
  ): Promise<ApplyReport | null> {
    let applied: ApplyReport | null = null;
    await this.#run(async () => {
      const report = await action();
      if (report.warnings.length > 0) {
        this.warnings = mergeWarnings(this.warnings, report.warnings);
      }
      applied = report;
    }, rethrow);
    return applied;
  }

  async #run(action: () => Promise<unknown>, rethrow = false): Promise<void> {
    try {
      await action();
    } catch (e) {
      if (rethrow) throw e;
      this.error = errorText(e);
      return;
    }
    await this.refresh();
  }
}
