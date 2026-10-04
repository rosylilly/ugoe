//! 深度推定による振り向きリグの提案 (`docs/ai-integration.md#深度推定による振り向きリグ`)。
//!
//! 1. 原画から [`DepthEstimator`] で相対的な深度マップを作る。
//! 2. ワープデフォーマの制御点 (基準の姿のキャンバス座標) で深度マップを標本化し、キャンバスの
//!    ピクセル単位の深度に直す。制御点を覆うメッシュの意味ラベルで補正する ([`label_depth`])。
//! 3. 深度・振り向きの軸のキー・深度からの回転の設定 (回転の中心と `angle_scale` はラベルから決める)
//!    を [`Command`] の列にして提案する。回転と投影は `ugoe-rigging` が決定的に行う
//!    (`docs/spec/keyform-inference.md#深度からの回転`)。
//!
//! 提案は複製したドキュメントに適用し、`ugoe_model::validate` を通ることを確かめてから返す (ADR 0006)。

use std::collections::{BTreeMap, BTreeSet};

use ugoe_core::Vec2;
use ugoe_editor_core::{ChildRef, Command, Document, InferMethod, KeyformCell, KeyformTarget};
use ugoe_model::parameter::ParameterKind;
use ugoe_model::{
    DeformerId, DeformerKind, DepthRotation, KeyformInference, KeyformOrigin, MeshId, Model,
    ParamId, Parameter, standard,
};
use ugoe_rigging::ParentWorld;
use ugoe_runtime::{CompiledModel, ModelInstance};

use crate::classify::{Label, LayerLabel};
use crate::depth::{DepthEstimator, DepthMap, DepthRequest};
use crate::grid;
use crate::{AiError, Proposal};

/// 相対的な深度を正規化するときに捨てる両端の割合 (外れ値に引きずられないように)。
pub(crate) const PERCENTILE: f32 = 0.02;
/// 深度の正規化で部位を標本化する格子の分割数 (一辺)。
const NORMALIZE_GRID: u32 = 48;

/// 振り向かせる部位。軸のパラメータ・起伏の大きさ・回転の中心の決め方が変わる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnRole {
    /// 頭 (`AngleX` / `AngleY`)。回転の中心は顔の外接矩形の中心。
    Head,
    /// 体 (`BodyAngleX` / `BodyAngleY`)。回転の中心は体の外接矩形の下端の中央 (腰)。
    Body,
}

impl TurnRole {
    /// 縦軸まわり (yaw) と横軸まわり (pitch) のパラメータ。
    pub fn parameters(self) -> (&'static str, &'static str) {
        match self {
            Self::Head => (standard::ANGLE_X, standard::ANGLE_Y),
            Self::Body => (standard::BODY_ANGLE_X, standard::BODY_ANGLE_Y),
        }
    }

    /// 最も手前の点の深度の、部位の幅に対する比。
    ///
    /// 絵の顔は実物より平たく描くので、実物 (鼻先が耳より顔の幅の半分ほど手前) より小さくする。
    pub fn relief(self) -> f32 {
        match self {
            Self::Head => 0.3,
            Self::Body => 0.2,
        }
    }

    /// パラメータの値に掛けて角度 (度) にする係数。標準パラメータの値域 (頭 ±30・体 ±10) を
    /// そのまま角度として使う。
    pub fn angle_scale(self) -> f32 {
        1.0
    }

    /// 部位の大きさと中心を決めるラベル。
    fn anchor_labels(self) -> &'static [Label] {
        match self {
            Self::Head => &[Label::Face],
            Self::Body => &[Label::Body, Label::Topwear],
        }
    }
}

/// 振り向きの対象。
#[derive(Debug, Clone, PartialEq)]
pub struct TurnTarget {
    /// ワープデフォーマ。親の座標系はアフィン (親なし・回転デフォーマ) でなければならない。
    pub deformer: DeformerId,
    /// 部位。`None` なら子孫のメッシュのラベルから決める (`face` があれば頭、`body` / `topwear` が
    /// あれば体)。
    pub role: Option<TurnRole>,
}

