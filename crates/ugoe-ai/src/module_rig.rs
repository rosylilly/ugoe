//! 着せ替えのモジュールのプロジェクト向けの自動リギングの提案
//! (`docs/ai-integration.md#モジュールの自動リギング`、ADR 0024)。
//!
//! モジュールのプロジェクトの作業用のドキュメント (本体とモジュールを合成したもの。`Document::module`) と、
//! 衣装のメッシュの意味ラベル ([`module_mesh_parts`]) から、**パラメータを足さず、本体の要素を変えない**
//! リグを [`Command`] の列として提案する。キーフォームの軸と物理の入力には、本体にある標準パラメータと
//! スロットが公開するパラメータだけを使う。
//!
//! - **追従 (取り付け先に合わせた変形):** 衣装のメッシュはスロットの取り付け先 (`Slot::anchor`) の子なので、
//!   取り付け先の動きはそのまま受け継ぐ。取り付け先の下にある本体のワープ (顔の向きのワープなど) の動きは
//!   受け継がないので、衣装を覆う本体のワープのうち、衣装と重なる本体の絵で描画順が衣装に最も近いもの
//!   (衣装が重なる層を動かすワープ) を選び、その変形を写したワープ (追従のワープ) で衣装を包む。
//!   追従のワープのキーフォーム (見た目の差分) は、本体のワープの各キーでの変形から推定する。
//!   組み方 [`FollowMode::Mesh`] では、ワープで近似せずに衣装のメッシュのキーフォームへ直接写す。
//! - **本体のメッシュへの追従:** 目の開閉のように本体のメッシュ自身のキーフォームで形が変わる部分に重なる衣装
//!   (化粧など) は、その変位を衣装の頂点へ写す (`mesh.rs`)。
//! - **揺れ:** 揺れる意味ラベル (髪・尻尾・耳飾り・首の飾り・下の服) の衣装は、揺れ用のワープで包み、
//!   追従のワープに取り付けた振り子の動的物理で揺らす (`crate::rig` の髪揺れと同じ方法)。スロットが公開する
//!   本体の衝突体 (`Slot::colliders`) があれば、振り子を当てる (体にめり込まない)。
//! - **ポーズ:** 衣装の差分 (名前の末尾の A / B だけが違うもの、絵が重なり意味ラベルが同じで PSD で片方を隠して
//!   あるもの) をポーズにする (`pose.rs`)。
//! - **AI が決める値** ([`ModuleRigParams`]): 揺れの大きさと振り子の長さ。問い直しと利用者の注文は
//!   `crate::rig` と同じ ([`propose_module_rig_with_ai_guided`])。
//!
//! 提案は作業用のドキュメントの複製に適用し、モジュールとして表せること (`Document::split_module`) と
//! `ugoe_model::validate` を確かめてから返す。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use serde::{Deserialize, Serialize};
use serde_json::json;
use ugoe_core::Vec2;
use ugoe_editor_core::ids::unique_id;
use ugoe_editor_core::physics_check::rest_pose_collisions;
use ugoe_editor_core::space::{ParameterValues, canvas_to_local};
use ugoe_editor_core::{Command, Document, KeyformTarget, ModuleContext};
use ugoe_model::deformer::WarpKeyform;
use ugoe_model::mesh::MeshKeyform;
use ugoe_model::physics::{ColliderId, PhysicsRig, VertexOwner};
use ugoe_model::{
    Deformer, DeformerId, DeformerKind, KeyformGrid, KeyformInterpolation, MeshId, Model, ParamId,
    standard,
};
use ugoe_rigging::ParentWorld;

use crate::classify::{Label, LabelCategory, LayerLabel, Side};
use crate::instructions::Instructions;
use crate::rig::hair::{self, Sway};
use crate::rig::{
    Answered, MeshPart, Rect, RejectedAnswer, Rig, RigTemplate, ask_until_valid, mesh_bounds,
    mesh_name, mesh_parts_where, push_rejected, range_errors, revision_rules, template,
    values_schema,
};
use crate::turn::Builder;
use crate::{AiError, Proposal, Provider, ProviderRequest};

mod mesh;
mod pose;

use mesh::{Frame, MeshFollow, MeshFollows, Poser};

/// 追従のワープの分割数 (一辺のセルの数)。本体の顔のワープ (6×6 の曲線補間) の丸みを写せるよう細かめにする
/// (見本の胸から上の人物で、4 では振り向きの移動量の 3 割、8 では 1 割ほどずれる)。
const FOLLOW_CELLS: u16 = 8;
/// 衣装が本体のワープに従うのに要る、衣装の外接矩形のうちワープの覆う割合 (ワープは外側へも外挿するので小さめ。
/// 顔の横に垂れる耳飾りも顔のワープに従えるように)。
const MIN_OVERLAP: f32 = 0.1;
/// 追従のワープのキーの組の数の上限 (軸のキーの数の積)。超える本体のワープには従わない。
const MAX_FOLLOW_CELLS: usize = 81;
/// 衣装のメッシュに直接写すときのキーの組の数の上限 (本体のワープと本体のメッシュの軸を合わせたもの)。
const MAX_MESH_CELLS: usize = 243;

/// レイヤーの分類の結果を、モジュールのメッシュ (本体のメッシュを除く) に名前で対応付ける
/// ([`crate::rig::mesh_parts`] と同じ規則。同じ名前かどうかもモジュールのメッシュの中で数える)。
/// `doc` がモジュールのプロジェクトでなければ [`crate::rig::mesh_parts`] と同じ。
pub fn module_mesh_parts(doc: &Document, layers: &[LayerLabel]) -> BTreeMap<MeshId, MeshPart> {
    match doc.module() {
        Some(context) => mesh_parts_where(doc.model(), layers, |m| !context.is_base_mesh(&m.id)),
        None => crate::rig::mesh_parts(doc.model(), layers),
    }
}

/// テンプレートでは決められない、モジュールのリグの数値。AI が衣装の絵から決めるか、既定値を使う。
///
/// 範囲は [`ModuleRigParams::RANGES`]。範囲の外の値は丸めずにエラーにする ([`ModuleRigParams::check`])。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ModuleRigParams {
    /// 揺れの大きさ: 揺れ用のワープの制御点の重み (ラベルごとの最大) に掛ける係数。0 で揺らさない。
    pub sway_scale: f32,
    /// 振り子の長さの係数 (揺れる衣装の高さに対する、ラベルごとの長さに掛ける)。長いほどゆっくり揺れる。
    pub sway_length: f32,
}

impl Default for ModuleRigParams {
    fn default() -> Self {
        Self {
            sway_scale: 1.0,
            sway_length: 1.0,
        }
    }
}

impl ModuleRigParams {
    /// 値の名前と、許す範囲 (両端を含む)。
    pub const RANGES: [(&'static str, f32, f32); 2] =
        [("sway_scale", 0.0, 2.0), ("sway_length", 0.5, 1.5)];

    fn values(&self) -> [f32; 2] {
        [self.sway_scale, self.sway_length]
    }

    /// 範囲の外の値のエラー (英語。AI に送り返す)。なければ空。
    pub fn check(&self) -> Vec<String> {
        range_errors(&Self::RANGES, &self.values())
    }
}

/// 本体のワープへの追従の形 (利用者が選ぶ)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FollowMode {
    /// 衣装を追従のワープ (`Follow<ID>`、8×8) で包み、ワープのキーフォームに本体のワープの変形を写す。
    /// 衣装を描き足しても組み直さずに済むが、本体のワープを格子で近似するので少しずれる
    /// (見本の胸から上の人物で、振り向きの移動量の 1 割ほど)。
    #[default]
    Warp,
    /// 衣装のメッシュのキーフォームに、本体のワープの変形を直接写す。キーの上では頂点が本体のワープの中の点と
    /// 同じ所へ動く (ずれない) が、メッシュの頂点を編集したら組み直す。
    Mesh,
}

/// モジュールのリグの組み方 (AI は決めない。利用者が選ぶ)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModuleRigOptions {
    /// 本体のワープへの追従の形。
    pub follow: FollowMode,
    /// 揺れのリグを、スロットが公開する本体の衝突体 (`Slot::colliders`) に当てる。
    pub colliders: bool,
    /// 衣装の差分 (名前の末尾が A / B … で違うだけのもの、絵が重なり意味ラベルが同じで PSD で片方を隠してあるもの) を、
    /// ポーズにする。
    pub poses: bool,
}

