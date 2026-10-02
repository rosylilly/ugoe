//! 親のワープの格子からのはみ出しの検出 (`ugoe_editor_core::overflow`)。
//! ビューポートのはみ出しの表示と「Verify」のパネル (`src/lib/panels/verify/`) が使う。モデルは変えない。

use tauri::State;
use ugoe_editor_core::Message;
use ugoe_editor_core::overflow::{self, DEFAULT_TOLERANCE, ElementOverflow, OverflowReport};
use ugoe_editor_core::space::ParameterValues;

use crate::AppState;
use crate::error::IpcResult;

/// パラメータの値 `parameters` (含まないパラメータは既定値) で、親のワープの格子の外にある点。
/// `tolerance` は UV の許容距離 (省略すると既定値)。
#[tauri::command]
pub fn find_overflows(
    state: State<'_, AppState>,
    parameters: ParameterValues,
    tolerance: Option<f32>,
) -> IpcResult<Vec<ElementOverflow>> {
    let doc = crate::commands::lock(&state)?;
    let tolerance = tolerance.unwrap_or(DEFAULT_TOLERANCE);
    overflow::overflows_at(doc.model(), &parameters, tolerance).map_err(Message::from)
}

/// 親がワープのすべての要素について、キーフォームのすべてのセルではみ出しを調べる。
#[tauri::command]
pub fn verify_overflows(
    state: State<'_, AppState>,
    tolerance: Option<f32>,
) -> IpcResult<OverflowReport> {
    let doc = crate::commands::lock(&state)?;
    let tolerance = tolerance.unwrap_or(DEFAULT_TOLERANCE);
    overflow::verify_overflows(doc.model(), tolerance).map_err(Message::from)
}
