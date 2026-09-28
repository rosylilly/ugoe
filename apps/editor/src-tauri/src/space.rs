//! 要素の親の座標系とキャンバス座標の変換 (`ugoe_editor_core::space`)。
//! ビューポートのフォーム編集ツール (`src/lib/viewport/tools/form/`) が使う。モデルは変えない。

use tauri::State;
use ugoe_editor_core::KeyformTarget;
use ugoe_editor_core::space::{self, ParameterValues, Vec2};

use crate::AppState;

fn to_vec2(points: &[[f32; 2]]) -> Vec<Vec2> {
    points.iter().map(|&[x, y]| Vec2::new(x, y)).collect()
}

fn to_pairs(points: Vec<Vec2>) -> Vec<[f32; 2]> {
    points.into_iter().map(|p| [p.x, p.y]).collect()
}

/// `target` の親の座標系の点 `points` をキャンバス座標に写す。
/// `parameters` はビューポートの表示中の値 (含まないパラメータは既定値)。
#[tauri::command]
pub fn local_to_canvas(
    state: State<'_, AppState>,
    target: KeyformTarget,
    parameters: ParameterValues,
    points: Vec<[f32; 2]>,
) -> Result<Vec<[f32; 2]>, String> {
    let doc = crate::commands::lock(&state)?;
    space::local_to_canvas(doc.model(), &target, &parameters, &to_vec2(&points))
        .map(to_pairs)
        .map_err(|e| e.to_string())
}

/// キャンバス座標の点 `points` を `target` の親の座標系に写す。`guesses` は反復の初期値
/// (ドラッグ前の位置、親の座標系)。
#[tauri::command]
pub fn canvas_to_local(
    state: State<'_, AppState>,
    target: KeyformTarget,
    parameters: ParameterValues,
    points: Vec<[f32; 2]>,
    guesses: Vec<[f32; 2]>,
) -> Result<Vec<[f32; 2]>, String> {
    let doc = crate::commands::lock(&state)?;
    space::canvas_to_local(
        doc.model(),
        &target,
        &parameters,
        &to_vec2(&points),
        &to_vec2(&guesses),
    )
    .map(to_pairs)
    .map_err(|e| e.to_string())
}
