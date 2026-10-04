//! エディタのバックエンド。ここは **薄い IPC 層** に保ち、編集ロジックは `ugoe-editor-core` に置く。
//! 設計: `docs/editor.md`。

// コマンドの許可の表 (build.rs が読む)。ここではその検査だけを行う (T0686)。
#[cfg(test)]
mod acl;
pub(crate) mod ai;
mod ai_provider;
mod atomic_write;
mod avatar;
pub mod bindings;
mod commands;
mod dress_up;
mod error;
mod history;
mod lighting;
mod mcp;
mod module;
mod overflow;
mod physics_check;
mod project;
mod proposals;
mod psd;
mod read_file;
mod sample;
mod screenshot;
mod space;
mod tracking;

use std::sync::Mutex;

use ugoe_editor_core::Document;

/// アプリ全体の状態。現状は単一ドキュメント。
/// 両方をロックするときは `document` → `project` の順にする。
#[derive(Debug)]
pub struct AppState {
    pub document: Mutex<Document>,
    /// 開いているプロジェクトの保存先・メタ情報など (`Document` の外のもの)。
    pub project: Mutex<project::ProjectSession>,
    /// 保存の書き込みを 1 つずつにする (ロックの外で書く保存が重ならず、書きかけのファイルを外の変更と取り違えない
    /// ように。`project::ProjectSession::begin_save`)。ロックの順序は `saving` → `document` → `project`。
    pub saving: Mutex<()>,
    /// AI の API キー (OS の資格情報の保存先と環境変数)。
    pub api_keys: ai::ApiKeys,
    /// 実行中の AI の送信の取り消しの印 (`ai::cancel_ai_requests`)。
    pub ai_jobs: ai::AiJobs,
    /// 選んだ AI のプロバイダ (エディタ全体の設定。起動時にアプリの設定のディレクトリから読む)。
    pub ai_provider: ai_provider::ProviderStore,
    /// エディタの中の MCP サーバ (エージェントをつなぐ。既定は無効。`docs/editor.md#エージェントをつなぐ`)。
    pub mcp: mcp::McpBridge,
    /// プレビューの本体だけの `.ugoe` (モジュールのプロジェクト。`module::get_preview_base_bundle`)。
    pub preview_base: module::PreviewBaseCache,
    /// Undo の上限 (エディタ全体の設定。起動時にアプリの設定のディレクトリから読む。`docs/editor.md#undo`)。
    pub undo_limit: history::UndoLimitStore,
}

pub fn run() {
    let (session, document) = project::initial_state();
    let state = AppState {
        document: Mutex::new(document),
        project: Mutex::new(session),
        saving: Mutex::new(()),
        api_keys: ai::ApiKeys::default(),
        ai_jobs: ai::AiJobs::default(),
        ai_provider: ai_provider::ProviderStore::default(),
        mcp: mcp::McpBridge::default(),
        preview_base: module::PreviewBaseCache::default(),
        undo_limit: history::UndoLimitStore::default(),
    };
    #[allow(clippy::expect_used)]
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .manage(state)
        .setup(|app| {
            use tauri::Manager;
            let loaded = match app.path().app_config_dir() {
                Ok(dir) => app.state::<AppState>().ai_provider.load(dir),
                Err(e) => Err(error::EditorError::Internal(e.to_string())),
            };
            // 読めなければ既定 (Claude) のまま起動する
            if let Err(e) = loaded {
                eprintln!("could not read the AI provider settings: {e}");
            }
            // 読めなければ既定の上限のまま起動する
            let state = app.state::<AppState>();
            let loaded = match app.path().app_config_dir() {
                Ok(dir) => state.undo_limit.load(dir),
                Err(e) => Err(error::EditorError::Internal(e.to_string())),
            };
            if let Err(e) = loaded {
                eprintln!("could not read the editor settings: {e}");
            }
            if let Ok(mut document) = state.document.lock() {
                document.set_undo_limit(state.undo_limit.get());
            }
            mcp::setup(app.handle());
            Ok(())
        })
        .on_permission_request(|_, kind| tracking::permission_response(kind))
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::apply_command,
            commands::get_model_json,
            commands::get_animations,
            commands::get_physics,
            commands::undo,
            commands::redo,
            history::history_status,
            history::set_undo_limit,
            project::project_status,
            project::new_project,
            project::new_project_from_psd,
            project::open_project,
            sample::open_sample_project,
            project::reload_project,
            project::check_external_change,
            project::save_project,
            project::save_project_as,
            project::get_export_settings,
            project::set_export_settings,
            project::get_preview_lighting,
            project::set_preview_lighting,
            project::export_bundle,
            project::get_model_bundle,
            module::module_status,
            module::read_module_base,
            module::new_module_project,
            module::open_module_with_base,
            module::export_module,
            module::module_open_problem,
            module::check_base_change,
            module::reload_base,
            module::get_preview_base_bundle,
            module::get_preview_module,
            module::compare_preview_module,
            psd::import_psd,
            space::local_to_canvas,
            space::canvas_to_local,
            overflow::find_overflows,
            overflow::verify_overflows,
            physics_check::check_physics,
            avatar::check_avatar_profile,
            ai::ai_settings,
            ai::set_ai_provider,
            ai::store_api_key,
            ai::delete_api_key,
            ai::set_ai_send_externally,
            ai::forget_ai_consent,
            ai::test_ai_connection,
            ai::cancel_ai_requests,
            proposals::get_proposals,
            proposals::classify_psd_layers,
            proposals::propose_rig_from_psd,
            proposals::regenerate_rig_proposal,
            proposals::propose_normal_maps,
            proposals::preview_proposal,
            proposals::get_proposal_preview_bundle,
            proposals::clear_proposal_preview,
            proposals::adopt_proposal,
            proposals::reject_proposal,
            proposals::dismiss_stale_rigs,
            mcp::mcp_bridge_status,
            mcp::set_mcp_bridge_enabled,
            mcp::set_mcp_bridge_port,
            mcp::regenerate_mcp_bridge_token,
            mcp::get_mcp_bridge_token,
            mcp::answer_mcp_approval,
            mcp::open_claude_desktop_bundle,
            mcp::save_claude_desktop_bundle,
            tracking::write_json_file,
            screenshot::write_png_file,
            dress_up::get_preview_modules,
            dress_up::set_preview_modules,
            dress_up::read_module_file,
            lighting::get_normal_map_group,
            lighting::get_normal_map_groups,
            lighting::generate_texture_normal_map,
            lighting::assign_texture_normal_map,
            lighting::get_texture_png,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // 終えるときにブリッジを止め、発見用のファイルを消す (待っている確認は拒否になる)
            if let tauri::RunEvent::Exit = event {
                use tauri::Manager;
                app.state::<AppState>().mcp.stop();
            }
        });
}
