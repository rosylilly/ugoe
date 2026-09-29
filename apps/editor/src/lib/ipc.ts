// Tauri バックエンド (apps/editor/src-tauri/src/commands.rs など) への型付きラッパー。
// バックエンドのコマンドを変えたら、ここも同じ PR で更新すること。
// 引数・戻り値の型は Rust から生成した ./generated/types.ts を使う (`just gen`、docs/editor.md#型の生成)。
import { invoke } from "@tauri-apps/api/core";
import type {
  AiReply,
  AiSettings,
  AnimationData,
  AppInfo,
  ApplyReport,
  AutoMeshOptions,
  Command,
  ConsentAnswer,
  ElementOverflow,
  KeyformTarget,
  OverflowReport,
  PhysicsSettings,
  ProjectStatus,
  PsdImportWarning,
  RestPoseCollision,
} from "./generated/types";
import type { Point } from "./panels/inspector/keyforms/keyforms";

export type * from "./generated/types";

/** ugoe_editor_core::Command の JSON 表現。`Command` の別名 (既存のコードとの互換のため)。 */
export type EditCommand = Command;

export const ipc = {
  appInfo: () => invoke<AppInfo>("app_info"),
  /** 適用はしたが知らせたいこと (自動のミラーのやり直しの失敗) を返す。 */
  applyCommand: (command: Command) => invoke<ApplyReport>("apply_command", { command }),
  /** ugoe_model::Model の JSON (JSON.parse すると `Model`)。 */
  getModelJson: () => invoke<string>("get_model_json"),
  /** ドキュメントのモーションと表情 (タイムラインの表示用。編集は applyCommand)。 */
  getAnimations: () => invoke<AnimationData>("get_animations"),
  /** 物理の設定 (ugoe_model::physics::PhysicsSettings)。なければ null。 */
  getPhysics: () => invoke<PhysicsSettings | null>("get_physics"),
  undo: () => invoke<boolean>("undo"),
  redo: () => invoke<boolean>("redo"),
  projectStatus: () => invoke<ProjectStatus>("project_status"),
  newProject: () => invoke<ProjectStatus>("new_project"),
  /** apps/editor/src-tauri/src/project.rs。PSD から新しいプロジェクトを作り、取り込みの警告を返す。 */
  newProjectFromPsd: (path: string, meshOptions?: Partial<AutoMeshOptions>) =>
    invoke<PsdImportWarning[]>("new_project_from_psd", { path, meshOptions: meshOptions ?? null }),
  openProject: (path: string) => invoke<ProjectStatus>("open_project", { path }),
  saveProject: () => invoke<ProjectStatus>("save_project"),
  /** 拡張子 .ugoeproj がなければバックエンドが付ける。 */
  saveProjectAs: (path: string) => invoke<ProjectStatus>("save_project_as", { path }),
  exportBundle: (path: string) => invoke<void>("export_bundle", { path }),
  /** 今のドキュメントの .ugoe バンドル (テクスチャ付き)。生のバイト列で届く。 */
  getModelBundle: async () => new Uint8Array(await invoke<ArrayBuffer>("get_model_bundle")),
  /** apps/editor/src-tauri/src/psd.rs。PSD を今のドキュメントに取り込み、警告を返す。 */
  /** `meshOptions` の省略したフィールドは既定値。 */
  importPsd: (path: string, meshOptions?: Partial<AutoMeshOptions>) =>
    invoke<PsdImportWarning[]>("import_psd", { path, meshOptions: meshOptions ?? null }),
  /** apps/editor/src-tauri/src/space.rs。`target` の親の座標系の点をキャンバス座標に写す。 */
  localToCanvas: (target: KeyformTarget, parameters: Record<string, number>, points: Point[]) =>
    invoke<Point[]>("local_to_canvas", { target, parameters, points }),
  /** キャンバス座標の点を `target` の親の座標系に写す。`guesses` は反復の初期値 (親の座標系)。 */
  canvasToLocal: (
    target: KeyformTarget,
    parameters: Record<string, number>,
    points: Point[],
    guesses: Point[],
  ) => invoke<Point[]>("canvas_to_local", { target, parameters, points, guesses }),
  /**
   * apps/editor/src-tauri/src/overflow.rs。パラメータの値 (含まないものは既定値) で、親のワープの格子の
   * 外にある点を要素ごとに返す。`tolerance` は UV の許容距離 (省略すると既定値)。
   */
  findOverflows: (parameters: Record<string, number>, tolerance?: number) =>
    invoke<ElementOverflow[]>("find_overflows", { parameters, tolerance: tolerance ?? null }),
  /** 親がワープのすべての要素について、キーフォームのすべてのセルではみ出しを調べる。 */
  verifyOverflows: (tolerance?: number) =>
    invoke<OverflowReport>("verify_overflows", { tolerance: tolerance ?? null }),
  /**
   * apps/editor/src-tauri/src/physics_check.rs。パラメータの既定値での静止姿勢で衝突体に当たっている
   * リグの鎖 (リグと衝突体の組ごと)。物理の設定がなければ空。
   */
  checkPhysics: () => invoke<RestPoseCollision[]>("check_physics"),
  /**
   * apps/editor/src-tauri/src/ai.rs。AI の設定 (API キーの状態・開いているプロジェクトの「外部に送らない」)。
   * キーの中身は返さない (docs/editor.md#ai-の設定と送信の同意)。
   */
  aiSettings: () => invoke<AiSettings>("ai_settings"),
  /** API キーを OS の資格情報の保存先に保存する。 */
  storeApiKey: (key: string) => invoke<AiSettings>("store_api_key", { key }),
  deleteApiKey: () => invoke<AiSettings>("delete_api_key"),
  /** 開いているプロジェクトのデータを外部の AI に送ってよいか (プロジェクトと一緒に保存する)。 */
  setAiSendExternally: (allowed: boolean) =>
    invoke<AiSettings>("set_ai_send_externally", { allowed }),
  /** 「今後は確かめない」を取り消す。 */
  forgetAiConsent: () => invoke<AiSettings>("forget_ai_consent"),
  /**
   * API キーと接続を確かめる (画像なしの小さなリクエスト)。外部に送る他のコマンドと同じく、
   * 同意がなければ送らずに `needs_consent` を返す。lib/ai/consent.ts の `withConsent` で呼ぶ。
   */
  testAiConnection: (consent: ConsentAnswer | null) =>
    invoke<AiReply<null>>("test_ai_connection", { consent }),
};