impl Default for ModuleRigOptions {
    fn default() -> Self {
        Self {
            follow: FollowMode::default(),
            colliders: true,
            poses: true,
        }
    }
}

/// 揺らす意味ラベルの振り子の値と、入力に使う標準パラメータ (頭の動きか体の動きか)。
/// 揺らさないラベル (帽子・上の服・眼鏡など) は `None` (追従だけ)。
fn sway_of(label: Label) -> Option<(Sway, SwayInput)> {
    let sway = match label {
        Label::HairFront | Label::HairSide | Label::HairBack => {
            return Some((hair::of(label), SwayInput::Head));
        }
        // 尻尾: 後ろ髪と同じく 2 区間で大きく揺らす。
        Label::Tail => return Some((hair::of(Label::HairBack), SwayInput::Body)),
        // 耳飾り: 上の留め具の下からよく揺れる。
        Label::Earwear => Sway {
            start: 0.15,
            max: 0.8,
            length: 1.0,
            sections: 1,
            mobility: 0.92,
            delay: 0.7,
            acceleration: 1.5,
            shift: 0.06,
        },
        // 首の飾り (リボン・ネクタイ): 結び目の下を小さく揺らす。
        Label::Neckwear => Sway {
            start: 0.3,
            max: 0.5,
            length: 1.0,
            sections: 1,
            mobility: 0.9,
            delay: 0.7,
            acceleration: 1.5,
            shift: 0.05,
        },
        // 下の服 (スカート): 裾だけを小さく揺らす。
        Label::Bottomwear => Sway {
            start: 0.3,
            max: 0.4,
            length: 1.0,
            sections: 1,
            mobility: 0.9,
            delay: 0.6,
            acceleration: 1.5,
            shift: 0.05,
        },
        _ => return None,
    };
    let input = match label {
        Label::Earwear => SwayInput::Head,
        _ => SwayInput::Body,
    };
    Some((sway, input))
}

/// 振り子の根元を横に動かす入力。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SwayInput {
    /// `AngleX` (頭の左右)。
    Head,
    /// `BodyAngleX` (体の左右)。本体に無ければ `AngleX`。
    Body,
}

/// 衣装が従う本体のワープと、追従のワープに付けるキーの軸。
#[derive(Debug, Clone)]
struct Followed {
    deformer: DeformerId,
    name: String,
    /// 本体のワープとその祖先 (取り付け先の手前まで) の軸のうち、使えるパラメータのもの (キーは既定値を含む)。
    axes: Vec<(ParamId, Vec<f32>)>,
    /// 使えないパラメータの軸 (スロットが公開していない。追従では無視する)。
    ignored: Vec<ParamId>,
    /// 曲線補間の軸を持つ (追従のワープも曲線補間にする)。
    smooth: bool,
    /// 基準の姿でワープが覆う矩形 (キャンバス座標)。
    region: Rect,
    /// 取り付け先からの深さ (深いほど細かい動きに従う)。
    depth: usize,
}

/// 揺らす衣装 (同じラベルと左右のメッシュ)。
#[derive(Debug, Clone)]
struct SwayGroup {
    label: Label,
    side: Side,
    meshes: Vec<MeshId>,
    sway: Sway,
    /// 入力のパラメータ。
    input: ParamId,
}

/// 同じ本体のワープに従う衣装 (追従のワープ 1 つにまとめる)。
#[derive(Debug, Clone)]
struct Group {
    followed: Option<Followed>,
    meshes: Vec<MeshId>,
    sways: Vec<SwayGroup>,
}

/// モジュールのリグの対象と、基準の姿の外接矩形。
#[derive(Debug, Clone)]
struct ModuleLayout {
    anchor: Option<DeformerId>,
    groups: Vec<Group>,
    /// メッシュの意味ラベル (プロンプトに書く)。
    labels: BTreeMap<MeshId, MeshPart>,
    bounds: BTreeMap<MeshId, Rect>,
    /// 本体のメッシュのキーフォームに従う衣装のメッシュ (目の開閉など)。
    mesh_follows: MeshFollows,
    /// 揺れのリグが当たってよい本体の衝突体 (スロットが公開し、本体の物理にあるもの)。
    colliders: Vec<ColliderId>,
    /// 衣装の差分 (ポーズにする候補)。
    variants: Vec<pose::Variants>,
    /// 組めなかったもの (提案の理由に書く)。
    skipped: Vec<String>,
}

impl ModuleLayout {
    fn new(doc: &Document, parts: &BTreeMap<MeshId, MeshPart>) -> Result<Self, AiError> {
        let context = doc.module().ok_or_else(|| {
            AiError::InvalidRequest("the document is not a module project".into())
        })?;
        let model = doc.model();
        let bounds = mesh_bounds(model)?;
        let anchor = context.slot().anchor.clone();
        let mut skipped = Vec::new();
        let top = costume_meshes(model, context, anchor.as_ref(), &bounds, &mut skipped)?;
        let usable = usable_parameters(model, context);
        let costume: Vec<Rect> = top.iter().map(|id| bounds[id]).collect();
        let candidates = candidates(
            model,
            context,
            anchor.as_ref(),
            &usable,
            &costume,
            &mut skipped,
        )?;
        let owned = owned_base_meshes(model, context, &bounds, &candidates);
        let mut groups: Vec<Group> = Vec::new();
        for id in &top {
            let rect = bounds[id];
            let order = model
                .meshes
                .iter()
                .find(|m| &m.id == id)
                .and_then(draw_order)
                .unwrap_or(0.0);
            let followed = followed_warp(&candidates, &owned, rect, order);
            let group = group_of(&mut groups, followed)?;
            group.meshes.push(id.clone());
            if let Some(part) = parts.get(id) {
                add_sway(group, model, &usable, id, part, &mut skipped);
            }
        }
        let frames = mesh_frames(&groups, anchor.as_ref());
        let mesh_follows = mesh_follows(model, context, &usable, &frames, anchor.as_ref(), parts)?;
        let colliders = exposed_colliders(doc, context);
        let variants = pose::variants(model, context, parts, &bounds);
        Ok(Self {
            anchor,
            groups,
            labels: parts.clone(),
            bounds,
            mesh_follows,
            colliders,
            variants,
            skipped,
        })
    }

    fn union(&self, ids: &[MeshId]) -> Option<Rect> {
        ids.iter()
            .filter_map(|id| self.bounds.get(id).copied())
            .reduce(Rect::union)
    }
}

/// 衣装のメッシュのうち、取り付け先の直下のもの (自分で組んだデフォーマの中のものは変えない。`skipped` に書く)。
/// なければ [`AiError::Template`]。
fn costume_meshes(
    model: &Model,
    context: &ModuleContext,
    anchor: Option<&DeformerId>,
    bounds: &BTreeMap<MeshId, Rect>,
    skipped: &mut Vec<String>,
) -> Result<Vec<MeshId>, AiError> {
    let mut top = Vec::new();
    for mesh in &model.meshes {
        if context.is_base_mesh(&mesh.id) || mesh.mask_only || !bounds.contains_key(&mesh.id) {
            continue;
        }
        if mesh.parent.as_ref() == anchor {
            top.push(mesh.id.clone());
        } else {
            skipped.push(format!(
                "`{}` is inside a deformer of the module, so it is left as is",
                mesh_name(model, &mesh.id)
            ));
        }
    }
    if top.is_empty() {
        return Err(template(
            RigTemplate::Module,
            "the module has no meshes directly under the slot's anchor; import the costume PSD first"
                .into(),
        ));
    }
    Ok(top)
}

