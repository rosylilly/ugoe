//! アバタープロファイルの準拠の検査 (`ugoe_editor_core::avatar::check`)。インスペクタの「Avatar profile」の節
//! (`src/lib/panels/inspector/avatar/AvatarSection.svelte`) が使う。モデルは変えない。

use tauri::State;
use ugoe_editor_core::avatar::{self, AvatarCheck};

use crate::AppState;

/// 今のモデルの、アバタープロファイルへの準拠の検査の結果。
#[tauri::command]
pub fn check_avatar_profile(state: State<'_, AppState>) -> Result<AvatarCheck, String> {
    let doc = crate::commands::lock(&state)?;
    Ok(avatar::check(doc.model()))
}