/// 意味ラベルによる深度の補正。値の単位は部位の起伏 (最も手前の点の深度) で、0 が輪郭、1 が最も手前。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LabelDepth {
    /// 推定した深度に `bias` を足す。
    Surface { bias: f32 },
    /// 推定した深度を `level` 以上にする (前髪は顔より手前、など)。
    AtLeast(f32),
    /// 推定した深度を `level` 以下にする。
    AtMost(f32),
    /// 推定を使わず `level` にする (後ろ髪は輪郭より奥、など)。
    Fixed(f32),
    /// 覆うメッシュとして数えない (背景など)。
    Ignore,
}

/// ラベルごとの深度の補正。
///
/// 単眼の深度推定は、平たく塗った絵では前後関係を取り違えやすい (前髪が額と同じ深さになる、
/// 後ろ髪が顔の横に並ぶ、など)。部位の前後関係はラベルで分かるので、それで上書きする。
pub fn label_depth(label: Label) -> LabelDepth {
    match label {
        Label::Nose => LabelDepth::Surface { bias: 0.1 },
        Label::HairFront => LabelDepth::AtLeast(0.8),
        Label::Eyewear => LabelDepth::AtLeast(1.0),
        Label::HairSide => LabelDepth::AtMost(0.3),
        Label::HairBack => LabelDepth::Fixed(-0.3),
        Label::Ear | Label::Earwear => LabelDepth::Fixed(-0.1),
        Label::Background => LabelDepth::Ignore,
        _ => LabelDepth::Surface { bias: 0.0 },
    }
}

impl LabelDepth {
    fn apply(self, estimated: f32) -> f32 {
        match self {
            Self::Surface { bias } => estimated + bias,
            Self::AtLeast(level) => estimated.max(level),
            Self::AtMost(level) => estimated.min(level),
            Self::Fixed(level) => level,
            Self::Ignore => estimated,
        }
    }
}

/// 1 つの対象について決めた振り向きの設定。
#[derive(Debug, Clone, PartialEq)]
pub struct TurnPlan {
    pub deformer: DeformerId,
    pub role: TurnRole,
    /// 制御点ごとの深度 (キャンバスのピクセル単位、手前が正)。
    pub depth: Vec<f32>,
    /// 起伏 (最も手前の点の深度、キャンバスのピクセル単位)。
    pub relief: f32,
    pub rotation: DepthRotation,
    /// 制御点ごとの、覆うメッシュのラベル (覆うものがなければ `None`)。
    pub labels: Vec<Option<Label>>,
}

/// レイヤーの分類の結果 ([`crate::classify`]) を、名前でメッシュに対応付ける。
///
/// PSD の取り込み (`ugoe_editor_core::import_psd`) はメッシュの名前をレイヤーの名前にするので、
/// `layer_path` の最後の名前と同じ名前のメッシュにラベルを付ける。同じ名前のメッシュやレイヤーが
/// 複数あって決められないものは付けない (ラベルのないメッシュは顔の表面として扱われる)。
/// 左右も要るなら [`crate::rig::mesh_parts`]。
pub fn mesh_labels(model: &Model, layers: &[LayerLabel]) -> BTreeMap<MeshId, Label> {
    crate::rig::mesh_parts(model, layers)
        .into_iter()
        .map(|(id, part)| (id, part.label))
        .collect()
}

/// 深度を推定して振り向きのリグを提案する。
///
/// - `request.image` はキャンバス全体を描いた原画 (PSD の合成画像など)。
/// - `labels` はメッシュの意味ラベル ([`mesh_labels`] でレイヤーの分類から作れる)。
///   ないメッシュは顔の表面として扱う。
///
/// 返す提案は `model` に `Command::Batch` として適用でき、`ugoe_model::validate` を通る。
pub fn propose_depth_turn(
    model: &Model,
    estimator: &dyn DepthEstimator,
    request: &DepthRequest,
    labels: &BTreeMap<MeshId, Label>,
    targets: &[TurnTarget],
) -> Result<Proposal, AiError> {
    let map = estimator.estimate(request)?;
    let plans = targets
        .iter()
        .map(|t| plan_turn(model, &map, labels, t))
        .collect::<Result<Vec<_>, _>>()?;
    proposal_from_plans(model, &plans)
}