/// 本体のメッシュを、いちばん近い候補の祖先のワープに振り分ける (そのワープで動く本体の絵の外接矩形と描画順)。
fn owned_base_meshes(
    model: &Model,
    context: &ModuleContext,
    bounds: &BTreeMap<MeshId, Rect>,
    candidates: &[Followed],
) -> BTreeMap<DeformerId, Vec<(Rect, f32)>> {
    let mut owned: BTreeMap<DeformerId, Vec<(Rect, f32)>> = BTreeMap::new();
    for mesh in &model.meshes {
        if !context.is_base_mesh(&mesh.id) || mesh.mask_only {
            continue;
        }
        let (Some(rect), Some(order)) = (bounds.get(&mesh.id), draw_order(mesh)) else {
            continue;
        };
        let mut parent = mesh.parent.as_ref();
        while let Some(id) = parent {
            if candidates.iter().any(|c| &c.deformer == id) {
                owned.entry(id.clone()).or_default().push((*rect, order));
                break;
            }
            parent = find(model, id).and_then(|d| d.parent.as_ref());
        }
    }
    owned
}

/// 外接矩形 `rect`・描画順 `order` の衣装が従う本体のワープ。衣装と重なる本体の絵のうち、描画順が衣装に
/// 最も近いもの (衣装が重なる層) を動かすワープ。同じなら深いもの、重なりの大きいもの。
fn followed_warp(
    candidates: &[Followed],
    owned: &BTreeMap<DeformerId, Vec<(Rect, f32)>>,
    rect: Rect,
    order: f32,
) -> Option<Followed> {
    let gap = |f: &Followed| {
        owned
            .get(&f.deformer)
            .into_iter()
            .flatten()
            .filter(|(r, _)| overlap(rect, *r) > 0.0)
            .map(|(_, o)| (o - order).abs())
            .fold(f32::INFINITY, f32::min)
    };
    candidates
        .iter()
        .filter(|f| overlap(rect, f.region) >= MIN_OVERLAP)
        .min_by(|a, b| {
            gap(a)
                .total_cmp(&gap(b))
                .then(b.depth.cmp(&a.depth))
                .then(overlap(rect, b.region).total_cmp(&overlap(rect, a.region)))
        })
        .cloned()
}

/// 本体のワープ `followed` に従う衣装のまとまり (なければ末尾に足す)。
fn group_of(groups: &mut Vec<Group>, followed: Option<Followed>) -> Result<&mut Group, AiError> {
    let key = followed.as_ref().map(|f| f.deformer.clone());
    let index = match groups
        .iter()
        .position(|g| g.followed.as_ref().map(|f| &f.deformer) == key.as_ref())
    {
        Some(index) => index,
        None => {
            groups.push(Group {
                followed,
                meshes: Vec::new(),
                sways: Vec::new(),
            });
            groups.len() - 1
        }
    };
    groups
        .get_mut(index)
        .ok_or_else(|| template(RigTemplate::Module, "no group".into()))
}

/// 揺れるラベルの衣装のメッシュ `id` を、まとまり `group` の揺れ (ラベルと左右ごと) に加える。
/// 入力のパラメータが本体になければ `skipped` に書く。
fn add_sway(
    group: &mut Group,
    model: &Model,
    usable: &BTreeSet<ParamId>,
    id: &MeshId,
    part: &MeshPart,
    skipped: &mut Vec<String>,
) {
    let Some((sway, input)) = sway_of(part.label) else {
        return;
    };
    let has = |p: &str| usable.contains(&ParamId::new(p));
    let input = match input {
        SwayInput::Body if has(standard::BODY_ANGLE_X) => standard::BODY_ANGLE_X,
        _ if has(standard::ANGLE_X) => standard::ANGLE_X,
        _ => {
            skipped.push(format!(
                "{} (`{}`) does not sway because the base has no `AngleX`",
                part.label,
                mesh_name(model, id)
            ));
            return;
        }
    };
    match group
        .sways
        .iter_mut()
        .find(|s| s.label == part.label && s.side == part.side)
    {
        Some(s) => s.meshes.push(id.clone()),
        None => group.sways.push(SwayGroup {
            label: part.label,
            side: part.side,
            meshes: vec![id.clone()],
            sway,
            input: ParamId::new(input),
        }),
    }
}

/// 本体のメッシュの変位を測る座標系: 衣装が従う本体のワープ (なければ取り付け先)。
fn mesh_frames(
    groups: &[Group],
    anchor: Option<&DeformerId>,
) -> BTreeMap<MeshId, Option<DeformerId>> {
    groups
        .iter()
        .flat_map(|g| {
            let frame = g
                .followed
                .as_ref()
                .map(|f| f.deformer.clone())
                .or_else(|| anchor.cloned());
            g.meshes.iter().map(move |m| (m.clone(), frame.clone()))
        })
        .collect()
}

