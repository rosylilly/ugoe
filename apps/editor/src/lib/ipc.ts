// Tauri バックエンド (apps/editor/src-tauri/src/commands.rs など) への型付きラッパー。
// バックエンドのコマンドを変えたら、ここも同じ PR で更新すること。
// 引数・戻り値の型は Rust から生成した ./generated/types.ts を使う (`just gen`、docs/editor.md#型の生成)。
import { invoke } from "@tauri-apps/api/core";
import type {
  AnimationData,
  AppInfo,
  ApplyReport,
  AutoMeshOptions,
  Command,
  ElementOverflow,
  KeyformTarget,
  OverflowReport,
  PhysicsSettings,
  ProjectStatus,
  PsdImportWarning,
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
};