/// 深度マップから 1 つの対象の振り向きの設定を決める (推定器を呼ばない部分)。
pub fn plan_turn(
    model: &Model,
    map: &DepthMap,
    labels: &BTreeMap<MeshId, Label>,
    target: &TurnTarget,
) -> Result<TurnPlan, AiError> {
    map.check()?;
    let deformer = model
        .deformers
        .iter()
        .find(|d| d.id == target.deformer)
        .ok_or_else(|| invalid(format!("deformer `{}` not found", target.deformer)))?;
    if !matches!(deformer.kind, DeformerKind::Warp(_)) {
        return Err(invalid(format!(
            "deformer `{}` is not a warp deformer",
            target.deformer
        )));
    }

    let scene = Scene::evaluate(model)?;
    let points = scene
        .instance
        .compiled()
        .deformer_index(target.deformer.as_str())
        .and_then(|i| scene.instance.warp_world_points(i))
        .ok_or_else(|| {
            invalid(format!(
                "deformer `{}` has no control points",
                target.deformer
            ))
        })?
        .to_vec();
    let regions = scene.regions(model, &target.deformer, labels);
    if regions.is_empty() {
        return Err(invalid(format!(
            "deformer `{}` has no meshes to estimate the depth of",
            target.deformer
        )));
    }
    let role = match target.role {
        Some(role) => role,
        None => guess_role(&regions).ok_or_else(|| {
            invalid(format!(
                "cannot tell whether `{}` is a head or a body from the labels",
                target.deformer
            ))
        })?,
    };

    // 部位 (顔・体) のメッシュで、大きさ・中心と、深度の正規化の範囲を決める。
    let anchors: Vec<&Region> = {
        let a: Vec<_> = regions
            .iter()
            .filter(|r| r.label.is_some_and(|l| role.anchor_labels().contains(&l)))
            .collect();
        if a.is_empty() {
            regions.iter().collect()
        } else {
            a
        }
    };
    let (min, max) = bounds(anchors.iter().flat_map(|r| r.positions.iter().copied()));
    let relief = role.relief() * (max.x - min.x);
    let sample = |p: Vec2| {
        map.sample(
            p.x / model.meta.canvas.width,
            p.y / model.meta.canvas.height,
        )
    };
    // 部位の外接矩形を格子に分け、部位のメッシュが覆う点で標本化する (頂点だけでは内側の点が少ない)。
    let raw: Vec<f32> = (0..NORMALIZE_GRID)
        .flat_map(|j| (0..NORMALIZE_GRID).map(move |i| (i, j)))
        .map(|(i, j)| {
            let t = (Vec2::new(i as f32, j as f32) + 0.5) / NORMALIZE_GRID as f32;
            min + (max - min) * t
        })
        .filter(|&p| anchors.iter().any(|r| covers(r, p)))
        .map(sample)
        .collect();
    let (lo, hi) = percentiles(raw, PERCENTILE);
    let normalize = |v: f32| {
        if hi - lo <= f32::EPSILON * hi.abs().max(1.0) {
            0.0
        } else {
            ((v - lo) / (hi - lo)).clamp(0.0, 1.0)
        }
    };

    let mut depth = Vec::with_capacity(points.len());
    let mut point_labels = Vec::with_capacity(points.len());
    for &p in &points {
        // 何も覆っていない制御点 (外周の余白) は輪郭と同じ深さにする。
        let (d, label) = match topmost(&regions, p) {
            Some(region) => {
                let rule = region
                    .label
                    .map_or(LabelDepth::Surface { bias: 0.0 }, label_depth);
                (rule.apply(normalize(sample(p))), region.label)
            }
            None => (0.0, None),
        };
        depth.push(d * relief);
        point_labels.push(label);
    }

    let pivot_canvas = match role {
        TurnRole::Head => (min + max) * 0.5,
        TurnRole::Body => Vec2::new((min.x + max.x) * 0.5, max.y),
    };
    let world = ParentWorld::new(model, deformer.parent.as_ref())
        .map_err(|e| invalid(format!("deformer `{}`: {e}", target.deformer)))?;
    let pivot = world
        .from_canvas(&[pivot_canvas], &[pivot_canvas])
        .map_err(|e| invalid(format!("deformer `{}`: {e}", target.deformer)))?[0];
    let (yaw, pitch) = role.parameters();
    Ok(TurnPlan {
        deformer: target.deformer.clone(),
        role,
        depth,
        relief,
        rotation: DepthRotation {
            angle_scale: role.angle_scale(),
            pivot,
            pivot_depth: 0.0,
            yaw: Some(yaw.into()),
            pitch: Some(pitch.into()),
        },
        labels: point_labels,
    })
}

