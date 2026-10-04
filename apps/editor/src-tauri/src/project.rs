//! プロジェクトの新規作成・開く・保存・書き出しの IPC コマンド。
//! 読み書きの中身は `ugoe_editor_core::project` にあり、ここはパスと状態の受け渡しだけを行う。
//! 引数・戻り値の型を変えたら `apps/editor/src/lib/ipc.ts` も同じ PR で更新すること。

use std::path::{Path, PathBuf};
use std::sync::MutexGuard;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;
use tauri::{AppHandle, State};
use ts_rs::TS;
use ugoe_editor_core::lighting::PreviewLighting;
use ugoe_editor_core::project::PROJECT_EXTENSION;
use ugoe_editor_core::project::ProjectError;
use ugoe_editor_core::{
    AutoMeshOptions, Document, ExportReport, ExportSettings, Project, ProjectFingerprint,
    ProjectMeta, PsdImportOptions, PsdImportWarning, export_bundle_with_settings, import,
};

use crate::AppState;
use crate::ai::ConsentMemory;
use crate::commands::blocking;
use crate::error::{EditorError, IpcResult};
use crate::proposals::{ProposalSession, RIGGED_MESHES_KEY};

/// 開いているプロジェクトのうち、`Document` の外にあるもの (保存先・メタ情報)。
/// モデル・物理・モーション・表情は `Document` が持つ (編集コマンドと Undo の対象)。
#[derive(Debug)]
pub struct ProjectSession {
    /// セッションの番号 (開く・作るたびに変わる。開き直しても変わる)。時間のかかる処理の間にプロジェクトが
    /// 替わったことに気づくために比べる (`crate::proposals`)。
    id: u64,
    /// 保存先の `.ugoeproj` ディレクトリ。一度も保存していなければ `None`。
    path: Option<PathBuf>,
    /// `path` のファイルの、開いた (または最後に保存した) ときの指紋。外での書き換えに気づくために比べる
    /// (`docs/editor.md#外での書き換え`)。`path` がなければ `None`。
    disk: Option<ProjectFingerprint>,
    /// モジュールのプロジェクトなら、本体のファイルの、開いた (または読み直した) ときの指紋。本体の変化に
    /// 気づくために比べる (`docs/editor.md#本体の変化`)。取れなければ `None`。
    base_disk: Option<ProjectFingerprint>,
    meta: ProjectMeta,
    /// `meta` を最後に保存 (または開いた) ときから変えたか (`Document` の変更とは別に数える)。
    meta_modified: bool,
    /// AI への送信の「今後は確かめない」。プロジェクトを替えると消え、ファイルには残さない。
    ai_consent: ConsentMemory,
    /// AI の提案と、提案の元にする PSD。プロジェクトを替えると消え、ファイルには残さない (本体の変形を写した衣装の
    /// メッシュの形の指紋だけは `editor.rigged_meshes` に残す。`crate::proposals`)。
    pub(crate) proposals: ProposalSession,
}

/// 次のセッションの番号 ([`ProjectSession::id`])。
static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

impl ProjectSession {
    /// `disk` は `path` のファイルの指紋 (`path` と一緒に渡す)。
    pub(crate) fn from_project(
        project: Project,
        path: Option<(PathBuf, ProjectFingerprint)>,
    ) -> (Self, Document) {
        let document = project.document();
        let (path, disk) = path.unzip();
        let mut proposals = ProposalSession::default();
        proposals.restore_rigged(project.meta.editor.get(RIGGED_MESHES_KEY));
        let session = Self {
            id: NEXT_SESSION.fetch_add(1, Ordering::Relaxed),
            path,
            disk,
            base_disk: base_fingerprint(&document),
            meta: project.meta,
            meta_modified: false,
            ai_consent: ConsentMemory::default(),
            proposals,
        };
        (session, document)
    }

    /// セッションの番号 (開く・作るたびに変わる)。
    pub(crate) fn id(&self) -> u64 {
        self.id
    }

    /// `Document` の外の設定 (本体の参照の付け替えなど) を変えたことにする (未保存の印を付ける)。
    pub(crate) fn mark_meta_modified(&mut self) {
        self.meta_modified = true;
    }

