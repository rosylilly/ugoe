//! 着せ替えのモジュールのプロジェクトの IPC コマンド (ADR 0024、`docs/editor.md#モジュールのプロジェクト`)。
//! 作業用のモデル・本体の読み取り専用・書き出しの中身は `ugoe_editor_core::module_project` と
//! `ugoe_editor_core::project` にあり、ここはパスと状態の受け渡しだけを行う。
//! 引数・戻り値の型を変えたら `apps/editor/src/lib/ipc.ts` も同じ PR で更新すること。

use serde::Serialize;
use tauri::State;
use ts_rs::TS;
use ugoe_editor_core::project::load_base;
use ugoe_editor_core::{Document, Project, ProjectFingerprint};

use crate::AppState;
use crate::project::{ProjectSession, ProjectStatus, absolute_path, lock_both, status};

/// 本体のスロット 1 つ (新規作成の画面で選ぶ)。
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct SlotSummary {
    pub id: String,
    /// 取り付け先のデフォーマ。`null` ならキャンバス。
    pub anchor: Option<String>,
    /// モジュールの最上位のパーツの親になる本体のパーツ。
    pub part: Option<String>,
    /// 描画順の範囲。
    pub draw_order_min: f32,
    pub draw_order_max: f32,
    /// モジュールを付けると隠れる本体のパーツ。
    pub replaces: Vec<String>,
    /// モジュールが使える本体のパラメータ (標準パラメータのほかに)。
    pub parameters: Vec<String>,
}

/// 本体として選んだファイルの要約 (`read_module_base` の戻り値)。
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct BaseSummary {
    /// 本体のモデルの名前。
    pub name: String,
    pub slots: Vec<SlotSummary>,
}

/// 開いているモジュールのプロジェクトの情報 (`module_status` の戻り値)。
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct ModuleStatus {
    /// モジュールの名前 (合成で ID の接頭辞になる)。
    pub name: String,
    /// 本体のファイルの絶対パス。
    pub base: String,
    pub slot: SlotSummary,
    /// 作業用のモデルのうち本体の要素 (読み取り専用) の ID。
    pub base_parts: Vec<String>,
    pub base_deformers: Vec<String>,
    pub base_meshes: Vec<String>,
}

/// `.ugoemod` の書き出しの結果。
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct ModuleExportReport {
    /// 書き出したファイルの大きさ (バイト)。
    #[ts(type = "number")]
    pub bytes: usize,
    /// 書き出しはしたが知らせたいこと。
    pub warnings: Vec<String>,
}

fn slot_summary(slot: &ugoe_model::Slot) -> SlotSummary {
    let ids = |ids: &[ugoe_model::PartId]| ids.iter().map(ToString::to_string).collect();
    SlotSummary {
        id: slot.id.to_string(),
        anchor: slot.anchor.as_ref().map(ToString::to_string),
        part: slot.part.as_ref().map(ToString::to_string),
        draw_order_min: slot.draw_order.min,
        draw_order_max: slot.draw_order.max,
        replaces: ids(&slot.replaces),
        parameters: slot.parameters.iter().map(ToString::to_string).collect(),
    }
}

pub(crate) fn module_status_of(document: &Document) -> Option<ModuleStatus> {
    let context = document.module()?;
    let model = document.model();
    Some(ModuleStatus {
        name: context.name().to_owned(),
        base: context.base().path.display().to_string(),
        slot: slot_summary(context.slot()),
        base_parts: model
            .parts
            .iter()
            .filter(|p| context.is_base_part(&p.id))
            .map(|p| p.id.to_string())
            .collect(),
        base_deformers: model
            .deformers
            .iter()
            .filter(|d| context.is_base_deformer(&d.id))
            .map(|d| d.id.to_string())
            .collect(),
        base_meshes: model
            .meshes
            .iter()
            .filter(|m| context.is_base_mesh(&m.id))
            .map(|m| m.id.to_string())
            .collect(),
    })
}

/// 開いているのがモジュールのプロジェクトなら、その本体とスロット。本体のモデルのプロジェクトなら `null`。
#[tauri::command]
pub fn module_status(state: State<'_, AppState>) -> Result<Option<ModuleStatus>, String> {
    Ok(module_status_of(&*crate::commands::lock(&state)?))
}

