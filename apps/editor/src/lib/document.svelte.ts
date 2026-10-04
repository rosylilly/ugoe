// バックエンドが持つドキュメント (モデルの正となる状態) の、フロントエンド側の写し。
// 編集は必ず IPC の Command で行い、その後 refresh() で読み直す (docs/editor.md)。
// 編集 (send・apply・applyWith・undo・redo) は 1 本の列に並べ、前の適用と読み直しが終わってから次を始める。
// 数値の欄の続けての確定は、列の中で 1 つに畳み、バックエンドで 1 つの Undo にまとめる (docs/editor.md#undo)。

import { numberFieldCommit } from "@ugoe/ui";
import { tick } from "svelte";
import { isEmptyModel } from "./emptyState";
import { errorText } from "./errors";
import {
  type ApplyReport,
  type Command,
  type Expression,
  type HistoryStatus,
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

/** モデルの JSON を解析する。無いか読めなければ null。 */
export function parseModelJson(json: string | null | undefined): Model | null {
  if (!json) return null;
  try {
    const value: unknown = JSON.parse(json);
    return value !== null && typeof value === "object" ? (value as Model) : null;
  } catch {
    return null;
  }
}

/** パネルが使うパラメータの定義 (ugoe_model::Parameter)。 */
export type ParameterDef = Parameter;

export class EditorDocument {
  /** ビューポートに渡すモデル (JSON と、テクスチャ付きの .ugoe バンドル)。refresh のたびに作り直す。 */
  model = $state.raw<ViewportModel | null>(null);
  /**
   * `model.json` を解析したモデル。部品はそれぞれ `JSON.parse` せずにこれを読む。読めない JSON なら null。
   * 部品の間で共有するので書き換えない。読み直しで JSON が同じ文字列なら解析し直さず同じオブジェクトを返すので、
   * これに依存する `$derived` / `$effect` は関係のない読み直しでは走らない。
   */
  readonly parsedModel: Model | null = $derived.by(() => this.#parse(this.model?.json ?? null));
  /** 最後に解析した JSON とその結果。 */
  #parsed: { json: string | null; model: Model | null } = { json: null, model: null };
  parameters = $state<ParameterDef[]>([]);
  /** モーションと表情 (保存・書き出しの順)。refresh のたびに読み直す。 */
  motions = $state.raw<Motion[]>([]);
  expressions = $state.raw<Expression[]>([]);
  /**
   * モジュールのプロジェクトなら、本体とスロットと本体の要素 (読み取り専用の下敷き)。本体のプロジェクトなら null。
   * docs/editor.md#モジュールのプロジェクト。
   */
  module = $state.raw<ModuleStatus | null>(null);
  /**
   * Undo / Redo できるかと履歴の上限 (docs/editor.md#undo)。refresh (編集・Undo・Redo の後) と上限の変更で読み直す。
   * 読む前は null (どちらも押せない)。
   */
  history = $state.raw<HistoryStatus | null>(null);
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
  /**
   * 列の末尾に並べてまだ始まっていない、数値の欄の確定 (`numberFieldCommit` の印 `field`)。同じ欄の次の確定は、
   * 新しく並べずにこれの組み立てを置き換える (矢印キーを押し続けても、適用と読み直しが溜まらない)。
   */
  #fieldCommit: {
    field: string;
    build: () => Command | null;
    result: Promise<ApplyReport | null>;
  } | null = null;

  #parse(json: string | null): Model | null {
    if (json !== this.#parsed.json) this.#parsed = { json, model: parseModelJson(json) };
    return this.#parsed.model;
  }

  async refresh(): Promise<void> {
    const generation = ++this.#generation;
    try {
      const [json, bundle, animations, physics, module, history] = await Promise.all([
        ipc.getModelJson(),
        // バンドルに書けない状態 (テクスチャの欠けなど) でも表示は続ける (JSON なら白いテクスチャで描く)。
        ipc.getModelBundle().catch((e) => {
          console.warn("ugoe: showing the model without textures:", e);
          return null;
        }),
        ipc.getAnimations(),
        ipc.getPhysics(),
        ipc.moduleStatus(),
        ipc.historyStatus(),
      ]);
      // 後から始めた読み直しがあれば、古い結果で上書きしない (Undo の連打・エージェントの変更の知らせなど)。
      if (generation !== this.#generation) return;
      this.physics = physics;
      this.module = module;
      this.history = history;
      // 表記から合成の接頭辞を隠す (モデルを差し替える前に。表記はモデルから作り直される)
      setHiddenIdPrefix(this.module ? `${this.module.name}/` : null);
      this.model = { json, bundle };
      this.motions = animations.motions;
      this.expressions = animations.expressions;
      const parsed = this.parsedModel;
      if (!parsed) throw new Error("the model JSON could not be read");
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
   *
   * 数値の欄 (`NumberField`) の `onchange` の中で呼ぶと、同じ欄の続けての確定として送る (docs/editor.md#undo):
   * まだ始まっていない同じ欄の確定があればその組み立てを置き換え (戻り値はそれと同じ)、バックエンドは同じ欄・同じ種類の
   * コマンドの続けての適用を 1 つの Undo にまとめる。
   */
  async send(build: () => Command | null): Promise<ApplyReport | null> {
    const field = numberFieldCommit();
    if (field !== null) return this.#sendFieldCommit(field, build);
    return this.#enqueue(() => this.#buildAndApply(build, null));
  }

  /**
   * コマンドを適用して読み直す (前に並べた編集の後に)。適用の報告 (ミラーの結果など) を返し、失敗したら null
   * (`error` に理由)。今の値から組み立てるコマンドは `send` で送る。数値の欄の確定は `send` と同じくまとめる。
   */
  async apply(command: Command): Promise<ApplyReport | null> {
    const field = numberFieldCommit();
    if (field !== null) return this.#sendFieldCommit(field, () => command);
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

  /** Undo で戻れるか (画面のボタンを押せるか)。 */
  get canUndo(): boolean {
    return this.history?.can_undo ?? false;
  }

  /** Redo で進めるか。 */
  get canRedo(): boolean {
    return this.history?.can_redo ?? false;
  }

  /**
   * Undo の上限を変える (エディタ全体の設定。次の起動でも使う)。前に並べた編集の後に変え、超えた古い履歴は捨てる。
   * 失敗したら例外のまま返す (設定の画面が自分のエラーとして出す)。
   */
  async setUndoLimit(limit: number): Promise<void> {
    await this.#enqueue(async () => {
      this.history = await ipc.setUndoLimit(limit);
    });
  }

  async undo(): Promise<void> {
    await this.#enqueue(() => this.#run(ipc.undo));
  }

  async redo(): Promise<void> {
    await this.#enqueue(() => this.#run(ipc.redo));
  }

  /** 数値の欄 `field` の確定を並べる。列の末尾の、まだ始まっていない同じ欄の確定があれば、その組み立てを置き換える。 */
  #sendFieldCommit(field: string, build: () => Command | null): Promise<ApplyReport | null> {
    const waiting = this.#fieldCommit;
    if (waiting && waiting.field === field) {
      waiting.build = build;
      return waiting.result;
    }
    const entry = { field, build, result: Promise.resolve<ApplyReport | null>(null) };
    entry.result = this.#enqueue(() => {
      // 始まったら置き換えない (次の確定は新しく並べる)。
      if (this.#fieldCommit === entry) this.#fieldCommit = null;
      return this.#buildAndApply(entry.build, field);
    });
    this.#fieldCommit = entry;
    return entry.result;
  }

  /** `build` でコマンドを組み立てて適用する (列の中で呼ぶ)。`field` は数値の欄の確定の印。 */
  async #buildAndApply(
    build: () => Command | null,
    field: string | null,
  ): Promise<ApplyReport | null> {
    // 読み直した値を、画面の写し ($derived・each の要素) にも行き渡らせてから組み立てる。
    await tick();
    let command: Command | null;
    try {
      command = build();
    } catch (e) {
      this.error = errorText(e);
      return null;
    }
    if (!command) return null;
    // 同じ欄でも、種類の違うコマンドはまとめない。
    const coalesce = field === null ? undefined : `${field}:${command.op}`;
    return this.#applyNow(() => ipc.applyCommand(command, coalesce));
  }

  /** `task` を列の末尾に並べる。前のものが失敗しても次へ進む。 */
  #enqueue<T>(task: () => Promise<T>): Promise<T> {
    // 後ろに別の編集が並ぶので、前の数値の欄の確定はもう置き換えない (順番を入れ替えない)。
    this.#fieldCommit = null;
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