    /// このプロジェクトのデータを外部の AI サービスに送ってよいか (`ProjectMeta::ai_send_externally`)。
    pub(crate) fn ai_send_externally(&self) -> bool {
        self.meta.ai_send_externally()
    }

    /// 送ってよいかを変える。送らないにすると「今後は確かめない」も忘れる。
    pub(crate) fn set_ai_send_externally(&mut self, allowed: bool) {
        if self.meta.ai_send_externally() == allowed {
            return;
        }
        self.meta.set_ai_send_externally(allowed);
        self.meta_modified = true;
        if !allowed {
            self.ai_consent.forget();
        }
    }

    pub(crate) fn ai_consent(&self) -> &ConsentMemory {
        &self.ai_consent
    }

    pub(crate) fn ai_consent_mut(&mut self) -> &mut ConsentMemory {
        &mut self.ai_consent
    }

    /// 保存先の `.ugoeproj` ディレクトリ (一度も保存していなければ `None`)。
    pub(crate) fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// `project.json` の `editor` の項目 `key` (エディタだけが使う設定。`crate::dress_up` など)。
    pub(crate) fn editor_value(&self, key: &str) -> Option<&serde_json::Value> {
        self.meta.editor.get(key)
    }

    /// `editor` の項目 `key` を `value` にする (`None` なら消す)。変われば未保存の印を付ける。
    pub(crate) fn set_editor_value(&mut self, key: &str, value: Option<serde_json::Value>) {
        if self.meta.editor.get(key) == value.as_ref() {
            return;
        }
        match value {
            Some(value) => self.meta.editor.insert(key.to_owned(), value),
            None => self.meta.editor.remove(key),
        };
        self.meta_modified = true;
    }

    /// エディタにつないだエージェント (`crate::mcp`) に貸す、メタ情報と提案の置き場。
    pub(crate) fn lend(&mut self) -> (&ProjectMeta, &mut ugoe_editor_core::Proposals) {
        (&self.meta, self.proposals.proposals_mut())
    }

    /// プロジェクトの書き出しの設定 (`export_bundle` と同じもの)。
    pub(crate) fn export_settings(&self) -> ExportSettings {
        self.meta.export_settings()
    }

    pub(crate) fn to_project(&self, document: &Document) -> Project {
        Project::from_document(self.meta.clone(), document)
    }

    /// モジュールのプロジェクトの本体のファイルが、開いた (読み直した) 後で変わっていれば、その内容。
    /// `files` は本体からの相対パス (`.ugoe` ならファイル名)、`missing` は本体が見つからないこと。
    pub(crate) fn base_change(&self, document: &Document) -> IpcResult<Option<ExternalChange>> {
        let (Some(context), Some(known)) = (document.module(), &self.base_disk) else {
            return Ok(None);
        };
        let path = &context.base().path;
        if !path.exists() {
            return Ok(Some(ExternalChange {
                files: Vec::new(),
                missing: true,
                token: "base-missing".into(),
            }));
        }
        match known.reread_base(path) {
            Ok(now) if now == *known => Ok(None),
            Ok(now) => Ok(Some(ExternalChange {
                files: known.changed_files(&now),
                missing: false,
                token: format!("base-{:016x}", now.digest()),
            })),
            Err(ProjectError::NotAProject { .. }) => Ok(Some(ExternalChange {
                files: Vec::new(),
                missing: true,
                token: "base-missing".into(),
            })),
            Err(e) => Err(e.into()),
        }
    }

    /// 本体を読み直した後で、本体の指紋を取り直す。
    /// モジュールのプロジェクトの本体のファイルの、開いた (読み直した) ときの指紋の値。取れなければ `None`。
    pub(crate) fn base_digest(&self) -> Option<u64> {
        self.base_disk.as_ref().map(ProjectFingerprint::digest)
    }

    pub(crate) fn remember_base(&mut self, document: &Document) {
        self.base_disk = base_fingerprint(document);
    }