/// 揺れのリグが当たってよい本体の衝突体 (スロットが公開し、本体の物理にあるもの)。
fn exposed_colliders(doc: &Document, context: &ModuleContext) -> Vec<ColliderId> {
    doc.physics()
        .map(|physics| {
            context
                .slot()
                .colliders
                .iter()
                .filter(|id| physics.colliders.iter().any(|c| &c.id == *id))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// 本体のメッシュのキーフォームに従う衣装のメッシュ ([`mesh::find`])。形の決まった飾り (帽子・眼鏡・耳飾り)・髪・
/// 服・体のラベルの衣装は従わない (目を閉じても眼鏡は潰れない)。ラベルのないものと、顔・目・口のラベルのもの
/// (化粧・まつ毛など) が従う。
fn mesh_follows(
    model: &Model,
    context: &ModuleContext,
    usable: &BTreeSet<ParamId>,
    frames: &BTreeMap<MeshId, Option<DeformerId>>,
    anchor: Option<&DeformerId>,
    parts: &BTreeMap<MeshId, MeshPart>,
) -> Result<MeshFollows, AiError> {
    let mut poser = Poser::new(model)?;
    let mut worlds: BTreeMap<Option<DeformerId>, Frame> = BTreeMap::new();
    let mut out = MeshFollows::new();
    for (id, frame) in frames {
        let follows = parts.get(id).is_none_or(|p| {
            matches!(
                p.label.category(),
                LabelCategory::Face
                    | LabelCategory::Eye
                    | LabelCategory::Mouth
                    | LabelCategory::Other
            )
        });
        if !follows {
            continue;
        }
        let Some(mesh) = model.meshes.iter().find(|m| &m.id == id) else {
            continue;
        };
        let Some(points) = poser.mesh(&[], id) else {
            continue;
        };
        let order = draw_order(mesh).unwrap_or(0.0);
        let world = match worlds.entry(frame.clone()) {
            std::collections::btree_map::Entry::Occupied(e) => e.into_mut(),
            std::collections::btree_map::Entry::Vacant(e) => {
                e.insert(Frame::new(model, frame.as_ref())?)
            }
        };
        if let Some(found) = mesh::find(
            &mut poser,
            model,
            |m| context.is_base_mesh(m),
            usable,
            world,
            anchor,
            &points,
            order,
        )? {
            out.insert(id.clone(), found);
        }
    }
    Ok(out)
}

/// `rect` のうち `region` が覆う割合 (0〜1)。
fn overlap(rect: Rect, region: Rect) -> f32 {
    let min = rect.min.max(region.min);
    let max = rect.max.min(region.max);
    let inter = (max - min).max(Vec2::ZERO);
    let area = rect.size().x.max(f32::EPSILON) * rect.size().y.max(f32::EPSILON);
    (inter.x * inter.y / area).clamp(0.0, 1.0)
}

/// モジュールが使えるパラメータ: 本体にある標準パラメータと、スロットが公開するもの (`docs/spec/modules.md`)。
fn usable_parameters(model: &Model, context: &ModuleContext) -> BTreeSet<ParamId> {
    model
        .parameters
        .iter()
        .filter(|p| {
            standard::range(p.id.as_str()).is_some() || context.slot().parameters.contains(&p.id)
        })
        .map(|p| p.id.clone())
        .collect()
}

/// メッシュの基準の姿の描画順 (格子の最初のキーフォーム)。
fn draw_order(mesh: &ugoe_model::Mesh) -> Option<f32> {
    mesh.keyforms.keyforms.first().map(|k| k.draw_order)
}

fn find<'a>(model: &'a Model, id: &DeformerId) -> Option<&'a Deformer> {
    model.deformers.iter().find(|d| &d.id == id)
}

fn grid_axes<T>(grid: &KeyformGrid<T>) -> (Vec<(ParamId, Vec<f32>)>, bool) {
    (
        grid.bindings
            .iter()
            .map(|b| (b.parameter.clone(), b.keys.clone()))
            .collect(),
        grid.interpolation == KeyformInterpolation::Smooth,
    )
}

/// 衣装が従えるワープ: 取り付け先の下 (取り付け先がなければどこでも) にある本体のワープで、
/// 使えるパラメータの軸を持つもの。追従のワープには、祖先 (取り付け先の手前まで) の軸も付ける。
fn candidates(
    model: &Model,
    context: &ModuleContext,
    anchor: Option<&DeformerId>,
    usable: &BTreeSet<ParamId>,
    costume: &[Rect],
    skipped: &mut Vec<String>,
) -> Result<Vec<Followed>, AiError> {
    let mut found = Vec::new();
    for deformer in &model.deformers {
        if !context.is_base_deformer(&deformer.id)
            || !matches!(deformer.kind, DeformerKind::Warp(_))
            || Some(&deformer.id) == anchor
        {
            continue;
        }
        // 自分から取り付け先の手前までの鎖。取り付け先の下になければ飛ばす。
        let mut chain = vec![deformer];
        let mut parent = deformer.parent.as_ref();
        let mut under_anchor = anchor.is_none();
        while let Some(id) = parent {
            if Some(id) == anchor {
                under_anchor = true;
                break;
            }
            let Some(d) = find(model, id) else { break };
            chain.push(d);
            parent = d.parent.as_ref();
        }
        if !under_anchor {
            continue;
        }
        // 自分の軸を持たないワープ (揺れ用のワープなど) は、親と同じに動くので候補にしない。
        let own = match &deformer.kind {
            DeformerKind::Warp(w) => &w.keyforms.bindings,
            _ => continue,
        };
        if !own.iter().any(|b| usable.contains(&b.parameter)) {
            continue;
        }
        let mut axes: BTreeMap<ParamId, BTreeSet<u32>> = BTreeMap::new();
        let mut ignored = Vec::new();
        let mut smooth = false;
        for d in &chain {
            let (grid, s) = match &d.kind {
                DeformerKind::Warp(w) => grid_axes(&w.keyforms),
                DeformerKind::Rotation(r) => grid_axes(&r.keyforms),
                DeformerKind::Unknown => continue,
            };
            smooth |= s;
            for (parameter, keys) in grid {
                if !usable.contains(&parameter) {
                    if !ignored.contains(&parameter) {
                        ignored.push(parameter);
                    }
                    continue;
                }
                let default = model
                    .parameters
                    .iter()
                    .find(|p| p.id == parameter)
                    .map_or(0.0, |p| p.default);
                axes.entry(parameter)
                    .or_default()
                    .extend(keys.into_iter().chain([default]).map(f32::to_bits));
            }
        }
        if axes.is_empty() {
            continue;
        }
        let mut axes: Vec<(ParamId, Vec<f32>)> = axes
            .into_iter()
            .map(|(p, keys)| {
                let mut keys: Vec<f32> = keys.into_iter().map(f32::from_bits).collect();
                keys.sort_by(f32::total_cmp);
                (p, keys)
            })
            .collect();
        axes.sort_by(|a, b| a.0.cmp(&b.0));
        let world = ParentWorld::new(model, Some(&deformer.id)).map_err(|e| {
            template(
                RigTemplate::Module,
                format!("deformer `{}`: {e}", deformer.id),
            )
        })?;
        let samples: Vec<Vec2> = (0..=4)
            .flat_map(|j| (0..=4).map(move |i| Vec2::new(i as f32 / 4.0, j as f32 / 4.0)))
            .collect();
        let Some(region) = Rect::of(&world.to_canvas(&samples)) else {
            continue;
        };
        let cells = crate::grid::cell_count(axes.iter().map(|(_, k)| k.len()));
        if cells > MAX_FOLLOW_CELLS {
            // 衣装と重なるものだけ理由に書く。
            if costume.iter().any(|r| overlap(*r, region) >= MIN_OVERLAP) {
                skipped.push(format!(
                    "the base warp `{}` has too many key combinations ({cells}), so no warp follows it",
                    deformer.id
                ));
            }
            continue;
        }
        found.push(Followed {
            deformer: deformer.id.clone(),
            name: deformer.name.clone(),
            axes,
            ignored,
            smooth,
            region,
            depth: chain.len(),
        });
    }
    Ok(found)
}

/// 作業用のモデルで使われておらず、接頭辞 (`<モジュールの名前>/`) を外した ID とも重ならない ID。
fn fresh_id<'a>(
    context: &ModuleContext,
    base: &str,
    existing: impl IntoIterator<Item = &'a str>,
) -> String {
    let existing: Vec<&str> = existing.into_iter().collect();
    let shown: Vec<&str> = existing.iter().map(|id| context.display_id(id)).collect();
    unique_id(base, existing.iter().copied().chain(shown))
}

/// モジュールのリグをテンプレートで提案する (AI を使わない)。
///
/// `doc` はモジュールのプロジェクトの作業用のドキュメント、`parts` は衣装のメッシュの意味ラベル
/// ([`module_mesh_parts`] でレイヤーの分類から作れる。ラベルのないメッシュは追従だけ)、`options` は組み方
/// (追従の形・衝突体・ポーズ)。返す提案は `doc` に `Command::Batch` として適用でき、パラメータを足さず本体を
/// 変えない。組むものがなければ [`AiError::Template`]。
pub fn propose_module_rig(
    doc: &Document,
    parts: &BTreeMap<MeshId, MeshPart>,
    params: &ModuleRigParams,
    options: &ModuleRigOptions,
) -> Result<Proposal, AiError> {
    let layout = ModuleLayout::new(doc, parts)?;
    build(doc, &layout, params, options)
}

fn build(
    doc: &Document,
    layout: &ModuleLayout,
    params: &ModuleRigParams,
    options: &ModuleRigOptions,
) -> Result<Proposal, AiError> {
    let errors = params.check();
    if !errors.is_empty() {
        return Err(AiError::InvalidProposal(errors.join("; ")));
    }
    let context = doc
        .module()
        .ok_or_else(|| AiError::InvalidRequest("the document is not a module project".into()))?;
    let mut rig = Rig {
        b: Builder::from_document(doc),
        template: RigTemplate::Module,
    };
    let anchor_name = layout
        .anchor
        .as_ref()
        .map_or_else(|| "the canvas".to_owned(), |a| format!("`{a}`"));
    let mut rationale = vec![format!(
        "The costume is a child of {anchor_name}, the anchor of slot `{}`, so it inherits that motion as is. No parameters are added",
        context.slot().id
    )];
    for group in &layout.groups {
        rig_group(
            &mut rig,
            context,
            layout,
            group,
            params,
            options,
            &mut rationale,
        )?;
    }
    if options.poses {
        pose::propose(
            &mut rig,
            context,
            &layout.variants,
            &layout.bounds,
            &mut rationale,
        )?;
    }
    if rig.b.commands.is_empty() {
        // 揺らすラベルの部品があるのに AI が揺れを 0 にしたなら、値の誤りとして問い直す。
        if params.sway_scale <= 0.0 && layout.groups.iter().any(|g| !g.sways.is_empty()) {
            return Err(AiError::InvalidProposal(
                "sway_scale is 0, which turns off the sway of every labeled part and leaves nothing to rig; \
                 choose a sway_scale above 0"
                    .into(),
            ));
        }
        return Err(template(
            RigTemplate::Module,
            format!(
                "nothing to rig: the costume already follows the slot's anchor, no base warp or mesh under it moves \
             the costume, no costume part is labeled to sway, and no costume parts are variants{}",
                if layout.skipped.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", layout.skipped.join("; "))
                }
            ),
        ));
    }
    rationale.extend(layout.skipped.iter().map(|s| format!("Not rigged: {s}")));
    rig.b.validate()?;
    if let Some(Err(e)) = rig.b.doc.split_module() {
        return Err(AiError::InvalidProposal(format!(
            "the proposal cannot be kept as a module: {e}"
        )));
    }
    Ok(Proposal {
        title: "Auto rig the module (follow the anchor, sway)".into(),
        rationale: rationale.join("\n"),
        commands: rig.b.commands,
    })
}

