//! キーフォームの推定のコマンドと、`derived` のセルの自動の作り直し
//! (`docs/spec/keyform-inference.md#推定の実行`、`docs/editor.md#推定とミラー`)。

use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use ugoe_model::deformer::DeformerKind;
use ugoe_model::{
    Deformer, Glue, KeyformBinding, KeyformGrid, KeyformInference, KeyformOrigin, Mesh,
    MirrorSource, Model, Part,
};
use ugoe_rigging::{
    InferScope, MirrorOptions, check_mirror, infer_additive_target, infer_depth_rotation,
    mirror_keyforms,
};

use crate::command::{InferMethod, KeyformCell, KeyformTarget};
use crate::document::EditError;
use crate::grid;

/// 自動の作り直しで、1 つの格子を作り直す回数の上限。ミラーが循環する (A が B を、B が A を
/// ミラー元にする) と作り直しが往復するので、値が変わらなくなるか、この回数で止める。
const MAX_PASSES: usize = 8;

/// 自動の作り直しで失敗したが、コマンドは拒否せずに続けたこと (`docs/editor.md#推定とミラー`)。
///
/// ミラーのやり直しの失敗 (元の頂点が許容距離を超えて動いた、対を外した、など) を報告する。
/// そのとき `target` の格子の `mirror` のセルは前の値のまま残る。
/// 対の片側の親を変えた (付け替えた、デフォーマで包んだ) 後に、その対がミラーし直せなくなった
/// ことも報告する。このときセルは付け替えで写し直した値のまま。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct RederiveWarning {
    /// ミラーのセルを作り直せなかった格子の要素。
    pub target: KeyformTarget,
    /// ミラー元 (`mirror_source`) の要素。
    pub source: KeyformTarget,
    /// 失敗の理由 (`ugoe_rigging::InferError` の表記)。
    pub message: String,
}

/// 型によらない格子の見え方 (軸・セル数・由来・推定の設定)。
struct GridView<'a> {
    bindings: &'a [KeyformBinding],
    len: usize,
    origins: &'a mut Vec<KeyformOrigin>,
    inference: &'a mut Option<KeyformInference>,
    mirror_source: &'a mut Option<MirrorSource>,
    /// セルを作り直す推定の方法 (`KeyformGrid::derivation` の値) の一覧。重複なし。
    methods: Vec<KeyformOrigin>,
}

