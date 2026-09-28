// Tauri バックエンド (apps/editor/src-tauri/src/commands.rs など) への型付きラッパー。
// バックエンドのコマンドを変えたら、ここも同じ PR で更新すること。
// 引数・戻り値の型は Rust から生成した ./generated/types.ts を使う (`just gen`、docs/editor.md#型の生成)。
import { invoke } from "@tauri-apps/api/core";
import type {
  AppInfo,
  AutoMeshOptions,
  Command,
  ProjectStatus,
  PsdImportWarning,
} from "./generated/types";

export type * from "./generated/types";

/** ugoe_editor_core::Command の JSON 表現。`Command` の別名 (既存のコードとの互換のため)。 */
export type EditCommand = Command;

export const ipc = {
  appInfo: () => invoke<AppInfo>("app_info"),
  applyCommand: (command: Command) => invoke<void>("apply_command", { command }),
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
};