fn rig_group(
    rig: &mut Rig,
    context: &ModuleContext,
    layout: &ModuleLayout,
    group: &Group,
    params: &ModuleRigParams,
    options: &ModuleRigOptions,
    rationale: &mut Vec<String>,
) -> Result<(), AiError> {
    let names = group
        .meshes
        .iter()
        .map(|id| format!("`{}`", mesh_name(rig.model(), id)))
        .collect::<Vec<_>>()
        .join(", ");
    let sways = params.sway_scale > 0.0 && !group.sways.is_empty();
    let follows_mesh = group
        .meshes
        .iter()
        .any(|m| layout.mesh_follows.contains_key(m));
    if group.followed.is_none() && !sways && !follows_mesh {
        rationale.push(format!(
            "{}: no base warp under the anchor covers it, so it only follows the anchor",
            names
        ));
        return Ok(());
    }
    // 本体のワープの変形を、ワープで写すか (追従のワープ) メッシュに直接写すか。
    let (warp_follow, direct) = match (&group.followed, options.follow) {
        (Some(f), FollowMode::Warp) => (Some(f), None),
        (Some(f), FollowMode::Mesh) => (None, Some(f)),
        (None, _) => (None, None),
    };
    // 衣装を包むワープ (追従のワープか、揺れの振り子の取り付け先)。
    let frame = if warp_follow.is_some() || sways {
        Some(create_frame(rig, context, layout, group, warp_follow)?)
    } else {
        None
    };
    if let (Some(followed), Some(follow)) = (warp_follow, &frame) {
        follow_keyforms(rig, follow, followed)?;
        rationale.push(format!(
            "Follow: the warp `{follow}` around {} copies the deformation of the base warp `{}` over {}{}",
            names,
            followed.deformer,
            axes_text(&followed.axes),
            ignored_text(followed)
        ));
    } else if let Some(frame) = &frame {
        rationale.push(format!(
            "{}: {}, so makes the warp `{frame}` to attach the sway to",
            names,
            if direct.is_some() {
                "the base warp's deformation is copied to the meshes directly"
            } else {
                "no base warp covers it"
            }
        ));
    }
    if sways {
        let frame = frame
            .as_ref()
            .ok_or_else(|| template(RigTemplate::Module, "no frame for the sway".into()))?;
        for sway in &group.sways {
            rig_sway(
                rig, context, layout, frame, sway, params, options, rationale,
            )?;
        }
    } else if !group.sways.is_empty() {
        rationale.push("Sway: the sway scale is 0, so nothing sways".into());
    }
    follow_meshes(rig, layout, group, direct, &names, rationale)
}

/// まとまり `group` を包むワープを取り付け先の下に作る。本体のワープ `warp_follow` に従うなら追従のワープ
/// (`Follow…`)、なければ揺れの振り子の取り付け先 (`SwayRoot`)。
fn create_frame(
    rig: &mut Rig,
    context: &ModuleContext,
    layout: &ModuleLayout,
    group: &Group,
    warp_follow: Option<&Followed>,
) -> Result<DeformerId, AiError> {
    let (base, name) = match warp_follow {
        Some(f) => (
            format!("Follow{}", f.deformer),
            format!("Follow {}", f.name),
        ),
        None => ("SwayRoot".to_owned(), "Sway root".to_owned()),
    };
    let id = DeformerId::new(fresh_id(
        context,
        &base,
        rig.model().deformers.iter().map(|d| d.id.as_str()),
    ));
    rig.create_warp_as(
        id,
        &name,
        &group.meshes,
        FOLLOW_CELLS,
        FOLLOW_CELLS,
        layout.anchor.as_ref(),
    )
}

/// まとまり `group` のメッシュのキーフォーム: 本体のワープ `direct` の変形 (直接写すとき) と、本体のメッシュの
/// キーフォームへの追従。`names` はまとまりのメッシュの名前 (提案の理由に書く)。
fn follow_meshes(
    rig: &mut Rig,
    layout: &ModuleLayout,
    group: &Group,
    direct: Option<&Followed>,
    names: &str,
    rationale: &mut Vec<String>,
) -> Result<(), AiError> {
    for mesh in &group.meshes {
        let follow = layout.mesh_follows.get(mesh);
        if direct.is_none() && follow.is_none() {
            continue;
        }
        mesh_keyforms(rig, mesh, direct, follow)?;
        if let Some(f) = follow {
            rationale.push(format!(
                "Follow: the vertices of `{}` copy the shape changes of the base mesh `{}` over {} (motion of the base mesh's own keyforms, such as the eyes opening and closing)",
                mesh_name(rig.model(), mesh),
                f.base,
                axes_text(&f.axes)
            ));
        }
    }
    if let Some(followed) = direct {
        rationale.push(format!(
            "Follow: the mesh keyforms of {} copy the deformation of the base warp `{}` over {} directly (on the keys they move where points inside the base warp move){}",
            names,
            followed.deformer,
            axes_text(&followed.axes),
            ignored_text(followed)
        ));
    }
    Ok(())
}

fn axes_text(axes: &[(ParamId, Vec<f32>)]) -> String {
    axes.iter()
        .map(|(p, keys)| format!("`{p}` ({} keys)", keys.len()))
        .collect::<Vec<_>>()
        .join(", ")
}