impl<'a> GridView<'a> {
    fn new<T>(grid: &'a mut KeyformGrid<T>) -> Self {
        let mut methods = Vec::new();
        for method in (0..grid.keyforms.len()).filter_map(|i| grid.derivation(i)) {
            if !methods.contains(&method) {
                methods.push(method);
            }
        }
        let KeyformGrid {
            bindings,
            keyforms,
            origins,
            inference,
            mirror_source,
        } = grid;
        Self {
            bindings,
            len: keyforms.len(),
            origins,
            inference,
            mirror_source,
            methods,
        }
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

/// `method` で `target` の格子の `scope` のセルを作り直す。`FromGrid` は呼び出し側で解決しておく。
fn run(
    model: &mut Model,
    target: &KeyformTarget,
    method: InferMethod,
    scope: InferScope,
) -> Result<(), EditError> {
    let rigging_target = target.into();
    let result = match method {
        InferMethod::FromGrid | InferMethod::Additive => {
            infer_additive_target(model, &rigging_target, scope)
        }
        InferMethod::DepthRotation => infer_depth_rotation(model, &rigging_target, scope),
    };
    result.map(drop).map_err(|error| EditError::Inference {
        target: target.to_string(),
        error,
    })
}

/// [`crate::Command::InferKeyforms`]。`overwrite_derived` なら別の方法で作ったセルも書き換える。
pub(crate) fn infer_keyforms(
    model: &mut Model,
    target: &KeyformTarget,
    method: InferMethod,
    overwrite_derived: bool,
) -> Result<(), EditError> {
    let method = match method {
        InferMethod::FromGrid => grid_method(grid_view(model, target)?.inference.as_ref())
            .ok_or_else(|| EditError::UnsupportedInference {
                target: target.to_string(),
            })?,
        method => {
            grid_view(model, target)?;
            method
        }
    };
    let scope = if overwrite_derived {
        InferScope::AllDerived
    } else {
        InferScope::Own
    };
    run(model, target, method, scope)
}

/// [`crate::Command::MirrorKeyforms`]。先の `authored` 以外のセルをすべて書き換える
/// (`options.scope` は無視する)。
pub(crate) fn mirror(
    model: &mut Model,
    source: &KeyformTarget,
    target: &KeyformTarget,
    options: MirrorOptions,
) -> Result<(), EditError> {
    grid_view(model, source)?;
    grid_view(model, target)?;
    let options = MirrorOptions {
        scope: InferScope::AllDerived,
        ..options
    };
    mirror_keyforms(model, &source.into(), &target.into(), &options)
        .map(drop)
        .map_err(|error| EditError::Inference {
            target: target.to_string(),
            error,
        })
}

/// [`crate::Command::SetKeyformInference`]。格子を作り直すのは [`rederive`]。
pub(crate) fn set_inference(
    model: &mut Model,
    target: &KeyformTarget,
    inference: Option<KeyformInference>,
) -> Result<(), EditError> {
    let view = grid_view(model, target)?;
    if inference == Some(KeyformInference::Unknown) {
        return Err(EditError::UnsupportedInference {
            target: target.to_string(),
        });
    }
    *view.inference = inference;
    Ok(())
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

/// コマンドが編集した格子 (`touched`) の `derived` のセルを、セルごとに記録した推定の方法で作り直し、
/// その格子をミラー元にする格子のミラーのセルも作り直す (`docs/spec/keyform-inference.md#推定の実行`)。
///
/// [`crate::Document::apply`] がコマンド (`Batch` なら全体) を適用した後に呼ぶ。
///
/// - 編集した格子では、加算合成・深度からの回転のセルを作り直す。ミラーのセルは、ミラー元が
///   自分自身 (左右対称な 1 つの要素) のときだけ作り直す。
/// - 格子が変わったら、その格子を `mirror_source` にする格子 (依存する格子) のミラーのセルを
///   作り直し、続けてその格子の加算合成・深度からの回転のセルも作り直す。依存する格子が変われば、
///   さらにその先へ伝える。循環は、値が変わらなくなるか [`MAX_PASSES`] 回で止める。
/// - ミラーのやり直しに失敗したら (頂点が対応しなくなったなど)、その格子のミラーのセルはそのまま
///   にし、[`RederiveWarning`] を返す (格子とミラー元の組ごとに 1 つ)。加算合成・深度からの回転に
///   失敗したら、コマンドを拒否する。
/// - 作り直す方法のセルがない格子と、コマンドの途中で削除された要素は飛ばす。方法を記録していない
///   `derived` のセルは格子の `inference` の方法で作り直し、`inference` が未知なら作り直さない。
///   `depth_rotation` のセルは、格子の `inference` が深度からの回転でなければ作り直さない。
pub(crate) fn rederive(
    model: &mut Model,
    touched: &[KeyformTarget],
) -> Result<Vec<RederiveWarning>, EditError> {
    let mut warnings = Vec::new();
    // (格子, ミラー元が変わったか)。
    let mut queue: VecDeque<(KeyformTarget, bool)> = VecDeque::new();
    for target in touched {
        enqueue(&mut queue, target, false);
    }
    let mut passes: HashMap<KeyformTarget, usize> = HashMap::new();
    while let Some((target, source_changed)) = queue.pop_front() {
        let pass = passes.entry(target.clone()).or_default();
        if *pass >= MAX_PASSES {
            continue;
        }
        *pass += 1;
        // 編集した格子は、作り直しで値が変わらなくても依存する格子へ伝える。
        let edited = *pass == 1 && touched.contains(&target);
        let Some(before) = Snapshot::of(model, &target) else {
            continue;
        };
        rederive_grid(model, &target, source_changed, &mut warnings)?;
        if edited || Snapshot::of(model, &target).as_ref() != Some(&before) {
            for dependent in dependents(model, &target) {
                enqueue(&mut queue, &dependent, true);
            }
        }
    }
    Ok(warnings)
}

fn enqueue(queue: &mut VecDeque<(KeyformTarget, bool)>, target: &KeyformTarget, changed: bool) {
    match queue.iter_mut().find(|(t, _)| t == target) {
        Some((_, c)) => *c |= changed,
        None => queue.push_back((target.clone(), changed)),
    }
}

/// 親が変わった要素の左右の対が、まだミラーし直せるかを確かめる
/// (`docs/spec/keyform-inference.md#自動の作り直し`)。
///
/// `before` はコマンドの前、`after` は作り直しの後のモデル。親が変わったメッシュ・デフォーマごとに、
/// 対の相手とのミラー (どちらかの格子が相手を `mirror_source` にし、`mirror` のセルを持つもの) を
/// 記録した `axis` / `tolerance` で試す。前はミラーできたのに後はできなくなった組を `warnings` に
/// 積む (すでに積んだ組は積まない)。モデルは変えない。
pub(crate) fn check_reparented_pairs(
    before: &Model,
    after: &Model,
    warnings: &mut Vec<RederiveWarning>,
) {
    let parents: HashMap<KeyformTarget, Option<&ugoe_model::DeformerId>> = before
        .meshes
        .iter()
        .map(|m| (KeyformTarget::Mesh(m.id.clone()), m.parent.as_ref()))
        .chain(
            before
                .deformers
                .iter()
                .map(|d| (KeyformTarget::Deformer(d.id.clone()), d.parent.as_ref())),
        )
        .collect();
    let moved = after
        .meshes
        .iter()
        .map(|m| {
            (
                KeyformTarget::Mesh(m.id.clone()),
                m.parent.as_ref(),
                m.mirror.as_ref().map(|id| id.to_string()),
            )
        })
        .chain(after.deformers.iter().map(|d| {
            (
                KeyformTarget::Deformer(d.id.clone()),
                d.parent.as_ref(),
                d.mirror.as_ref().map(|id| id.to_string()),
            )
        }))
        .filter(|(target, parent, _)| parents.get(target).is_some_and(|p| p != parent));
    let mut pairs: Vec<(KeyformTarget, KeyformTarget)> = Vec::new();
    for (element, _, mirror) in moved {
        let Some(partner) = mirror.filter(|m| *m != element_id(&element)) else {
            continue;
        };
        let partner = same_kind(&element, &partner);
        for (from, to) in [(&element, &partner), (&partner, &element)] {
            let pair = (from.clone(), to.clone());
            if !pairs.contains(&pair) {
                pairs.push(pair);
            }
        }
    }
    for (from, to) in pairs {
        let Some(source) = mirrored_from(after, &to).filter(|s| s.id == element_id(&from)) else {
            continue;
        };
        if warnings.iter().any(|w| w.target == to && w.source == from) {
            continue;
        }
        let options = MirrorOptions {
            axis: source.axis,
            tolerance: source.tolerance,
            overwrite_authored: false,
            scope: InferScope::Own,
        };
        let (rigging_from, rigging_to) = ((&from).into(), (&to).into());
        if check_mirror(before, &rigging_from, &rigging_to, &options).is_err() {
            continue; // 前からミラーできなかった (そのときに警告済み)。
        }
        if let Err(error) = check_mirror(after, &rigging_from, &rigging_to, &options) {
            warnings.push(RederiveWarning {
                target: to,
                source: from,
                message: error.to_string(),
            });
        }
    }
}

/// `target` の格子のミラー元の記録。`mirror` のセルがなければ `None` (ミラーし直さないので)。
fn mirrored_from(model: &Model, target: &KeyformTarget) -> Option<MirrorSource> {
    fn of<T>(grid: &KeyformGrid<T>) -> Option<MirrorSource> {
        (0..grid.keyforms.len())
            .any(|i| grid.derivation(i) == Some(KeyformOrigin::Mirror))
            .then(|| grid.mirror_source.clone())
            .flatten()
    }
    match target {
        KeyformTarget::Mesh(id) => of(&model.meshes.iter().find(|m| &m.id == id)?.keyforms),
        KeyformTarget::Deformer(id) => match &model.deformers.iter().find(|d| &d.id == id)?.kind {
            DeformerKind::Warp(w) => of(&w.keyforms),
            DeformerKind::Rotation(r) => of(&r.keyforms),
            DeformerKind::Unknown => None,
        },
        KeyformTarget::Part(_) | KeyformTarget::Glue(_) => None,
    }
}

/// 1 つの格子を作り直す。`source_changed` ならミラーのセルもミラー元から作り直す。
/// ミラーのやり直しの失敗は `warnings` に積む。
fn rederive_grid(
    model: &mut Model,
    target: &KeyformTarget,
    source_changed: bool,
    warnings: &mut Vec<RederiveWarning>,
) -> Result<(), EditError> {
    let Ok(view) = grid_view(model, target) else {
        return Ok(());
    };
    let methods = view.methods;
    let depth = matches!(*view.inference, Some(KeyformInference::DepthRotation(_)));
    let mirror = view
        .mirror_source
        .clone()
        .filter(|source| source_changed || source.id == element_id(target));
    if let (true, Some(source)) = (methods.contains(&KeyformOrigin::Mirror), mirror) {
        let options = MirrorOptions {
            axis: source.axis,
            tolerance: source.tolerance,
            overwrite_authored: false,
            scope: InferScope::Own,
        };
        let from = same_kind(target, &source.id);
        // 失敗してもエラーのときはモデルを変えないので、ミラーのセルは前の値のまま残る。
        if let Err(error) = mirror_keyforms(model, &(&from).into(), &target.into(), &options) {
            let warning = RederiveWarning {
                target: target.clone(),
                source: from,
                message: error.to_string(),
            };
            // 循環で同じ格子を何度も作り直すことがあるので、同じ組の警告は 1 つにする。
            if !warnings
                .iter()
                .any(|w| w.target == warning.target && w.source == warning.source)
            {
                warnings.push(warning);
            }
        }
    }
    if depth && methods.contains(&KeyformOrigin::DepthRotation) {
        run(model, target, InferMethod::DepthRotation, InferScope::Own)?;
    }
    if methods.contains(&KeyformOrigin::Additive) {
        run(model, target, InferMethod::Additive, InferScope::Own)?;
    }
    Ok(())
}

/// `source` をミラー元 (`mirror_source`) にする、同じ種類の別の要素の格子。
fn dependents(model: &mut Model, source: &KeyformTarget) -> Vec<KeyformTarget> {
    let id = element_id(source);
    let depends = |grid: Option<&MirrorSource>| grid.is_some_and(|s| s.id == id);
    let ids: Vec<String> = match source {
        KeyformTarget::Part(_) => model
            .parts
            .iter()
            .filter(|p| depends(p.keyforms.mirror_source.as_ref()))
            .map(|p| p.id.to_string())
            .collect(),
        KeyformTarget::Mesh(_) => model
            .meshes
            .iter()
            .filter(|m| depends(m.keyforms.mirror_source.as_ref()))
            .map(|m| m.id.to_string())
            .collect(),
        KeyformTarget::Deformer(_) => model
            .deformers
            .iter()
            .filter(|d| match &d.kind {
                DeformerKind::Warp(w) => depends(w.keyforms.mirror_source.as_ref()),
                DeformerKind::Rotation(r) => depends(r.keyforms.mirror_source.as_ref()),
                DeformerKind::Unknown => false,
            })
            .map(|d| d.id.to_string())
            .collect(),
        KeyformTarget::Glue(_) => model
            .glues
            .iter()
            .filter(|g| depends(g.keyforms.mirror_source.as_ref()))
            .map(|g| g.id.clone())
            .collect(),
    };
    ids.into_iter()
        .filter(|dependent| *dependent != id)
        .map(|dependent| same_kind(source, &dependent))
        .collect()
}

/// 削除などで存在しなくなった要素をミラー元にしている格子から、ミラー元の記録を外す。
/// その格子のミラーのセルは作り直せなくなるので `authored` にする (値は変えない)。
pub(crate) fn forget_missing_mirror_sources(model: &mut Model) {
    let targets: Vec<KeyformTarget> = model
        .parts
        .iter()
        .map(|p| KeyformTarget::Part(p.id.clone()))
        .chain(
            model
                .meshes
                .iter()
                .map(|m| KeyformTarget::Mesh(m.id.clone())),
        )
        .chain(
            model
                .deformers
                .iter()
                .map(|d| KeyformTarget::Deformer(d.id.clone())),
        )
        .chain(
            model
                .glues
                .iter()
                .map(|g| KeyformTarget::Glue(g.id.clone())),
        )
        .collect();
    for target in targets {
        let Some(source) = grid_view(model, &target)
            .ok()
            .and_then(|view| view.mirror_source.clone())
        else {
            continue;
        };
        if Snapshot::of(model, &same_kind(&target, &source.id)).is_some() {
            continue;
        }
        if let Ok(view) = grid_view(model, &target) {
            *view.mirror_source = None;
            for origin in view.origins.iter_mut() {
                if *origin == KeyformOrigin::Mirror {
                    *origin = KeyformOrigin::Authored;
                }
            }
        }
    }
}

/// 要素の ID の文字列。
fn element_id(target: &KeyformTarget) -> String {
    match target {
        KeyformTarget::Part(id) => id.to_string(),
        KeyformTarget::Mesh(id) => id.to_string(),
        KeyformTarget::Deformer(id) => id.to_string(),
        KeyformTarget::Glue(id) => id.clone(),
    }
}

/// `target` と同じ種類の、ID が `id` の要素。
fn same_kind(target: &KeyformTarget, id: &str) -> KeyformTarget {
    match target {
        KeyformTarget::Part(_) => KeyformTarget::Part(id.into()),
        KeyformTarget::Mesh(_) => KeyformTarget::Mesh(id.into()),
        KeyformTarget::Deformer(_) => KeyformTarget::Deformer(id.into()),
        KeyformTarget::Glue(_) => KeyformTarget::Glue(id.to_owned()),
    }
}

/// 作り直しの前後で格子が変わったかを比べるための、要素の複製。
#[derive(PartialEq)]
enum Snapshot {
    Part(Part),
    Mesh(Mesh),
    Deformer(Deformer),
    Glue(Glue),
}

impl Snapshot {
    fn of(model: &Model, target: &KeyformTarget) -> Option<Self> {
        match target {
            KeyformTarget::Part(id) => model
                .parts
                .iter()
                .find(|p| &p.id == id)
                .cloned()
                .map(Self::Part),
            KeyformTarget::Mesh(id) => model
                .meshes
                .iter()
                .find(|m| &m.id == id)
                .cloned()
                .map(Self::Mesh),
            KeyformTarget::Deformer(id) => model
                .deformers
                .iter()
                .find(|d| &d.id == id)
                .cloned()
                .map(Self::Deformer),
            KeyformTarget::Glue(id) => model
                .glues
                .iter()
                .find(|g| &g.id == id)
                .cloned()
                .map(Self::Glue),
        }
    }
}