/// 提案するコマンドを、複製したドキュメントに適用しながら集める。
pub(crate) struct Builder {
    pub(crate) doc: Document,
    pub(crate) commands: Vec<Command>,
}

impl Builder {
    pub(crate) fn new(model: &Model) -> Self {
        Self::from_document(&Document::new(model.clone()))
    }

    /// `doc` の複製 (履歴なし) に適用しながら集める。モジュールのプロジェクトなら、本体を変える
    /// コマンドやモジュールとして表せないコマンドは、適用のときに拒否される。
    pub(crate) fn from_document(doc: &Document) -> Self {
        Self {
            doc: doc.detached(),
            commands: Vec::new(),
        }
    }

    /// 複製に適用し、通ればコマンドを提案に加える。適用できない (検証エラーを生じる) コマンドは
    /// [`AiError::InvalidProposal`]。
    pub(crate) fn apply(&mut self, command: Command) -> Result<(), AiError> {
        self.doc
            .apply(command.clone())
            .map_err(|e| AiError::InvalidProposal(e.to_string()))?;
        self.commands.push(command);
        Ok(())
    }

    /// パラメータ `id` を返す。なければ標準の値域で足す。
    pub(crate) fn parameter(&mut self, id: &ParamId) -> Result<Parameter, AiError> {
        if let Some(p) = self.doc.model().parameters.iter().find(|p| &p.id == id) {
            return Ok(p.clone());
        }
        let parameter = standard_parameter(id)?;
        self.apply(Command::AddParameter {
            parameter: parameter.clone(),
        })?;
        Ok(parameter)
    }

    /// 格子の軸にキーがなければ足す。足したら `true`。
    fn add_key(&mut self, deformer: &DeformerId, id: &ParamId, key: f32) -> Result<bool, AiError> {
        let target = KeyformTarget::Deformer(deformer.clone());
        if grid::has_key(grid::axis_keys(self.doc.model(), &target, id), key) {
            return Ok(false);
        }
        self.apply(Command::AddKeyformKey {
            target,
            parameter: id.clone(),
            key,
        })?;
        Ok(true)
    }

    /// 組み終えたモデルが `ugoe_model::validate` を通ることを確かめる。
    pub(crate) fn validate(&self) -> Result<(), AiError> {
        let errors = ugoe_model::validate::validate(self.doc.model());
        if errors.is_empty() {
            Ok(())
        } else {
            Err(AiError::InvalidProposal(format!(
                "the proposal does not validate: {errors:?}"
            )))
        }
    }
}

/// 設定をコマンドの列にし、複製したドキュメントに適用して確かめる。
fn proposal_from_plans(model: &Model, plans: &[TurnPlan]) -> Result<Proposal, AiError> {
    let mut b = Builder::new(model);
    let rationale = plans
        .iter()
        .map(|plan| push_turn(&mut b, plan))
        .collect::<Result<Vec<_>, _>>()?;
    b.validate()?;
    Ok(Proposal {
        title: proposal_title(plans),
        rationale: format!(
            "Samples the depth estimated from the artwork at the warp control points, corrects it with the part labels, and makes the turn keyforms by rotating the depth.\n{}",
            rationale.join("\n")
        ),
        commands: b.commands,
    })
}

