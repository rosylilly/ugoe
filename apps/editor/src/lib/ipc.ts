// Tauri バックエンド (apps/editor/src-tauri/src/commands.rs など) への型付きラッパー。
// バックエンドのコマンドを変えたら、ここも同じ PR で更新すること。
// 引数・戻り値の型は Rust から生成した ./generated/types.ts を使う (`just gen`、docs/editor.md#型の生成)。
import { invoke } from "@tauri-apps/api/core";
import type {
  AppInfo,
  ApplyReport,
  AutoMeshOptions,
  Command,
  KeyformTarget,
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
  undo: () => invoke<boolean>("undo"),
  redo: () => invoke<boolean>("redo"),
  projectStatus: () => invoke<ProjectStatus>("project_status"),
  newProject: () => invoke<ProjectStatus>("new_project"),
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
};