    /// 保存先のファイルが、開いた (最後に保存した) 後で外で変わっていれば、その内容。
    /// 保存先がなければ `None`。
    pub(crate) fn external_change(&self) -> IpcResult<Option<ExternalChange>> {
        let (Some(dir), Some(known)) = (&self.path, &self.disk) else {
            return Ok(None);
        };
        match known.reread(dir) {
            Ok(now) if now == *known => Ok(None),
            Ok(now) => Ok(Some(ExternalChange {
                files: known.changed_files(&now),
                missing: false,
                token: format!("{:016x}", now.digest()),
            })),
            Err(ProjectError::NotAProject { .. }) => Ok(Some(ExternalChange {
                files: Vec::new(),
                missing: true,
                token: "missing".into(),
            })),
            Err(e) => Err(e.into()),
        }
    }

    /// `dir` に保存し、保存先と指紋を覚え直す (自分の保存を外の変更と取り違えないよう、書いた直後に取る)。
    pub(crate) fn save_to(&mut self, document: &mut Document, dir: PathBuf) -> IpcResult<()> {
        self.to_project(document).save(&dir)?;
        let known = match (&self.path, &self.disk) {
            // PNG のハッシュを使い回すため、同じ場所なら前の指紋から取り直す
            (Some(path), Some(disk)) if *path == dir => disk.reread(&dir),
            _ => ProjectFingerprint::read(&dir),
        };
        // 保存はできたので、指紋が取れなくても保存したことにする (外での書き換えには気づけなくなる)
        self.disk = known
            .inspect_err(|e| eprintln!("could not fingerprint the saved project: {e}"))
            .ok();
        self.path = Some(dir);
        self.meta_modified = false;
        document.mark_saved();
        Ok(())
    }
}

/// モジュールのプロジェクトなら、本体のファイルの指紋 (取れなければ `None`)。
fn base_fingerprint(document: &Document) -> Option<ProjectFingerprint> {
    let path = &document.module()?.base().path;
    ProjectFingerprint::read_base(path)
        .inspect_err(|e| eprintln!("could not fingerprint the base model: {e}"))
        .ok()
}

/// 開いているプロジェクトのファイルが、エディタの外で変わったこと (`docs/editor.md#外での書き換え`)。
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct ExternalChange {
    /// 足された・消された・中身が変わったファイル (プロジェクトからの相対パス、`/` 区切り、名前の順)。
    pub files: Vec<String>,
    /// `project.json` がない (プロジェクトが消された・移された)。開き直せない。
    pub missing: bool,
    /// 外の状態の目印。同じ変更を二度知らせないために使う (エディタを起動している間だけ意味がある)。
    pub token: String,
}

/// 起動時の状態: 空のモデルの、保存先のないプロジェクト (`new_project` と同じ)。
/// 空のモデルに PSD を取り込むとキャンバスが PSD の大きさになる (`docs/editor.md#psd-の取り込み`)。
pub fn initial_state() -> (ProjectSession, Document) {
    ProjectSession::from_project(Project::empty(UNTITLED), None)
}

/// 開いているプロジェクトを `next` (開き直した・作ったもの) に置き換える。Undo の上限 (エディタ全体の設定。
/// `crate::history`) は前のドキュメントから引き継ぐ。
pub(crate) fn replace_document(
    session: &mut ProjectSession,
    document: &mut Document,
    (next_session, next_document): (ProjectSession, Document),
) {
    let limit = document.undo_limit();
    *session = next_session;
    *document = next_document.with_undo_limit(limit);
}

/// 新しいプロジェクトの名前 (保存するまで)。
const UNTITLED: &str = "Untitled";

/// PSD から新しいプロジェクトを作る: `options.name` (PSD のファイル名) の空のプロジェクトに取り込む。
/// 空のモデルへの取り込みなので、キャンバスは PSD の大きさになる。取り込みは Undo の 1 単位で、
/// 未保存の状態から始まる (Undo すると空のプロジェクトに戻る)。
pub(crate) fn from_psd(
    bytes: &[u8],
    options: &PsdImportOptions,
) -> IpcResult<(ProjectSession, Document, Vec<PsdImportWarning>)> {
    let (session, mut document) =
        ProjectSession::from_project(Project::empty(options.name.clone()), None);
    let import = import::import_psd(document.model(), bytes, options)?;
    document.apply(import.command)?;
    Ok((session, document, import.warnings))
}