/// 1 つの対象の振り向きのコマンドを `b` に加え、提案の理由の 1 行を返す。
pub(crate) fn push_turn(b: &mut Builder, plan: &TurnPlan) -> Result<String, AiError> {
    let target = KeyformTarget::Deformer(plan.deformer.clone());
    let parameters = [&plan.rotation.yaw, &plan.rotation.pitch]
        .into_iter()
        .flatten()
        .map(|id| b.parameter(id))
        .collect::<Result<Vec<_>, _>>()?;
    b.apply(Command::SetDepth {
        target: ChildRef::Deformer(plan.deformer.clone()),
        depth: Some(plan.depth.clone()),
    })?;
    // 中立のキーを先に置き (`docs/editor.md#キーフォームの編集`)、推定の設定を深度からの回転に
    // してから端のキーを足す。加算合成の格子のまま端のキーを足すと、斜めのセルがその場で
    // 加算合成で作られてしまう。
    for p in &parameters {
        b.add_key(&plan.deformer, &p.id, p.default)?;
    }
    b.apply(Command::SetKeyformInference {
        target: target.clone(),
        inference: Some(KeyformInference::DepthRotation(plan.rotation.clone())),
    })?;
    let mut added: BTreeMap<ParamId, Vec<f32>> = BTreeMap::new();
    for p in &parameters {
        for key in [p.min, p.max] {
            if b.add_key(&plan.deformer, &p.id, key)? {
                added.entry(p.id.clone()).or_default().push(key);
            }
        }
    }
    // 足したキーのセルは、見た目を保つために補間した値の `authored` になっている。推定に戻す。
    let cells = new_cells(b.doc.model(), &plan.deformer, &added);
    if !cells.is_empty() {
        b.apply(Command::MarkKeyformDerived {
            target: target.clone(),
            cells,
        })?;
    }
    b.apply(Command::InferKeyforms {
        target,
        method: InferMethod::DepthRotation,
        overwrite_derived: false,
    })?;
    let covered: BTreeSet<&str> = plan.labels.iter().flatten().map(|l| l.as_str()).collect();
    Ok(format!(
        "{} ({}): relief {:.0} px, pivot ({:.0}, {:.0}), angle scale {}, labels under the control points [{}]",
        plan.deformer,
        match plan.role {
            TurnRole::Head => "head",
            TurnRole::Body => "body",
        },
        plan.relief,
        plan.rotation.pivot.x,
        plan.rotation.pivot.y,
        plan.rotation.angle_scale,
        covered.into_iter().collect::<Vec<_>>().join(", "),
    ))
}

/// 標準パラメータ (`ugoe_model::standard::ALL`) を作る。
fn standard_parameter(id: &ParamId) -> Result<Parameter, AiError> {
    let &(_, min, default, max) = standard::ALL
        .iter()
        .find(|(s, ..)| *s == id.as_str())
        .ok_or_else(|| AiError::InvalidProposal(format!("`{id}` is not a standard parameter")))?;
    Ok(Parameter {
        id: id.clone(),
        name: id.to_string(),
        min,
        max,
        default,
        kind: ParameterKind::Normal,
        mirror: None,
        group: None,
        internal: false,
    })
}

/// 足したキー (`added`) のどれかにある `authored` のセル。
fn new_cells(
    model: &Model,
    deformer: &DeformerId,
    added: &BTreeMap<ParamId, Vec<f32>>,
) -> Vec<KeyformCell> {
    let Some(grid) = model
        .deformers
        .iter()
        .find(|d| &d.id == deformer)
        .and_then(|d| match &d.kind {
            DeformerKind::Warp(w) => Some(&w.keyforms),
            _ => None,
        })
    else {
        return Vec::new();
    };
    (0..grid.keyforms.len())
        .filter(|index| grid.origin(*index) == KeyformOrigin::Authored)
        .filter_map(|index| grid::cell_at(&grid.bindings, index))
        .filter(|cell| {
            cell.iter().any(|(parameter, key)| {
                added
                    .get(parameter)
                    .is_some_and(|keys| grid::has_key(keys, *key))
            })
        })
        .collect()
}

/// 子孫のメッシュのラベルから部位を決める。
fn guess_role(regions: &[Region]) -> Option<TurnRole> {
    let has = |role: TurnRole| {
        regions
            .iter()
            .any(|r| r.label.is_some_and(|l| role.anchor_labels().contains(&l)))
    };
    [TurnRole::Head, TurnRole::Body]
        .into_iter()
        .find(|&r| has(r))
}

/// 既定値のパラメータで評価したモデル。
struct Scene {
    instance: ModelInstance,
}

/// 基準の姿のメッシュ (キャンバス座標の三角形)。
struct Region {
    label: Option<Label>,
    positions: Vec<Vec2>,
    indices: Vec<u16>,
    /// 前後の順 (大きいほど手前)。描画順、同じなら配列の後ろが手前。
    order: (f32, usize),
}

impl Scene {
    fn evaluate(model: &Model) -> Result<Self, AiError> {
        let compiled = CompiledModel::compile(model.clone())
            .map_err(|e| invalid(format!("the model cannot be evaluated: {e}")))?;
        let mut instance = ModelInstance::new(compiled);
        instance.update();
        Ok(Self { instance })
    }