fn ignored_text(followed: &Followed) -> String {
    if followed.ignored.is_empty() {
        return String::new();
    }
    format!(
        " (does not follow {}, which the slot does not expose)",
        followed
            .ignored
            .iter()
            .map(|p| format!("`{p}`"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// 既定値のパラメータの値 (軸の並びの順)。
fn defaults_of(model: &Model, axes: &[(ParamId, Vec<f32>)]) -> Vec<f32> {
    axes.iter()
        .map(|(p, _)| {
            model
                .parameters
                .iter()
                .find(|x| &x.id == p)
                .map_or(0.0, |x| x.default)
        })
        .collect()
}

/// 追従のワープ `follow` のキーフォームを、本体のワープの各キーでの変形から作る。
///
/// 基準の姿での追従のワープの制御点を、本体のワープの局所座標 (その子の座標系) に写し、キーの組ごとに
/// 本体のワープで写した点を、追従のワープの親 (取り付け先) の座標系に戻す。キーの上では本体のワープの
/// 中の点と同じ位置に来る。
fn follow_keyforms(rig: &mut Rig, follow: &DeformerId, followed: &Followed) -> Result<(), AiError> {
    let target = KeyformTarget::Deformer(follow.clone());
    for (parameter, keys) in &followed.axes {
        rig.add_keys(&target, parameter.as_str(), keys)?;
    }
    if followed.smooth {
        rig.apply(Command::SetKeyformInterpolation {
            target: target.clone(),
            blend_shape: None,
            interpolation: KeyformInterpolation::Smooth,
        })?;
    }
    let (rest_local, ..) = rig.keyform_at(&target, &[])?;
    let rest = rig.canvas_points(&target)?;
    let mut world = ParentWorld::new(rig.model(), Some(&followed.deformer)).map_err(|e| {
        template(
            RigTemplate::Module,
            format!("deformer `{}`: {e}", followed.deformer),
        )
    })?;
    let guesses: Vec<Vec2> = rest.iter().map(|p| followed.region.normalize(*p)).collect();
    let inner = world.from_canvas(&rest, &guesses).map_err(|e| {
        template(
            RigTemplate::Module,
            format!("deformer `{}`: {e}", followed.deformer),
        )
    })?;
    let defaults = defaults_of(rig.model(), &followed.axes);
    for cell in cells(&followed.axes) {
        if cell == defaults {
            continue;
        }
        let values: ParameterValues = followed
            .axes
            .iter()
            .zip(&cell)
            .map(|((p, _), v)| (p.clone(), *v))
            .collect();
        world.set_pose(values.iter().map(|(p, v)| (p.as_str(), *v)));
        let moved = world.to_canvas(&inner);
        let points = canvas_to_local(rig.model(), &target, &values, &moved, &rest_local)
            .map_err(|e| template(RigTemplate::Module, format!("{target}: {e}")))?;
        let named: Vec<(&str, f32)> = followed
            .axes
            .iter()
            .zip(&cell)
            .map(|((p, _), v)| (p.as_str(), *v))
            .collect();
        rig.set_cell(&target, &named, |base| match base {
            ugoe_editor_core::Keyform::Warp(keyform) => {
                Ok(ugoe_editor_core::Keyform::Warp(WarpKeyform {
                    points,
                    ..keyform
                }))
            }
            _ => Err(template(
                RigTemplate::Module,
                format!("{target} is not a warp deformer"),
            )),
        })?;
    }
    Ok(())
}

/// 衣装のメッシュ `mesh` のキーフォームに、本体のワープ `warp` の変形 ([`FollowMode::Mesh`] のとき) と、
/// 本体のメッシュのキーフォームでの形の変化 (`follow`) を合わせて写す。
///
/// 軸は両方の軸を合わせたもの。基準の姿の頂点をフレーム (本体のワープか取り付け先。[`mesh::Frame`]) の
/// 局所座標に写し、キーの組ごとに本体のメッシュの変位 (フレームの局所座標) を足してから、その組の姿の
/// フレームで写す (目の変位は顔の向きと一緒に動き、フレームの動きを二重に数えない)。すでにあるほかの軸の
/// キーフォームは、既定値のセルだけを変える。
fn mesh_keyforms(
    rig: &mut Rig,
    mesh: &MeshId,
    warp: Option<&Followed>,
    follow: Option<&MeshFollow>,
) -> Result<(), AiError> {
    let target = KeyformTarget::Mesh(mesh.clone());
    let mut axes: Vec<(ParamId, Vec<f32>)> = Vec::new();
    for (parameter, keys) in warp
        .map(|w| w.axes.as_slice())
        .into_iter()
        .chain(follow.map(|f| f.axes.as_slice()))
        .flatten()
    {
        match axes.iter_mut().find(|(p, _)| p == parameter) {
            Some((_, existing)) => {
                existing.extend(keys);
                existing.sort_by(f32::total_cmp);
                existing.dedup();
            }
            None => axes.push((parameter.clone(), keys.clone())),
        }
    }
    let count = crate::grid::cell_count(axes.iter().map(|(_, k)| k.len()));
    if axes.is_empty() || count > MAX_MESH_CELLS {
        return Err(template(
            RigTemplate::Module,
            format!(
                "mesh `{mesh}` would need {count} keyforms to follow the base, which is too many"
            ),
        ));
    }
    for (parameter, keys) in &axes {
        rig.add_keys(&target, parameter.as_str(), keys)?;
    }
    if warp.is_some_and(|w| w.smooth) || follow.is_some_and(|f| f.smooth) {
        rig.apply(Command::SetKeyformInterpolation {
            target: target.clone(),
            blend_shape: None,
            interpolation: KeyformInterpolation::Smooth,
        })?;
    }
    let (rest_local, ..) = rig.keyform_at(&target, &[])?;
    let rest = rig.canvas_points(&target)?;
    // 変位を測ったフレームと、直接写す本体のワープは同じ (どちらも衣装が従う本体のワープ)。
    let frame_id = match (warp, follow) {
        (Some(w), _) => Some(w.deformer.clone()),
        (None, Some(f)) => f.frame.clone(),
        (None, None) => None,
    };
    let mut frame = Frame::new(rig.model(), frame_id.as_ref())?;
    let inner = frame.local_of(&rest, None)?;
    let defaults = defaults_of(rig.model(), &axes);
    for cell in cells(&axes) {
        if cell == defaults {
            continue;
        }
        let value_of = |p: &ParamId| {
            axes.iter()
                .zip(&cell)
                .find(|((q, _), _)| q == p)
                .map(|(_, v)| *v)
        };
        let mut moved = inner.clone();
        if let Some(f) = follow {
            let key: Option<Vec<f32>> = f.axes.iter().map(|(p, _)| value_of(p)).collect();
            if let Some(offsets) = key.as_deref().and_then(|k| f.offsets_at(k)) {
                for (p, o) in moved.iter_mut().zip(offsets) {
                    *p += *o;
                }
            }
        }
        let values: ParameterValues = axes
            .iter()
            .zip(&cell)
            .map(|((p, _), v)| (p.clone(), *v))
            .collect();
        let pose: Vec<(ParamId, f32)> = values.iter().map(|(p, v)| (p.clone(), *v)).collect();
        frame.set_pose(&pose);
        let points = frame.canvas_of(&moved);
        let local = canvas_to_local(rig.model(), &target, &values, &points, &rest_local)
            .map_err(|e| template(RigTemplate::Module, format!("{target}: {e}")))?;
        let named: Vec<(&str, f32)> = axes
            .iter()
            .zip(&cell)
            .map(|((p, _), v)| (p.as_str(), *v))
            .collect();
        rig.set_cell(&target, &named, |base| match base {
            ugoe_editor_core::Keyform::Mesh(keyform) => {
                Ok(ugoe_editor_core::Keyform::Mesh(MeshKeyform {
                    positions: local,
                    ..keyform
                }))
            }
            _ => Err(template(
                RigTemplate::Module,
                format!("{target} is not a mesh"),
            )),
        })?;
    }
    Ok(())
}

/// 軸のキーの直積 (軸の並びの順の値の組)。
fn cells(axes: &[(ParamId, Vec<f32>)]) -> Vec<Vec<f32>> {
    axes.iter().fold(vec![Vec::new()], |acc, (_, keys)| {
        acc.iter()
            .flat_map(|prefix| {
                keys.iter().map(move |k| {
                    let mut cell = prefix.clone();
                    cell.push(*k);
                    cell
                })
            })
            .collect()
    })
}

/// 揺れ: 揺れる衣装を包む揺れ用のワープを `frame` (追従のワープか `SwayRoot`) の中に作り、`frame` に取り付けた
/// 振り子の先の横の変位を、制御点に重み付きで足す (`crate::rig` の髪揺れと同じ)。スロットが公開する本体の
/// 衝突体があれば振り子を当てる ([`sway_colliders`])。
#[allow(clippy::too_many_arguments)]
fn rig_sway(
    rig: &mut Rig,
    context: &ModuleContext,
    layout: &ModuleLayout,
    frame: &DeformerId,
    group: &SwayGroup,
    params: &ModuleRigParams,
    options: &ModuleRigOptions,
    rationale: &mut Vec<String>,
) -> Result<(), AiError> {
    let rect = layout.union(&group.meshes).ok_or_else(|| {
        template(
            RigTemplate::Module,
            format!("{} has no bounds", group.label),
        )
    })?;
    let side = match group.side {
        Side::Left => "_l",
        Side::Right => "_r",
        Side::Center => "",
    };
    let base = format!("Sway_{}{side}", group.label.as_str());
    let id = DeformerId::new(fresh_id(
        context,
        &base,
        rig.model().deformers.iter().map(|d| d.id.as_str()),
    ));
    let warp = rig.create_warp_as(
        id,
        &format!("Sway {} {}", group.label, group.side.as_str()),
        &group.meshes,
        3,
        3,
        Some(frame),
    )?;
    let sway = group.sway;
    let max = (sway.max * params.sway_scale).min(1.0);
    let target = KeyformTarget::Deformer(warp.clone());
    let weights = rig
        .canvas_points(&target)?
        .iter()
        .map(|p| {
            let t = ((rect.normalize(*p).y - sway.start) / (1.0 - sway.start)).clamp(0.0, 1.0);
            max * t * t
        })
        .collect();
    let root = Vec2::new(rect.center().x, rect.min.y);
    let length = rect.size().y * sway.length * params.sway_length;
    let lengths = vec![length / sway.sections as f32; sway.sections];
    let shift = sway.shift * rect.size().x;
    let existing: Vec<String> = rig
        .b
        .doc
        .physics()
        .map(|p| p.rigs.iter().map(|r| r.id.to_string()).collect())
        .unwrap_or_default();
    let rig_id = fresh_id(context, &base, existing.iter().map(String::as_str));
    let mut physics = rig.pendulum(
        &rig_id,
        &format!("Sway {} {}", group.label, group.side.as_str()),
        frame,
        &target,
        group.input.as_str(),
        root,
        &lengths,
        sway,
        shift,
        VertexOwner::Warp(warp),
        weights,
    )?;
    let collide = if options.colliders {
        sway_colliders(rig, &mut physics, &layout.colliders, rect)?
    } else {
        None
    };
    rig.apply(Command::AddPhysicsRig { rig: physics })?;
    rationale.push(format!(
        "Sway: {} ({}, {} meshes) sways with a {}-section pendulum ({length:.0} px long) attached to `{frame}` (weight up to {max:.2}; the root moves {shift:.0} px at the ends of `{}`)",
        group.label,
        group.side.as_str(),
        group.meshes.len(),
        sway.sections,
        group.input,
    ));
    if let Some(line) = collide {
        rationale.push(line);
    }
    Ok(())
}

/// 振り子の粒子の衝突の半径 (揺れる衣装の幅に対する割合)。
const COLLISION_RADIUS: f32 = 0.1;

/// 揺れのリグ `physics` を、スロットが公開する本体の衝突体 `colliders` に当てる (体にめり込まないように)。
/// 静止姿勢で当たる衝突体は外し、残りに当てる。粒子には衝突の半径 (衣装の幅の 1 割) を付け、それで静止姿勢で
/// 当たるなら半径は 0 にする
/// (静止姿勢で当たると、最初の更新で衝突体の手前に押し出されて垂れ方が変わる。`docs/spec/physics.md`)。
/// 理由の文を返す。当てる衝突体がなければ `None`。
fn sway_colliders(
    rig: &Rig,
    physics: &mut PhysicsRig,
    colliders: &[ColliderId],
    rect: Rect,
) -> Result<Option<String>, AiError> {
    let Some(settings) = rig.b.doc.physics() else {
        return Ok(None);
    };
    if colliders.is_empty() {
        return Ok(None);
    }
    let hits = |colliders: &[ColliderId], radius: f32| -> Result<Vec<ColliderId>, AiError> {
        let mut candidate = physics.clone();
        candidate.colliders = colliders.to_vec();
        for particle in candidate.particles.iter_mut().skip(1) {
            particle.collision_radius = radius;
        }
        let mut settings = settings.clone();
        settings.rigs.push(candidate);
        let found = rest_pose_collisions(rig.model(), &settings).map_err(|e| {
            template(
                RigTemplate::Module,
                format!("the physics cannot be checked: {e}"),
            )
        })?;
        Ok(found
            .into_iter()
            .filter(|c| c.rig == physics.id)
            .map(|c| c.collider)
            .collect())
    };
    // 半径 0 でも当たる衝突体は外し、残りに半径を付けても当たらなければ半径を付ける。
    let dropped = hits(colliders, 0.0)?;
    let used: Vec<ColliderId> = colliders
        .iter()
        .filter(|c| !dropped.contains(c))
        .cloned()
        .collect();
    let wide = COLLISION_RADIUS * rect.size().x;
    let radius = if !used.is_empty() && hits(&used, wide)?.is_empty() {
        wide
    } else {
        0.0
    };
    let list = |ids: &[ColliderId]| {
        ids.iter()
            .map(|c| format!("`{c}`"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut line = String::new();
    if !used.is_empty() {
        for particle in physics.particles.iter_mut().skip(1) {
            particle.collision_radius = radius;
        }
        physics.colliders.clone_from(&used);
        let _ = write!(
            line,
            "Collision: `{}` collides with the base colliders {} that the slot exposes (particle radius {radius:.0} px, so it does not sink into the body)",
            physics.id,
            list(&used)
        );
    }
    if !dropped.is_empty() {
        if !line.is_empty() {
            line.push('\n');
        }
        let _ = write!(
            line,
            "Collision: `{}` touches the base colliders {} at rest, so it does not collide with them",
            physics.id,
            list(&dropped)
        );
    }
    Ok((!line.is_empty()).then_some(line))
}

// ---- AI が決める値 ----

/// [`propose_module_rig_with_ai`] の結果。
#[derive(Debug, Clone, PartialEq)]
pub struct AiModuleRig {
    pub proposal: Proposal,
    /// 採用した値。
    pub params: ModuleRigParams,
    /// 採用した答えの理由 (反映できなかった注文を除いた本文)。
    pub reason: String,
    /// 2 つの値では反映できなかった注文 (答えの理由の `Not applied:` の部分。AI の言葉のまま)。
    pub unapplied: Vec<String>,
    /// 捨てた答え (古い順)。
    pub rejected: Vec<RejectedAnswer>,
}

/// 作り直しで AI に示す、前の答え。
#[derive(Debug, Clone, PartialEq)]
pub struct PreviousModuleRig {
    pub params: ModuleRigParams,
    /// 前の答えの理由。
    pub reason: String,
}

/// モジュールのリグの値を決めるときに添える、利用者の注文と前の答え (作り直し)。
/// 意味は [`crate::rig::RigGuidance`] と同じ。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ModuleRigGuidance {
    pub instructions: Option<Instructions>,
    pub previous: Option<PreviousModuleRig>,
    /// 同じ注文でレイヤーを分類し直した (ラベルに関わる注文は分類で反映した)。
    pub labels_follow_instructions: bool,
}

/// AI の答えの JSON Schema: [`ModuleRigParams`] の 2 つの数と、理由 (`reason`)。
pub fn module_rig_output_schema() -> serde_json::Value {
    values_schema(&ModuleRigParams::RANGES)
}

#[derive(Deserialize)]
struct Answer {
    sway_scale: f32,
    sway_length: f32,
    reason: String,
}

/// [`module_rig_output_schema`] の形の答えを読む。値と理由を返す。範囲は確かめない。
pub fn module_params_from_answer(
    answer: &serde_json::Value,
) -> Result<(ModuleRigParams, String), AiError> {
    let answer: Answer = serde_json::from_value(answer.clone()).map_err(|e| {
        AiError::InvalidProposal(format!("the answer does not match the schema: {e}"))
    })?;
    Ok((
        ModuleRigParams {
            sway_scale: answer.sway_scale,
            sway_length: answer.sway_length,
        },
        answer.reason,
    ))
}

fn system_prompt() -> String {
    let d = ModuleRigParams::default();
    let [(_, s0, s1), (_, l0, l1)] = ModuleRigParams::RANGES;
    format!(
        "You tune the rig of a costume module for a 2D puppet. The module is attached to a slot of \
         a base character and cannot add parameters; it moves only with the base character's \
         parameters. A fixed template already makes the costume follow the base character's \
         deformation and adds pendulum sway (driven by the head or body turning) to parts labeled \
         as hair, tail, earwear, neckwear or bottomwear. You only choose two numbers, by looking at \
         the costume drawing.\n\n\
         - sway_scale ({s0} to {s1}, default {}): how strongly the swaying parts swing, as a factor \
           on the template's weights. Use larger values for light, soft, loosely hanging things \
           (ribbons, long hair, dangling earrings) and smaller ones for stiff or heavy things. 0 \
           turns the sway off.\n\
         - sway_length ({l0} to {l1}, default {}): the pendulum length as a factor on the height of \
           each swaying part. Longer pendulums swing more slowly.\n\n\
         Stay within the ranges. If your previous answer was rejected, fix the reported errors.",
        d.sway_scale, d.sway_length,
    )
}

/// 注文か前の答えがあるときにシステムプロンプトに足す規則。
fn guidance_rules(guidance: &ModuleRigGuidance) -> String {
    revision_rules(
        guidance.previous.is_some(),
        guidance.instructions.is_some(),
        guidance.labels_follow_instructions,
        "the two values above",
        "new parameters, which a module cannot add, other parts, a different rig structure, \
         motions, or changes to the drawing",
    )
}

fn prompt(
    doc: &Document,
    layout: &ModuleLayout,
    rejected: &[RejectedAnswer],
    guidance: &ModuleRigGuidance,
) -> String {
    let model = doc.model();
    let canvas = Vec2::new(model.meta.canvas.width, model.meta.canvas.height);
    let bbox = |r: Rect| {
        let (min, size) = (r.min / canvas, r.size() / canvas);
        format!("{:.3}, {:.3}, {:.3}, {:.3}", min.x, min.y, size.x, size.y)
    };
    let mut s = String::from(
        "The image is the costume drawing. Bounding boxes are fractions of the image as \
         x, y, width, height.\n\n",
    );
    if let Some(context) = doc.module() {
        let _ = writeln!(
            s,
            "The costume goes into slot `{}` of the base character.",
            context.slot().id
        );
    }
    for group in &layout.groups {
        for id in &group.meshes {
            let Some(rect) = layout.bounds.get(id) else {
                continue;
            };
            let label = layout.labels.get(id).map_or_else(
                || "unlabeled".to_owned(),
                |p| format!("{} ({})", p.label, p.side.as_str()),
            );
            let sways = group.sways.iter().any(|g| g.meshes.contains(id));
            let _ = writeln!(
                s,
                "- {label}: {}; {}{}",
                bbox(*rect),
                match &group.followed {
                    Some(f) => format!("follows the base warp `{}`", f.deformer),
                    None => "follows the slot's anchor only".to_owned(),
                },
                if sways { "; sways" } else { "" }
            );
        }
    }
    if let Some(previous) = &guidance.previous {
        let p = previous.params;
        let _ = writeln!(
            s,
            "\nYour previous answer: {}",
            json!({
                "sway_scale": p.sway_scale,
                "sway_length": p.sway_length,
                "reason": previous.reason,
            })
        );
    }
    if let Some(instructions) = &guidance.instructions {
        s.push_str("\nThe user's instructions:\n");
        s.push_str(&instructions.block());
    }
    push_rejected(&mut s, rejected);
    s.push_str("\nChoose the values.");
    s
}

fn request(
    doc: &Document,
    layout: &ModuleLayout,
    image: &[u8],
    rejected: &[RejectedAnswer],
    guidance: &ModuleRigGuidance,
) -> ProviderRequest {
    ProviderRequest {
        system: system_prompt() + &guidance_rules(guidance),
        prompt: prompt(doc, layout, rejected, guidance),
        images: vec![image.to_vec()],
    }
}

/// [`propose_module_rig_with_ai_guided`] が送るリクエスト (`rejected` は前の答えと理由)。エディタは送る前に、
/// これを [`Provider::disclosure`] に渡して利用者に確認を取る。
pub fn module_rig_request_guided(
    doc: &Document,
    parts: &BTreeMap<MeshId, MeshPart>,
    image: &[u8],
    rejected: &[RejectedAnswer],
    guidance: &ModuleRigGuidance,
) -> Result<ProviderRequest, AiError> {
    let layout = ModuleLayout::new(doc, parts)?;
    Ok(request(doc, &layout, image, rejected, guidance))
}

/// テンプレートでモジュールのリグを組み、テンプレートで決められない値 ([`ModuleRigParams`]) を `provider` に
/// 決めさせる。`image` は衣装の原画の PNG。問い直しの規則は [`crate::rig::propose_rig_with_ai`] と同じ。
pub fn propose_module_rig_with_ai(
    provider: &dyn Provider,
    doc: &Document,
    parts: &BTreeMap<MeshId, MeshPart>,
    image: &[u8],
    max_attempts: usize,
) -> Result<AiModuleRig, AiError> {
    propose_module_rig_with_ai_guided(
        provider,
        doc,
        parts,
        image,
        &ModuleRigOptions::default(),
        max_attempts,
        &ModuleRigGuidance::default(),
    )
}

/// [`propose_module_rig_with_ai`] の、利用者の注文と前の答え (作り直し) を添える版
/// (`docs/ai-integration.md#利用者の注文`)。2 つの値で反映できない注文は [`AiModuleRig::unapplied`] に返す。
/// `options` は利用者が選ぶ組み方 (AI には送らない)。
pub fn propose_module_rig_with_ai_guided(
    provider: &dyn Provider,
    doc: &Document,
    parts: &BTreeMap<MeshId, MeshPart>,
    image: &[u8],
    options: &ModuleRigOptions,
    max_attempts: usize,
    guidance: &ModuleRigGuidance,
) -> Result<AiModuleRig, AiError> {
    let layout = ModuleLayout::new(doc, parts)?;
    let Answered {
        mut proposal,
        params,
        reason,
        unapplied,
        rejected,
    } = ask_until_valid(
        provider,
        &module_rig_output_schema(),
        max_attempts,
        |rejected| request(doc, &layout, image, rejected, guidance),
        module_params_from_answer,
        |params| build(doc, &layout, params, options),
    )?;
    proposal.rationale = format!(
        "Values chosen by the AI: sway scale {:.2}, pendulum length {:.2} ({})\n{}",
        params.sway_scale, params.sway_length, reason, proposal.rationale
    );
    Ok(AiModuleRig {
        proposal,
        params,
        reason,
        unapplied,
        rejected,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn params_outside_the_ranges_are_errors() {
        assert!(ModuleRigParams::default().check().is_empty());
        let bad = ModuleRigParams {
            sway_scale: 3.0,
            sway_length: f32::NAN,
        };
        let errors = bad.check();
        assert_eq!(errors.len(), 2);
        assert!(errors[0].contains("sway_scale is 3"), "{errors:?}");
    }

    #[test]
    fn the_schema_requires_every_value() {
        let schema = module_rig_output_schema();
        assert_eq!(schema["required"].as_array().unwrap().len(), 3);
        assert_eq!(schema["additionalProperties"], json!(false));
        let (params, reason) = module_params_from_answer(&json!({
            "sway_scale": 1.4, "sway_length": 0.8, "reason": "soft ribbon"
        }))
        .unwrap();
        assert_eq!(params.sway_scale, 1.4);
        assert_eq!(reason, "soft ribbon");
        assert!(module_params_from_answer(&json!({ "sway_scale": 1.0 })).is_err());
    }

    #[test]
    fn cells_are_the_product_of_the_keys() {
        let axes = vec![
            (ParamId::new("A"), vec![-1.0, 0.0, 1.0]),
            (ParamId::new("B"), vec![0.0, 1.0]),
        ];
        let cells = cells(&axes);
        assert_eq!(cells.len(), 6);
        assert_eq!(cells[0], vec![-1.0, 0.0]);
        assert_eq!(cells[5], vec![1.0, 1.0]);
    }

    #[test]
    fn overlap_is_the_covered_fraction() {
        let r = |x0, y0, x1, y1| Rect {
            min: Vec2::new(x0, y0),
            max: Vec2::new(x1, y1),
        };
        assert_eq!(
            overlap(r(0.0, 0.0, 10.0, 10.0), r(0.0, 0.0, 20.0, 20.0)),
            1.0
        );
        assert!((overlap(r(0.0, 0.0, 10.0, 10.0), r(5.0, 0.0, 20.0, 20.0)) - 0.5).abs() < 1e-6);
        assert_eq!(
            overlap(r(0.0, 0.0, 10.0, 10.0), r(30.0, 30.0, 40.0, 40.0)),
            0.0
        );
    }
}