/// フロントエンドに見せるプロジェクトの状態。
#[derive(Debug, Serialize, TS)]
pub struct ProjectStatus {
    pub name: String,
    /// 保存先のディレクトリ。未保存なら `null`。
    pub path: Option<String>,
    /// 最後に保存 (または開いた) ときから変更があるか。
    pub modified: bool,
}

pub(crate) fn status(session: &ProjectSession, document: &Document) -> ProjectStatus {
    ProjectStatus {
        name: session.meta.name.clone(),
        path: session.path.as_ref().map(|p| p.display().to_string()),
        modified: document.is_modified() || session.meta_modified,
    }
}

/// ロックの順序は常に `document` → `project`。
pub(crate) fn lock_both(
    state: &AppState,
) -> IpcResult<(MutexGuard<'_, Document>, MutexGuard<'_, ProjectSession>)> {
    let document = state
        .document
        .lock()
        .map_err(|_| EditorError::LockPoisoned { what: "document" })?;
    let project = state
        .project
        .lock()
        .map_err(|_| EditorError::LockPoisoned { what: "project" })?;
    Ok((document, project))
}

/// フロントエンドから受け取ったパス。絶対パスだけを受け付ける。
/// 相対パスはプロセスの作業ディレクトリ (開発時は `apps/editor/src-tauri`) を基準にしてしまうため拒否する
/// (パスはファイルダイアログで選ぶ。`docs/editor.md#ファイルダイアログ`)。
pub(crate) fn absolute_path(path: &str) -> IpcResult<PathBuf> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(EditorError::EmptyPath.into());
    }
    let path = PathBuf::from(trimmed);
    if !path.is_absolute() {
        return Err(EditorError::RelativePath(trimmed.to_owned()).into());
    }
    Ok(path)
}

/// 絶対パスで、拡張子が `.ugoeproj` でなければ付け足す。
fn project_dir(path: &str) -> IpcResult<PathBuf> {
    let path = absolute_path(path)?;
    if path.extension().is_some_and(|e| e == PROJECT_EXTENSION) {
        return Ok(path);
    }
    let mut name = path.as_os_str().to_owned();
    name.push(".");
    name.push(PROJECT_EXTENSION);
    Ok(PathBuf::from(name))
}

