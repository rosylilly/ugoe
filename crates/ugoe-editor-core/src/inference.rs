//! キーフォームの推定のコマンドと、`derived` のセルの自動の作り直し
//! (`docs/spec/keyform-inference.md#推定の実行`、`docs/editor.md#推定とミラー`)。

use ugoe_model::deformer::DeformerKind;
use ugoe_model::{KeyformBinding, KeyformGrid, KeyformInference, KeyformOrigin, Model};
use ugoe_rigging::{MirrorOptions, infer_additive_target, infer_depth_rotation, mirror_keyforms};

use crate::command::{InferMethod, KeyformCell, KeyformTarget};
use crate::document::EditError;
use crate::grid;

/// 型によらない格子の見え方 (軸・セル数・由来・推定の設定)。
struct GridView<'a> {
    bindings: &'a [KeyformBinding],
    len: usize,
    origins: &'a mut Vec<KeyformOrigin>,
    inference: Option<&'a KeyformInference>,
}

impl<'a> GridView<'a> {
    fn new<T>(grid: &'a mut KeyformGrid<T>) -> Self {
        let KeyformGrid {
            bindings,
            keyforms,
            origins,
            inference,
        } = grid;
        Self {
            bindings,
            len: keyforms.len(),
            origins,
            inference: inference.as_ref(),
        }
    }

    fn has_derived(&self) -> bool {
        self.origins.contains(&KeyformOrigin::Derived)
    }
}

/// `target` の要素の格子。要素がなければ [`EditError::NotFound`]。
fn grid_view<'a>(model: &'a mut Model, target: &KeyformTarget) -> Result<GridView<'a>, EditError> {
    let not_found = |kind| EditError::NotFound {
        kind,
        id: match target {
            KeyformTarget::Part(id) => id.to_string(),
            KeyformTarget::Mesh(id) => id.to_string(),
            KeyformTarget::Deformer(id) => id.to_string(),
            KeyformTarget::Glue(id) => id.clone(),
        },
    };
    match target {
        KeyformTarget::Part(id) => model
            .parts
            .iter_mut()
            .find(|p| &p.id == id)
            .map(|p| GridView::new(&mut p.keyforms))
            .ok_or_else(|| not_found("part")),
        KeyformTarget::Mesh(id) => model
            .meshes
            .iter_mut()
            .find(|m| &m.id == id)
            .map(|m| GridView::new(&mut m.keyforms))
            .ok_or_else(|| not_found("mesh")),
        KeyformTarget::Deformer(id) => {
            let deformer = model
                .deformers
                .iter_mut()
                .find(|d| &d.id == id)
                .ok_or_else(|| not_found("deformer"))?;
            match &mut deformer.kind {
                DeformerKind::Warp(warp) => Ok(GridView::new(&mut warp.keyforms)),
                DeformerKind::Rotation(rotation) => Ok(GridView::new(&mut rotation.keyforms)),
                DeformerKind::Unknown => Err(EditError::DeformerKindMismatch {
                    id: id.to_string(),
                    expected: "warp or rotation",
                }),
            }
        }
        KeyformTarget::Glue(id) => model
            .glues
            .iter_mut()
            .find(|g| &g.id == id)
            .map(|g| GridView::new(&mut g.keyforms))
            .ok_or_else(|| not_found("glue")),
    }
}

/// 格子の推定の設定から決まる方法。未知の方法 (新しい版のファイル) なら `None`。
fn grid_method(inference: Option<&KeyformInference>) -> Option<InferMethod> {
    match inference {
        None | Some(KeyformInference::Additive) => Some(InferMethod::Additive),
        Some(KeyformInference::DepthRotation(_)) => Some(InferMethod::DepthRotation),
        Some(KeyformInference::Unknown) => None,
    }
}

/// `method` で `target` の格子の `derived` のセルを作り直す。`FromGrid` は呼び出し側で解決しておく。
fn run(model: &mut Model, target: &KeyformTarget, method: InferMethod) -> Result<(), EditError> {
    let rigging_target = target.into();
    let result = match method {
        InferMethod::FromGrid | InferMethod::Additive => {
            infer_additive_target(model, &rigging_target)
        }
        InferMethod::DepthRotation => infer_depth_rotation(model, &rigging_target),
    };
    result.map(drop).map_err(|error| EditError::Inference {
        target: target.to_string(),
        error,
    })
}

/// [`crate::Command::InferKeyforms`]。
pub(crate) fn infer_keyforms(
    model: &mut Model,
    target: &KeyformTarget,
    method: InferMethod,
) -> Result<(), EditError> {
    let method = match method {
        InferMethod::FromGrid => {
            grid_method(grid_view(model, target)?.inference).ok_or_else(|| {
                EditError::UnsupportedInference {
                    target: target.to_string(),
                }
            })?
        }
        method => {
            grid_view(model, target)?;
            method
        }
    };
    run(model, target, method)
}

/// [`crate::Command::MirrorKeyforms`]。
pub(crate) fn mirror(
    model: &mut Model,
    source: &KeyformTarget,
    target: &KeyformTarget,
    options: MirrorOptions,
) -> Result<(), EditError> {
    grid_view(model, source)?;
    grid_view(model, target)?;
    mirror_keyforms(model, &source.into(), &target.into(), &options)
        .map(drop)
        .map_err(|error| EditError::Inference {
            target: target.to_string(),
            error,
        })
}

/// [`crate::Command::MarkKeyformAuthored`] / [`crate::Command::MarkKeyformDerived`]。
/// 格子を作り直すのは [`rederive`] (呼び出し側が `touched` に積む)。
pub(crate) fn mark(
    model: &mut Model,
    target: &KeyformTarget,
    cells: &[KeyformCell],
    origin: KeyformOrigin,
) -> Result<(), EditError> {
    let view = grid_view(model, target)?;
    let indices = cells
        .iter()
        .map(|cell| grid::cell_index_in(view.bindings, view.len, cell))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| EditError::InvalidCell {
            kind: "keyform grid",
            id: target.to_string(),
            error,
        })?;
    if view.origins.is_empty() {
        if origin == KeyformOrigin::Authored {
            return Ok(());
        }
        *view.origins = vec![KeyformOrigin::Authored; view.len];
    }
    for index in indices {
        if let Some(o) = view.origins.get_mut(index) {
            *o = origin;
        }
    }
    Ok(())
}

/// コマンドが編集した格子 (`touched`) の `derived` のセルを、格子の推定の設定で作り直す。
///
/// [`crate::Document::apply`] がコマンド (`Batch` なら全体) を適用した後に呼ぶ。
/// `derived` のセルがない格子、未知の推定の方法の格子、コマンドの途中で削除された要素は飛ばす。
pub(crate) fn rederive(model: &mut Model, touched: &[KeyformTarget]) -> Result<(), EditError> {
    for (i, target) in touched.iter().enumerate() {
        if touched[..i].contains(target) {
            continue;
        }
        let Ok(view) = grid_view(model, target) else {
            continue;
        };
        if !view.has_derived() {
            continue;
        }
        let Some(method) = grid_method(view.inference) else {
            continue;
        };
        run(model, target, method)?;
    }
    Ok(())
}