    /// `deformer` の子孫のメッシュ (ラベルが `background` のもの・マスク専用のものを除く)。
    fn regions(
        &self,
        model: &Model,
        deformer: &DeformerId,
        labels: &BTreeMap<MeshId, Label>,
    ) -> Vec<Region> {
        let compiled = self.instance.compiled();
        model
            .meshes
            .iter()
            .enumerate()
            .filter(|(_, m)| !m.mask_only && is_under(model, m.parent.as_ref(), deformer))
            .filter_map(|(i, m)| {
                let label = labels.get(&m.id).copied();
                if label.map(label_depth) == Some(LabelDepth::Ignore) {
                    return None;
                }
                let state = &self.instance.drawables()[compiled.mesh_index(m.id.as_str())?];
                Some(Region {
                    label,
                    positions: state.positions.clone(),
                    indices: m.indices.clone(),
                    order: (state.draw_order, i),
                })
            })
            .collect()
    }
}

/// 親 `parent` が `deformer` かその子孫か。
fn is_under<'a>(
    model: &'a Model,
    mut parent: Option<&'a DeformerId>,
    deformer: &DeformerId,
) -> bool {
    // 親の連鎖は検証済みのモデルでは循環しないが、念のため深さを数える。
    for _ in 0..=model.deformers.len() {
        match parent {
            Some(p) if p == deformer => return true,
            Some(p) => {
                parent = model
                    .deformers
                    .iter()
                    .find(|d| &d.id == p)
                    .and_then(|d| d.parent.as_ref());
            }
            None => return false,
        }
    }
    false
}

/// `p` を覆うメッシュのうち最も手前のもの。
fn topmost(regions: &[Region], p: Vec2) -> Option<&Region> {
    regions.iter().filter(|r| covers(r, p)).max_by(|a, b| {
        a.order
            .0
            .total_cmp(&b.order.0)
            .then(a.order.1.cmp(&b.order.1))
    })
}

fn covers(region: &Region, p: Vec2) -> bool {
    region.indices.as_chunks::<3>().0.iter().any(|t| {
        let [a, b, c] = t.map(|i| region.positions.get(usize::from(i)));
        match (a, b, c) {
            (Some(&a), Some(&b), Some(&c)) => in_triangle(p, a, b, c),
            _ => false,
        }
    })
}

/// 点が三角形の内側 (辺の上を含む) にあるか。向きは問わない。面積のない三角形と NaN を含むものは
/// 何も覆わない (符号が付かないので、そのままではどの点も内側と判定してしまう)。
fn in_triangle(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    let cross = |o: Vec2, u: Vec2, v: Vec2| (u.x - o.x) * (v.y - o.y) - (u.y - o.y) * (v.x - o.x);
    let area = cross(a, b, c);
    if area == 0.0 || !area.is_finite() {
        return false;
    }
    let (d1, d2, d3) = (cross(a, b, p), cross(b, c, p), cross(c, a, p));
    let inside_ccw = d1 >= 0.0 && d2 >= 0.0 && d3 >= 0.0;
    let inside_cw = d1 <= 0.0 && d2 <= 0.0 && d3 <= 0.0;
    inside_ccw || inside_cw
}

fn bounds(points: impl Iterator<Item = Vec2>) -> (Vec2, Vec2) {
    points.fold(
        (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY)),
        |(lo, hi), p| (lo.min(p), hi.max(p)),
    )
}

/// 値の列の `q` と `1 - q` の分位点 (近い順位の値)。
pub(crate) fn percentiles(mut values: Vec<f32>, q: f32) -> (f32, f32) {
    if values.is_empty() {
        return (0.0, 0.0);
    }
    values.sort_by(f32::total_cmp);
    let last = values.len() - 1;
    let at = |t: f32| values[((last as f32 * t).round() as usize).min(last)];
    (at(q), at(1.0 - q))
}

/// 提案の題。振り向かせる部位を書く。
fn proposal_title(plans: &[TurnPlan]) -> String {
    let has = |role| plans.iter().any(|p| p.role == role);
    let parts = match (has(TurnRole::Head), has(TurnRole::Body)) {
        (true, true) => "Head and body",
        (false, true) => "Body",
        _ => "Head",
    };
    format!("{parts} turn from estimated depth")
}

