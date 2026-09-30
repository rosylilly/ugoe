//! 着せ替えのモジュールのプロジェクトの IPC コマンド (ADR 0024、`docs/editor.md#モジュールのプロジェクト`)。
//! 作業用のモデル・本体の読み取り専用・書き出しの中身は `ugoe_editor_core::module_project` と
//! `ugoe_editor_core::project` にあり、ここはパスと状態の受け渡しだけを行う。
//! 引数・戻り値の型を変えたら `apps/editor/src/lib/ipc.ts` も同じ PR で更新すること。

use std::path::Path;

use serde::Serialize;
use tauri::State;
use ts_rs::TS;
use ugoe_editor_core::project::{ProjectError, load_base, read_module_link};
use ugoe_editor_core::{Document, ModuleError, Project, ProjectFingerprint};
use ugoe_model::module::ModuleRequirements;

use crate::AppState;
use crate::project::{
    ExternalChange, ProjectSession, ProjectStatus, absolute_path, lock_both, status,
};

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
    /// 本体のテクスチャの名前 (作業用のモデルの `textures` の先頭から、この数だけが本体のもの)。
    pub base_textures: Vec<String>,
    /// 本体の物理のリグと衝突体の ID (読み取り専用)。
    pub base_rigs: Vec<String>,
    pub base_colliders: Vec<String>,
    /// モジュールの条件 (`Module::requires`)。変えるのは `Command::SetModuleSettings`。
    pub requires: ModuleRequirements,
    /// 本体のスロット (モジュールの設定の画面で、付けるスロットと条件のスロットを選ぶ)。
    pub base_slots: Vec<SlotSummary>,
    /// 本体のパラメータの ID (条件のパラメータを選ぶ)。
    pub base_parameters: Vec<String>,
    /// 本体が名乗るアバタープロファイルのレベル (なければ `null`)。
    pub base_avatar_level: Option<u8>,
}

/// モジュールのプロジェクトが本体の問題で開けない・読み直せないときの、問題の種類 (直し方を選ぶ)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ModuleProblemKind {
    /// 本体のファイルが見つからない (動かした・名前を変えた)。本体を選び直す。
    BaseNotFound,
    /// 本体を読めない (壊れている・モジュールのプロジェクトを指している・PNG でない画像)。本体を選び直す。
    BaseUnreadable,
    /// 本体にモジュールのスロットがない。別のスロットを選ぶか、本体にスロットを戻す。
    UnknownSlot,
    /// 本体はあるが、モジュールが合わない (条件・公開されたパラメータ・マスクなど)。本体を直すか、選び直す。
    DoesNotFit,
}

/// モジュールのプロジェクトが本体の問題で開けない・読み直せないこと (`module_open_problem`・`reload_base`)。
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct ModuleProblem {
    pub kind: ModuleProblemKind,
    /// 本体のファイルの絶対パス。
    pub base: String,
    /// モジュールのスロット。
    pub slot: String,
    /// 本体を読めたなら、そのスロットの ID (別のスロットを選ぶ)。
    pub base_slots: Vec<String>,
    /// 元のエラーの文。
    pub message: String,
}

/// 本体の読み直し (`reload_base`) の結果。読み直せなければ `problem` があり、ドキュメントはそのまま。
#[derive(Debug, Serialize, TS)]
pub struct BaseReload {
    pub status: ProjectStatus,
    pub problem: Option<ModuleProblem>,
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
    let base = &context.base().model;
    let physics = context.base().physics.as_ref();
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
        base_textures: base.textures.iter().map(|t| t.name.clone()).collect(),
        base_rigs: physics.map_or_else(Vec::new, |p| p.rigs.iter().map(|r| r.id.clone()).collect()),
        base_colliders: physics.map_or_else(Vec::new, |p| {
            p.colliders.iter().map(|c| c.id.clone()).collect()
        }),
        requires: context.requires().clone(),
        base_slots: base.slots.iter().map(slot_summary).collect(),
        base_parameters: base.parameters.iter().map(|p| p.id.to_string()).collect(),
        base_avatar_level: base.avatar.as_ref().map(|a| a.level),
    })
}