/// ディレクトリ名から拡張子を除いたもの。
fn stem(path: &Path) -> Option<String> {
    path.file_stem().map(|s| s.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn project_status(state: State<'_, AppState>) -> IpcResult<ProjectStatus> {
    let (document, session) = lock_both(&state)?;
    Ok(status(&session, &document))
}

/// 空のモデルの新しいプロジェクトにする。未保存の変更は破棄する (確認はフロントエンドで行う)。
#[tauri::command]
pub fn new_project(state: State<'_, AppState>) -> IpcResult<ProjectStatus> {
    let (mut document, mut session) = lock_both(&state)?;
    replace_document(
        &mut session,
        &mut document,
        ProjectSession::from_project(Project::empty(UNTITLED), None),
    );
    Ok(status(&session, &document))
}

/// PSD ファイルから新しいプロジェクトを作る (File > New from PSD…)。プロジェクトの名前は
/// ファイル名 (拡張子を除く)、キャンバスは PSD の大きさ。未保存の変更は破棄する (確認はフロントエンドで行う)。
/// 失敗したら今のプロジェクトはそのまま。戻り値は取り込みの警告。
#[tauri::command]
pub async fn new_project_from_psd(
    app: AppHandle,
    path: String,
    mesh_options: Option<AutoMeshOptions>,
) -> IpcResult<Vec<PsdImportWarning>> {
    blocking(app, move |state| {
        // 読み込みと自動メッシュはロックの外で行う
        let (bytes, options) = crate::psd::read_psd(&path, mesh_options)?;
        let (mut new_session, new_document, warnings) = from_psd(&bytes, &options)?;
        new_session.proposals.set_psd(absolute_path(&path)?);
        let (mut document, mut session) = lock_both(state)?;
        replace_document(&mut session, &mut document, (new_session, new_document));
        Ok(warnings)
    })
    .await
}

/// `.ugoeproj` ディレクトリを開く。失敗したら今のプロジェクトはそのまま。
#[tauri::command]
pub async fn open_project(app: AppHandle, path: String) -> IpcResult<ProjectStatus> {
    blocking(app, move |state| {
        let dir = absolute_path(&path)?;
        let (project, disk) = Project::load_tracked(&dir)?;
        let (mut document, mut session) = lock_both(state)?;
        replace_document(
            &mut session,
            &mut document,
            ProjectSession::from_project(project, Some((dir, disk))),
        );
        Ok(status(&session, &document))
    })
    .await
}

/// 開いているプロジェクトを保存先から開き直す (外で書き換えられたとき)。未保存の変更は破棄する
/// (確認はフロントエンドで行う)。AI への送信の「今後は確かめない」は同じプロジェクトなので残す。
/// AI の提案は前のドキュメントに対するものなので消す。失敗したら今のプロジェクトはそのまま。
#[tauri::command]
pub async fn reload_project(app: AppHandle) -> IpcResult<ProjectStatus> {
    blocking(app, |state| {
        let (mut document, mut session) = lock_both(state)?;
        let dir = session.path.clone().ok_or(EditorError::NotSaved)?;
        let (project, disk) = Project::load_tracked(&dir)?;
        let consent = std::mem::take(session.ai_consent_mut());
        replace_document(
            &mut session,
            &mut document,
            ProjectSession::from_project(project, Some((dir, disk))),
        );
        *session.ai_consent_mut() = consent;
        Ok(status(&session, &document))
    })
    .await
}

/// 開いたとき (または最後に保存したとき) から、保存先のファイルが外で変わったか。変わっていなければ `null`。
/// 保存先がなければ `null`。フロントエンドがウィンドウに戻ったとき・一定の間隔・保存の前に呼ぶ。
#[tauri::command]
pub async fn check_external_change(app: AppHandle) -> IpcResult<Option<ExternalChange>> {
    blocking(app, |state| {
        // 保存とぶつからないよう、ロックを持ったまま比べる (保存は `document` → `project` の順にロックする)
        crate::ai::lock_project(state)?.external_change()
    })
    .await
}

/// 開いたときの (または前回保存した) 場所に保存する。開いた後で外で書き換えられていたら、
/// `overwrite` が真でない限り書かずに [`EditorError::ExternalChange`] を返す。
#[tauri::command]
pub async fn save_project(app: AppHandle, overwrite: Option<bool>) -> IpcResult<ProjectStatus> {
    blocking(app, move |state| {
        let (mut document, mut session) = lock_both(state)?;
        let dir = session.path.clone().ok_or(EditorError::NotSavedUseSaveAs)?;
        if !overwrite.unwrap_or(false) && session.external_change()?.is_some() {
            return Err(EditorError::ExternalChange.into());
        }
        session.save_to(&mut document, dir)?;
        Ok(status(&session, &document))
    })
    .await
}

/// 別の場所に保存し、以後の保存先をそこにする。拡張子 `.ugoeproj` がなければ付ける。
/// プロジェクト名はディレクトリ名 (拡張子を除く) にする。
#[tauri::command]
pub async fn save_project_as(app: AppHandle, path: String) -> IpcResult<ProjectStatus> {
    blocking(app, move |state| {
        let dir = project_dir(&path)?;
        let (mut document, mut session) = lock_both(state)?;
        save_as(&mut document, &mut session, dir)?;
        Ok(status(&session, &document))
    })
    .await
}

/// [`save_project_as`] の中身 (テストから呼ぶ)。失敗したらメタ情報は元のまま。
fn save_as(document: &mut Document, session: &mut ProjectSession, dir: PathBuf) -> IpcResult<()> {
    let (old_meta, old_modified) = (session.meta.clone(), session.meta_modified);
    if let Some(name) = stem(&dir) {
        session.meta.name = name;
    }
    // 前の保存先からの相対パスで残したものを、新しい保存先から指し直す
    crate::dress_up::move_to(session, &dir);
    if let Err(e) = session.save_to(document, dir) {
        (session.meta, session.meta_modified) = (old_meta, old_modified);
        return Err(e);
    }
    Ok(())
}

/// 今のドキュメントを `.ugoe` バンドルのバイト列で返す (ビューポートがテクスチャ付きで表示するため)。
/// JSON を経由しないよう、生のバイト列 (`tauri::ipc::Response`) で送る (`docs/editor.md#ビューポート`)。
/// 編集のたびに呼ぶので、ロックの中では写しを取るだけにし、バンドル (PNG を含む) はロックの外で作る。
#[tauri::command]
pub async fn get_model_bundle(app: AppHandle) -> IpcResult<tauri::ipc::Response> {
    blocking(app, |state| {
        let project = {
            let (document, session) = lock_both(state)?;
            session.to_project(&document)
        };
        let bytes = ugoe_editor_core::export_bundle(&project)?;
        Ok(tauri::ipc::Response::new(bytes))
    })
    .await
}

/// プロジェクトに残した書き出しの設定 (`project.json` の `editor.export`、`docs/editor.md#書き出し`)。
#[tauri::command]
pub fn get_export_settings(state: State<'_, AppState>) -> IpcResult<ExportSettings> {
    let (_document, session) = lock_both(&state)?;
    Ok(session.meta.export_settings())
}

/// 書き出しの設定をプロジェクトに残す (保存するまでファイルには書かない。変われば未保存の印が付く)。
/// Undo の対象ではない。範囲の外ならエラーで、設定はそのまま。
#[tauri::command]
pub fn set_export_settings(
    state: State<'_, AppState>,
    settings: ExportSettings,
) -> IpcResult<ProjectStatus> {
    let (document, mut session) = lock_both(&state)?;
    if session.meta.set_export_settings(settings)? {
        session.meta_modified = true;
    }
    Ok(status(&session, &document))
}

/// プロジェクトに残したプレビューの光源 (`project.json` の `editor.lighting`、`docs/editor.md#ライティングの画面`)。
#[tauri::command]
pub fn get_preview_lighting(state: State<'_, AppState>) -> IpcResult<PreviewLighting> {
    let (_document, session) = lock_both(&state)?;
    Ok(session.meta.preview_lighting())
}

/// プレビューの光源をプロジェクトに残す (保存するまでファイルには書かない。変われば未保存の印が付く)。
/// Undo の対象ではない。範囲の外ならエラーで、設定はそのまま。
#[tauri::command]
pub fn set_preview_lighting(
    state: State<'_, AppState>,
    lighting: PreviewLighting,
) -> IpcResult<ProjectStatus> {
    let (document, mut session) = lock_both(&state)?;
    if session.meta.set_preview_lighting(lighting)? {
        session.meta_modified = true;
    }
    Ok(status(&session, &document))
}

/// プロジェクトの書き出しの設定に従って (アトラス化は任意)、`.ugoe` バンドルとしてファイルに書き出す。
/// 保存状態は変えない。書き出しの前後のテクスチャの枚数と描画の状態の切り替えの見積もりを返す。
#[tauri::command]
pub async fn export_bundle(app: AppHandle, path: String) -> IpcResult<ExportReport> {
    blocking(app, move |state| {
        let path = absolute_path(&path)?;
        let (project, settings) = {
            let (document, session) = lock_both(state)?;
            (
                session.to_project(&document),
                session.meta.export_settings(),
            )
        };
        let (bytes, report) = export_bundle_with_settings(&project, &settings)?;
        crate::atomic_write::write_atomic(&path, &bytes).map_err(|e| EditorError::io(&path, e))?;
        Ok(report)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 開き直しても Undo の上限 (エディタの設定) は変わらず、履歴は新しいドキュメントのものになる (T1605)。
    #[test]
    fn replacing_the_document_keeps_the_undo_limit() {
        let (mut session, mut document) = initial_state();
        document.set_undo_limit(7);
        document
            .apply(ugoe_editor_core::Command::AddParameter {
                parameter: ugoe_model::fixtures::single_quad().parameters[0].clone(),
            })
            .unwrap();
        assert!(document.can_undo());
        replace_document(
            &mut session,
            &mut document,
            ProjectSession::from_project(Project::empty("next"), None),
        );
        assert_eq!(document.undo_limit(), 7);
        assert!(!document.can_undo());
        assert_eq!(session.meta.name, "next");
    }

    #[test]
    fn project_dir_appends_extension() {
        let dir = std::env::temp_dir();
        assert_eq!(
            project_dir(&dir.join("a").display().to_string()).unwrap(),
            dir.join("a.ugoeproj")
        );
        assert_eq!(
            project_dir(&dir.join("a.ugoeproj").display().to_string()).unwrap(),
            dir.join("a.ugoeproj")
        );
        assert!(project_dir(" ").is_err());
    }

    #[test]
    fn relative_paths_are_rejected() {
        let err = absolute_path("test.ugoeproj").unwrap_err();
        assert!(err.to_string().contains("must be absolute"), "{err}");
        assert!(project_dir("test").is_err());
        assert!(crate::psd::read_psd("layers.psd", None).is_err());
        let dir = std::env::temp_dir();
        assert_eq!(absolute_path(&format!(" {} ", dir.display())).unwrap(), dir);
    }

    #[test]
    fn initial_state_is_empty_and_can_be_exported() {
        let (session, document) = initial_state();
        assert!(!document.is_modified());
        let model = document.model();
        assert!(model.parts.is_empty() && model.meshes.is_empty() && model.textures.is_empty());
        let bytes = ugoe_editor_core::export_bundle(&session.to_project(&document)).unwrap();
        let bundle = ugoe_format::read_bundle(&bytes).unwrap();
        assert!(bundle.model.meshes.is_empty());
    }

    /// 書き出しの設定は未保存の印を付け、保存するとプロジェクトに残る。アトラス化するとテクスチャが減る。
    #[test]
    fn export_settings_are_saved_with_the_project() {
        let (session, document, _) =
            from_psd(&character_psd(), &PsdImportOptions::new("character")).unwrap();
        let dir = std::env::temp_dir().join(format!("t0639-{}.ugoeproj", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        session.to_project(&document).save(&dir).unwrap();
        let (project, disk) = Project::load_tracked(&dir).unwrap();
        let (mut session, document) =
            ProjectSession::from_project(project, Some((dir.clone(), disk)));
        assert!(!status(&session, &document).modified);

        let settings = ExportSettings {
            atlas: true,
            ..ExportSettings::default()
        };
        assert!(session.meta.set_export_settings(settings).unwrap());
        session.meta_modified = true;
        assert!(status(&session, &document).modified);
        session.to_project(&document).save(&dir).unwrap();
        let loaded = Project::load(&dir).unwrap();
        assert_eq!(loaded.meta.export_settings(), settings);

        let (bytes, report) = export_bundle_with_settings(&loaded, &settings).unwrap();
        assert!(report.before.textures > 1, "{report:?}");
        assert_eq!(report.after.textures, 1, "{report:?}");
        assert!(report.after.state_changes <= report.before.state_changes);
        assert_eq!(report.bytes, bytes.len());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// 外での書き換えに気づき、自分の保存は外の変更と取り違えない (`docs/editor.md#外での書き換え`)。
    #[test]
    fn external_changes_are_detected_but_own_saves_are_not() {
        let (session, document, _) =
            from_psd(&character_psd(), &PsdImportOptions::new("character")).unwrap();
        let dir = std::env::temp_dir().join(format!("t0641-{}.ugoeproj", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        session.to_project(&document).save(&dir).unwrap();
        let (project, disk) = Project::load_tracked(&dir).unwrap();
        let (mut session, mut document) =
            ProjectSession::from_project(project, Some((dir.clone(), disk)));
        assert_eq!(session.external_change().unwrap(), None);

        // 自分の保存 (モデルを変えて)
        let mut parameter = ugoe_model::fixtures::single_quad().parameters[0].clone();
        parameter.id = "Extra".into();
        document
            .apply(ugoe_editor_core::Command::AddParameter { parameter })
            .unwrap();
        session.save_to(&mut document, dir.clone()).unwrap();
        assert_eq!(session.external_change().unwrap(), None);
        assert!(!status(&session, &document).modified);

        // 外での書き換え
        let meta = dir.join("project.json");
        let text = std::fs::read_to_string(&meta).unwrap();
        std::fs::write(&meta, text.replace("character", "renamed")).unwrap();
        let change = session.external_change().unwrap().unwrap();
        assert_eq!(change.files, vec!["project.json"]);
        assert!(!change.missing);
        // 同じ外の状態なら目印も同じ
        assert_eq!(
            session.external_change().unwrap().unwrap().token,
            change.token
        );

        // 上書きの保存の後は、もう変わっていない
        session.save_to(&mut document, dir.clone()).unwrap();
        assert_eq!(session.external_change().unwrap(), None);
        assert!(
            std::fs::read_to_string(&meta)
                .unwrap()
                .contains("character")
        );

        // 消されたら missing
        std::fs::remove_dir_all(&dir).unwrap();
        let change = session.external_change().unwrap().unwrap();
        assert!(change.missing && change.files.is_empty());
    }

    /// 保存先のないプロジェクトは比べない。
    #[test]
    fn unsaved_project_has_no_external_change() {
        let (session, _) = initial_state();
        assert_eq!(session.external_change().unwrap(), None);
    }

    /// 別のフォルダーに保存し直しても (Save As)、プレビューの着せ替えの一覧は同じファイルを指す
    /// (前の保存先からの相対パスで残したものも)。
    #[test]
    fn save_as_keeps_the_dress_up_modules() {
        let root = std::env::temp_dir().join(format!("t0669-save-as-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (mut session, mut document) = initial_state();
        save_as(&mut document, &mut session, root.join("work/a.ugoeproj")).unwrap();
        let hair = root.join("work/mods/hair.ugoemod").display().to_string();
        let modules = vec![crate::dress_up::PreviewModule {
            path: hair,
            attached: true,
        }];
        crate::dress_up::set_modules(&mut session, &modules);
        save_as(
            &mut document,
            &mut session,
            root.join("elsewhere/deeper/b.ugoeproj"),
        )
        .unwrap();
        assert_eq!(crate::dress_up::modules(&session), modules);
        assert_eq!(status(&session, &document).name, "b");
        // 開き直しても同じ
        let (project, disk) = Project::load_tracked(session.path().unwrap()).unwrap();
        let path = session.path().unwrap().to_path_buf();
        let (reopened, _) = ProjectSession::from_project(project, Some((path, disk)));
        assert_eq!(crate::dress_up::modules(&reopened), modules);
        std::fs::remove_dir_all(&root).unwrap();
    }

    fn character_psd() -> Vec<u8> {
        std::fs::read(crate::bindings::repo_root().join("assets/fixtures/character.psd")).unwrap()
    }

    /// 起動時の状態に取り込むと、キャンバスが PSD の大きさになり、キャンバスの警告は出ない。
    #[test]
    fn importing_into_initial_state_uses_psd_canvas() {
        let (_, mut document) = initial_state();
        let import = import::import_psd(
            document.model(),
            &character_psd(),
            &PsdImportOptions::new("character"),
        )
        .unwrap();
        assert!(import.warnings.is_empty(), "{:?}", import.warnings);
        document.apply(import.command).unwrap();
        let canvas = &document.model().meta.canvas;
        assert_eq!((canvas.width, canvas.height), (320.0, 400.0));
    }

    #[test]
    fn new_project_from_psd_is_named_after_the_file() {
        let (session, mut document, warnings) =
            from_psd(&character_psd(), &PsdImportOptions::new("character")).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        let status = status(&session, &document);
        assert_eq!(status.name, "character");
        assert_eq!(status.path, None);
        assert!(status.modified);
        let canvas = &document.model().meta.canvas;
        assert_eq!((canvas.width, canvas.height), (320.0, 400.0));
        assert!(!document.model().meshes.is_empty());
        // 書き出せる。Undo で空のプロジェクトに戻る。
        ugoe_editor_core::export_bundle(&session.to_project(&document)).unwrap();
        assert!(document.undo());
        assert!(document.model().meshes.is_empty());
        assert!(!document.is_modified());
    }

    #[test]
    fn new_project_from_psd_rejects_invalid_files() {
        assert!(from_psd(b"not a psd", &PsdImportOptions::new("broken")).is_err());
    }
}