fn invalid(message: String) -> AiError {
    AiError::InvalidRequest(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_cells_name_the_cells_on_the_added_keys() {
        let model = ugoe_model::fixtures::warp_grid();
        let added = BTreeMap::from([(ParamId::new("X"), vec![1.0])]);
        let cells = new_cells(&model, &DeformerId::new("Warp"), &added);
        assert_eq!(cells, vec![KeyformCell::from([(ParamId::new("X"), 1.0)])]);
    }

    #[test]
    fn new_cells_skip_an_axis_without_keys() {
        // 検証を通らない格子 (キーのない軸) でも添字の外を読まない。
        let mut model = ugoe_model::fixtures::warp_grid();
        let Some(DeformerKind::Warp(warp)) = model
            .deformers
            .iter_mut()
            .find(|d| d.id.as_str() == "Warp")
            .map(|d| &mut d.kind)
        else {
            panic!("no warp");
        };
        warp.keyforms.bindings.push(ugoe_model::KeyformBinding {
            parameter: ParamId::new("Y"),
            keys: Vec::new(),
        });
        let added = BTreeMap::from([(ParamId::new("X"), vec![1.0])]);
        assert!(new_cells(&model, &DeformerId::new("Warp"), &added).is_empty());
    }

    #[test]
    fn labels_order_the_parts_front_to_back() {
        // 前髪は額 (推定 0.5) より手前に、後ろ髪は輪郭より奥に、横髪は頬より前に出ない。
        assert_eq!(label_depth(Label::HairFront).apply(0.5), 0.8);
        assert_eq!(label_depth(Label::HairFront).apply(0.9), 0.9);
        assert_eq!(label_depth(Label::HairBack).apply(0.7), -0.3);
        assert_eq!(label_depth(Label::HairSide).apply(0.6), 0.3);
        assert!((label_depth(Label::Nose).apply(0.9) - 1.0).abs() < 1e-6);
        assert_eq!(label_depth(Label::Face).apply(0.4), 0.4);
        assert_eq!(label_depth(Label::Background), LabelDepth::Ignore);
    }

    #[test]
    fn percentiles_ignore_outliers() {
        let mut values: Vec<f32> = (0..=100).map(|i| i as f32).collect();
        values.push(1e6);
        let (lo, hi) = percentiles(values, 0.02);
        assert_eq!(lo, 2.0);
        assert_eq!(hi, 99.0);
        assert_eq!(percentiles(Vec::new(), 0.02), (0.0, 0.0));
    }

    #[test]
    fn triangles_cover_their_edges() {
        let (a, b, c) = (
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(0.0, 10.0),
        );
        assert!(in_triangle(Vec2::new(1.0, 1.0), a, b, c));
        assert!(in_triangle(Vec2::new(5.0, 0.0), a, b, c));
        assert!(!in_triangle(Vec2::new(6.0, 6.0), a, b, c));
        // 向きによらない。
        assert!(in_triangle(Vec2::new(1.0, 1.0), a, c, b));
    }

    #[test]
    fn degenerate_triangles_cover_nothing() {
        // 1 点につぶれた三角形や NaN の頂点は、どの点も覆わない (覆うと最も手前の部位を取り違える)
        let a = Vec2::new(5.0, 5.0);
        assert!(!in_triangle(Vec2::new(100.0, -40.0), a, a, a));
        assert!(!in_triangle(a, a, a, a));
        let nan = Vec2::new(f32::NAN, 0.0);
        assert!(!in_triangle(
            Vec2::new(1.0, 1.0),
            nan,
            a,
            Vec2::new(0.0, 9.0)
        ));
        assert!(!in_triangle(
            nan,
            Vec2::ZERO,
            Vec2::new(10.0, 0.0),
            Vec2::new(0.0, 10.0)
        ));
    }

    #[test]
    fn the_title_names_the_turned_parts() {
        let plan = |role| TurnPlan {
            deformer: "W".into(),
            role,
            depth: vec![],
            relief: 0.0,
            rotation: DepthRotation {
                angle_scale: 1.0,
                pivot: Vec2::ZERO,
                pivot_depth: 0.0,
                yaw: None,
                pitch: None,
            },
            labels: vec![],
        };
        assert_eq!(
            proposal_title(&[plan(TurnRole::Head)]),
            "Head turn from estimated depth"
        );
        assert_eq!(
            proposal_title(&[plan(TurnRole::Body)]),
            "Body turn from estimated depth"
        );
        assert_eq!(
            proposal_title(&[plan(TurnRole::Body), plan(TurnRole::Head)]),
            "Head and body turn from estimated depth"
        );
    }
}
