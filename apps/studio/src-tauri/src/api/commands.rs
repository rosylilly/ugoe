//! 操作ウィンドウの「操作 API」と「ブラウザソース」のパネルが呼ぶ Tauri のコマンド (`apps/studio/src/lib/host.ts`)。
//! 様子が変わると `Monitor::Api` の通知でも届く。

use std::sync::Arc;

use super::ApiView;
use super::store::Permission;
use crate::Studio;
use crate::error::CommandError;

/// 今の様子。
#[tauri::command]
pub(crate) fn api_state(studio: tauri::State<'_, Arc<Studio>>) -> ApiView {
    studio.api_view()
}

/// 待ち受けを始める・止める (覚える。止めると接続はすべて切る)。
#[tauri::command]
pub(crate) fn set_api_enabled(studio: tauri::State<'_, Arc<Studio>>, enabled: bool) -> ApiView {
    studio.set_api_enabled(enabled);
    studio.api_view()
}

/// ポートを変える (待ち受けていれば始め直す)。1024 未満は断る。
#[tauri::command]
pub(crate) fn set_api_port(
    studio: tauri::State<'_, Arc<Studio>>,
    port: u16,
) -> Result<ApiView, CommandError> {
    studio.set_api_port(port)?;
    Ok(studio.api_view())
}

/// トークン (クライアントに入れてもらうために写す)。
#[tauri::command]
pub(crate) fn api_token(studio: tauri::State<'_, Arc<Studio>>) -> String {
    studio.api_token()
}

/// トークンを作り直す (古いトークンは使えなくなる)。新しいトークンを返す。
#[tauri::command]
pub(crate) fn regenerate_api_token(
    studio: tauri::State<'_, Arc<Studio>>,
) -> Result<String, CommandError> {
    studio.regenerate_api_token()
}

/// 承認を待つクライアントを `permission` で承認する。承認したクライアントの ID を返す
/// (承認を待つ間の ID とは違う。画面の「取り消す」はこの ID で取り消す)。
#[tauri::command]
pub(crate) fn approve_api_client(
    studio: tauri::State<'_, Arc<Studio>>,
    id: String,
    permission: Permission,
) -> Result<String, CommandError> {
    studio.approve_api_client(&id, permission)
}

/// 承認を待つクライアントを断る。
#[tauri::command]
pub(crate) fn deny_api_client(studio: tauri::State<'_, Arc<Studio>>, id: String) {
    studio.deny_api_client(&id);
}

/// 承認したクライアントの鍵を取り消す (つないでいれば切る)。
#[tauri::command]
pub(crate) fn revoke_api_client(studio: tauri::State<'_, Arc<Studio>>, id: String) {
    studio.revoke_api_client(&id);
}

/// 承認したクライアントの今の接続を切る。
#[tauri::command]
pub(crate) fn disconnect_api_client(studio: tauri::State<'_, Arc<Studio>>, id: String) {
    studio.disconnect_api_client(&id);
}

/// ブラウザソースを出す・やめる (覚える。やめるとページの接続はすべて閉じる。T0808)。
#[tauri::command]
pub(crate) fn set_source_enabled(studio: tauri::State<'_, Arc<Studio>>, enabled: bool) -> ApiView {
    studio.set_source_enabled(enabled);
    studio.api_view()
}

/// OBS のブラウザソースに入れる URL (トークンを含む)。
#[tauri::command]
pub(crate) fn source_url(studio: tauri::State<'_, Arc<Studio>>) -> String {
    studio.source_url()
}

/// ブラウザソースのトークンを作り直す (古い URL は使えなくなる)。新しい URL を返す。
#[tauri::command]
pub(crate) fn regenerate_source_token(
    studio: tauri::State<'_, Arc<Studio>>,
) -> Result<String, CommandError> {
    studio.regenerate_source_token()
}