/// 本体の候補 (`.ugoeproj` のディレクトリか `.ugoe`) を読み、名前とスロットを返す (新規作成の画面)。
#[tauri::command]
pub fn read_module_base(path: String) -> Result<BaseSummary, String> {
    let base = load_base(&absolute_path(&path)?).map_err(|e| e.to_string())?;
    Ok(BaseSummary {
        name: base.model.meta.name.clone(),
        slots: base.model.slots.iter().map(slot_summary).collect(),
    })
}

/// 本体 `base` のスロット `slot` に付ける、名前 `name` の空のモジュールのプロジェクトにする
/// (未保存。未保存の変更は破棄する。確認はフロントエンドで行う)。失敗したら今のプロジェクトはそのまま。
#[tauri::command]
pub fn new_module_project(
    state: State<'_, AppState>,
    base: String,
    slot: String,
    name: String,
) -> Result<ProjectStatus, String> {
    let project = Project::new_module(name.trim(), &absolute_path(&base)?, &slot.as_str().into())
        .map_err(|e| e.to_string())?;
    let (mut document, mut session) = lock_both(&state)?;
    (*session, *document) = ProjectSession::from_project(project, None);
    Ok(status(&session, &document))
}

/// モジュールのプロジェクト `path` を、本体を `base` に付け替えて開く (本体を動かした・見つからないとき)。
/// 付け替えは保存するまでファイルに書かない (未保存の印が付く)。失敗したら今のプロジェクトはそのまま。
#[tauri::command]
pub fn open_module_with_base(
    state: State<'_, AppState>,
    path: String,
    base: String,
) -> Result<ProjectStatus, String> {
    let dir = absolute_path(&path)?;
    let disk = ProjectFingerprint::read(&dir).map_err(|e| e.to_string())?;
    let project =
        Project::load_with_base(&dir, &absolute_path(&base)?).map_err(|e| e.to_string())?;
    let (mut document, mut session) = lock_both(&state)?;
    (*session, *document) = ProjectSession::from_project(project, Some((dir, disk)));
    session.mark_meta_modified();
    Ok(status(&session, &document))
}

/// 開いているモジュールのプロジェクトを `.ugoemod` として書き出す。書き出す前に、モジュールの検証と
/// 本体との合成を確かめる。保存状態は変えない。
#[tauri::command]
pub fn export_module(
    state: State<'_, AppState>,
    path: String,
) -> Result<ModuleExportReport, String> {
    let path = absolute_path(&path)?;
    let export = {
        let (document, session) = lock_both(&state)?;
        ugoe_editor_core::export_module(&session.to_project(&document))
            .map_err(|e| e.to_string())?
    };
    std::fs::write(&path, &export.bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(ModuleExportReport {
        bytes: export.bytes.len(),
        warnings: export.warnings,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ugoe_editor_core::{Command, ProjectMeta};

    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("t0648-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    /// 本体を保存し、そのスロットに付けるモジュールのプロジェクトを作って書き出す。
    #[test]
    fn new_module_project_can_be_exported() {
        let dir = temp("export");
        let base_dir = dir.join("avatar.ugoeproj");
        let model = ugoe_model::fixtures::dress_up_base();
        let textures = model
            .textures
            .iter()
            .map(|t| {
                let png = ugoe_editor_core::texture::solid_png(2, 2, [255; 4]);
                (t.name.clone(), Arc::from(png))
            })
            .collect();
        Project::new(ProjectMeta::new("avatar"), model, textures)
            .save(&base_dir)
            .unwrap();
        let summary = read_module_base(base_dir.display().to_string()).unwrap();
        assert_eq!(summary.slots.len(), 2);

        let project = Project::new_module("Hat", &base_dir, &"Hair".into()).unwrap();
        let (session, mut document) = ProjectSession::from_project(project, None);
        let module = module_status_of(&document).unwrap();
        assert_eq!(module.slot.id, "Hair");
        assert!(module.base_meshes.contains(&"Face".to_owned()));
        // 本体は読み取り専用
        let error = document
            .apply(Command::RemoveMesh { id: "Face".into() })
            .unwrap_err();
        assert!(error.to_string().contains("base model"), "{error}");
        let export = ugoe_editor_core::export_module(&session.to_project(&document)).unwrap();
        let file = ugoe_format::read_module(&export.bytes).unwrap();
        assert_eq!(file.module.slot.as_str(), "Hair");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
