// Tauri バックエンド (apps/editor/src-tauri/src/commands.rs) への型付きラッパー。
// バックエンドのコマンドを変えたら、ここも同じ PR で更新すること。
// TODO(T0603): ts-rs で Rust 型から自動生成した型に置き換える。
import { invoke } from "@tauri-apps/api/core";

export interface AppInfo {
  version: string;
}

/** ugoe_editor_core::Command の JSON 表現 (一部)。 */
export type EditCommand =
  | { op: "set_parameter_range"; id: string; min: number; default: number; max: number }
  | { op: "remove_parameter"; id: string }
  | { op: "batch"; label: string; commands: EditCommand[] };

/** apps/editor/src-tauri/src/project.rs の ProjectStatus。 */
export interface ProjectStatus {
  name: string;
  /** 保存先の .ugoeproj ディレクトリ。未保存なら null。 */
  path: string | null;
  /** 最後に保存 (または開いた) ときから変更があるか。 */
  modified: boolean;
}

export const ipc = {
  appInfo: () => invoke<AppInfo>("app_info"),
  applyCommand: (command: EditCommand) => invoke<void>("apply_command", { command }),
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
};