/// 本体の問題で開けない・読み直せないときのエラーを、直し方を選べる形にする。本体の問題でなければ `None`。
fn diagnose(error: &ProjectError, base: &Path, slot: &str) -> Option<ModuleProblem> {
    let kind = match error {
        ProjectError::BaseNotFound { .. } => ModuleProblemKind::BaseNotFound,
        ProjectError::Base { .. }
        | ProjectError::BaseIsModule { .. }
        | ProjectError::UnsupportedBaseTexture { .. } => ModuleProblemKind::BaseUnreadable,
        ProjectError::Module(ModuleError::UnknownSlot(_)) => ModuleProblemKind::UnknownSlot,
        ProjectError::Module(ModuleError::Compose(_) | ModuleError::Shape { .. }) => {
            ModuleProblemKind::DoesNotFit
        }
        _ => return None,
    };
    let base_slots = load_base(base)
        .map(|b| b.model.slots.iter().map(|s| s.id.to_string()).collect())
        .unwrap_or_default();
    Some(ModuleProblem {
        kind,
        base: base.display().to_string(),
        slot: slot.to_owned(),
        base_slots,
        message: error.to_string(),
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

/// モジュールのプロジェクト `path` を、本体を `base` に、スロットを `slot` に付け替えて開く (`null` なら
/// `project.json` のまま。本体を動かした・見つからない・本体からスロットがなくなったとき)。
/// 付け替えは保存するまでファイルに書かない (未保存の印が付く)。失敗したら今のプロジェクトはそのまま。
#[tauri::command]
pub fn open_module_with_base(
    state: State<'_, AppState>,
    path: String,
    base: Option<String>,
    slot: Option<String>,
) -> Result<ProjectStatus, String> {
    let dir = absolute_path(&path)?;
    let disk = ProjectFingerprint::read(&dir).map_err(|e| e.to_string())?;
    let base = base.as_deref().map(absolute_path).transpose()?;
    let slot = slot.map(|s| ugoe_model::SlotId::from(s.as_str()));
    let project = Project::load_module_with(&dir, base.as_deref(), slot.as_ref())
        .map_err(|e| e.to_string())?;
    let (mut document, mut session) = lock_both(&state)?;
    (*session, *document) = ProjectSession::from_project(project, Some((dir, disk)));
    session.mark_meta_modified();
    Ok(status(&session, &document))
}

/// `path` がモジュールのプロジェクトで、本体の問題 (見つからない・読めない・スロットがない・合わない) で
/// 開けないなら、その問題と直すための情報。開ける・モジュールのプロジェクトでない・本体と関係のない
/// エラーなら `null`。`base` があれば `project.json` の本体の代わりにそれで確かめる (選び直した本体)。
/// `open_project`・`open_module_with_base` が失敗した後にフロントエンドが呼ぶ。
#[tauri::command]
pub fn module_open_problem(
    path: String,
    base: Option<String>,
) -> Result<Option<ModuleProblem>, String> {
    let dir = absolute_path(&path)?;
    let Ok((linked, slot)) = read_module_link(&dir) else {
        return Ok(None);
    };
    let base = base.as_deref().map(absolute_path).transpose()?;
    Ok(
        match Project::load_module_with(&dir, base.as_deref(), None) {
            Ok(_) => None,
            Err(error) => diagnose(&error, base.as_deref().unwrap_or(&linked), slot.as_str()),
        },
    )
}

/// モジュールのプロジェクトの本体のファイルが、開いた (読み直した) 後で変わったか。変わっていなければ `null`
/// (`docs/editor.md#本体の変化`)。フロントエンドが外での書き換えと同じ機会に呼ぶ。
#[tauri::command]
pub fn check_base_change(state: State<'_, AppState>) -> Result<Option<ExternalChange>, String> {
    let (document, session) = lock_both(&state)?;
    session.base_change(&document)
}

/// モジュールのプロジェクトの本体をファイルから読み直す。保存していないモジュールの編集は保ち、Undo の履歴は消す。
/// 読み直せなければ (`problem`)、ドキュメントはそのまま。
#[tauri::command]
pub fn reload_base(state: State<'_, AppState>) -> Result<BaseReload, String> {
    let (mut document, mut session) = lock_both(&state)?;
    let context = document
        .module()
        .ok_or("this is not a module project")?
        .clone();
    let path = context.base().path.clone();
    let slot = context.slot().id.to_string();
    let result =
        load_base(&path).and_then(|base| document.rebase_module(base).map_err(ProjectError::from));
    let problem = match result {
        Ok(()) => {
            session.remember_base(&document);
            None
        }
        Err(error) => Some(
            diagnose(&error, &path, &slot).unwrap_or_else(|| ModuleProblem {
                kind: ModuleProblemKind::DoesNotFit,
                base: path.display().to_string(),
                slot: slot.clone(),
                base_slots: Vec::new(),
                message: error.to_string(),
            }),
        ),
    };
    Ok(BaseReload {
        status: status(&session, &document),
        problem,
    })
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

    fn save_base(dir: &Path) -> std::path::PathBuf {
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
        base_dir
    }

    /// 本体が変わったことに気づき、読み直せる。スロットがなくなれば、直し方を選べる問題になる。
    #[test]
    fn base_changes_are_detected_and_diagnosed() {
        let dir = temp("base-change");
        let base_dir = save_base(&dir);
        let module_dir = dir.join("hat.ugoeproj");
        Project::new_module("Hat", &base_dir, &"Hair".into())
            .unwrap()
            .save(&module_dir)
            .unwrap();
        let (project, disk) = Project::load_tracked(&module_dir).unwrap();
        let (mut session, mut document) =
            ProjectSession::from_project(project, Some((module_dir.clone(), disk)));
        assert_eq!(session.base_change(&document).unwrap(), None);
        assert!(
            module_open_problem(module_dir.display().to_string(), None)
                .unwrap()
                .is_none()
        );

        let mut base = Project::load(&base_dir).unwrap();
        base.model.slots[0].draw_order.max = 600.0;
        base.save(&base_dir).unwrap();
        let change = session.base_change(&document).unwrap().unwrap();
        assert_eq!(change.files, vec!["model.json".to_owned()]);
        assert!(!change.missing);
        // 読み直すと本体の新しいスロットの範囲になり、変化は消える。
        document
            .rebase_module(load_base(&base_dir).unwrap())
            .unwrap();
        session.remember_base(&document);
        assert_eq!(session.base_change(&document).unwrap(), None);
        assert_eq!(
            module_status_of(&document).unwrap().slot.draw_order_max,
            600.0
        );

        // スロット `Hair` を消すと開けない。本体のスロットの一覧が付く。
        base.model.slots.remove(0);
        base.model.avatar = None;
        base.save(&base_dir).unwrap();
        let problem = module_open_problem(module_dir.display().to_string(), None)
            .unwrap()
            .unwrap();
        assert_eq!(problem.kind, ModuleProblemKind::UnknownSlot);
        assert_eq!(problem.slot, "Hair");
        assert_eq!(problem.base_slots, vec!["Outfit".to_owned()]);
        // 本体を動かすと、見つからない問題になる。
        std::fs::rename(&base_dir, dir.join("moved.ugoeproj")).unwrap();
        assert!(session.base_change(&document).unwrap().unwrap().missing);
        let problem = module_open_problem(module_dir.display().to_string(), None)
            .unwrap()
            .unwrap();
        assert_eq!(problem.kind, ModuleProblemKind::BaseNotFound);
        assert!(problem.base_slots.is_empty());
        // 選び直した本体で確かめると、スロットの問題になる。
        let moved = dir.join("moved.ugoeproj").display().to_string();
        let problem = module_open_problem(module_dir.display().to_string(), Some(moved.clone()))
            .unwrap()
            .unwrap();
        assert_eq!(problem.kind, ModuleProblemKind::UnknownSlot);
        assert_eq!(problem.base, moved);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
