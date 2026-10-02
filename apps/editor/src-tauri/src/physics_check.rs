//! 物理の警告 (`ugoe_editor_core::physics_check`)。「Verify」と「Physics」のパネル
//! (`src/lib/panels/physics/RestPoseWarnings.svelte`) が使う。モデルも物理も変えない。

use tauri::State;
use ugoe_editor_core::Message;
use ugoe_editor_core::physics_check::{self, RestPoseCollision};

use crate::AppState;
use crate::error::IpcResult;

/// パラメータの既定値での静止姿勢で、衝突体に当たっているリグの鎖。物理の設定がなければ空。
#[tauri::command]
pub fn check_physics(state: State<'_, AppState>) -> IpcResult<Vec<RestPoseCollision>> {
    let doc = crate::commands::lock(&state)?;
    let Some(physics) = doc.physics() else {
        return Ok(Vec::new());
    };
    physics_check::rest_pose_collisions(doc.model(), physics).map_err(Message::from)
}
