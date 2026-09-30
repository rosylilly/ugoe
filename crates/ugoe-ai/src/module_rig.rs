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
//! - **揺れ:** 揺れる意味ラベル (髪・尻尾・耳飾り・首の飾り・下の服) の衣装は、揺れ用のワープで包み、
//!   追従のワープに取り付けた振り子の動的物理で揺らす (`crate::rig` の髪揺れと同じ方法)。
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
use ugoe_editor_core::space::{ParameterValues, canvas_to_local};
use ugoe_editor_core::{Command, Document, KeyformTarget, ModuleContext};
use ugoe_model::deformer::WarpKeyform;
use ugoe_model::physics::VertexOwner;
use ugoe_model::{
    Deformer, DeformerId, DeformerKind, KeyformGrid, KeyformInterpolation, MeshId, Model, ParamId,
    standard,
};
use ugoe_rigging::ParentWorld;

use crate::classify::{Label, LayerLabel, Side};
use crate::instructions::{Instructions, split_unapplied, system_rules};
use crate::rig::hair::{self, Sway};
use crate::rig::{
    MeshPart, Rect, RejectedAnswer, Rig, mesh_bounds, mesh_name, mesh_parts_where, template,
};
use crate::turn::Builder;
use crate::{AiError, Proposal, Provider, ProviderRequest};

/// 追従のワープの分割数 (一辺のセルの数)。本体の顔のワープ (6×6 の曲線補間) の丸みを写せるよう細かめにする
/// (見本の胸から上の人物で、4 では振り向きの移動量の 3 割、8 では 1 割ほどずれる)。
const FOLLOW_CELLS: u16 = 8;
/// 衣装が本体のワープに従うのに要る、衣装の外接矩形のうちワープの覆う割合 (ワープは外側へも外挿するので小さめ。
/// 顔の横に垂れる耳飾りも顔のワープに従えるように)。
const MIN_OVERLAP: f32 = 0.1;
/// 追従のワープのキーの組の数の上限 (軸のキーの数の積)。超える本体のワープには従わない。
const MAX_FOLLOW_CELLS: usize = 81;

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
        Self::RANGES
            .iter()
            .zip(self.values())
            .filter(|((_, min, max), v)| !(v.is_finite() && *min <= *v && *v <= *max))
            .map(|((name, min, max), v)| {
                format!("{name} is {v}, but it must be between {min} and {max}")
            })
            .collect()
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
        // 衣装のメッシュのうち、取り付け先の直下のもの (自分で組んだデフォーマの中のものは変えない)。
        let mut top = Vec::new();
        for mesh in &model.meshes {
            if context.is_base_mesh(&mesh.id) || mesh.mask_only || !bounds.contains_key(&mesh.id) {
                continue;
            }
            if mesh.parent == anchor {
                top.push(mesh.id.clone());
            } else {
                skipped.push(format!(
                    "`{}` はモジュールのデフォーマの中にあるので変えない",
                    mesh_name(model, &mesh.id)
                ));
            }
        }
        if top.is_empty() {
            return Err(AiError::InvalidRequest(
                "the module has no meshes directly under the slot's anchor; import the costume PSD first"
                    .into(),
            ));
        }
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

        // 本体のメッシュを、いちばん近い候補の祖先のワープに振り分ける (そのワープで動く本体の絵)。
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
        let mut groups: Vec<Group> = Vec::new();
        for id in &top {
            let rect = bounds[id];
            let order = model
                .meshes
                .iter()
                .find(|m| &m.id == id)
                .and_then(draw_order)
                .unwrap_or(0.0);
            // 衣装と重なる本体の絵のうち、描画順が衣装に最も近いもの (衣装が重なる層) を動かすワープに従う。
            let gap = |f: &Followed| {
                owned
                    .get(&f.deformer)
                    .into_iter()
                    .flatten()
                    .filter(|(r, _)| overlap(rect, *r) > 0.0)
                    .map(|(_, o)| (o - order).abs())
                    .fold(f32::INFINITY, f32::min)
            };
            let followed = candidates
                .iter()
                .filter(|f| overlap(rect, f.region) >= MIN_OVERLAP)
                .min_by(|a, b| {
                    gap(a)
                        .total_cmp(&gap(b))
                        .then(b.depth.cmp(&a.depth))
                        .then(overlap(rect, b.region).total_cmp(&overlap(rect, a.region)))
                })
                .cloned();
            let key = followed.as_ref().map(|f| f.deformer.clone());
            let group = match groups
                .iter_mut()
                .find(|g| g.followed.as_ref().map(|f| &f.deformer) == key.as_ref())
            {
                Some(g) => g,
                None => {
                    groups.push(Group {
                        followed,
                        meshes: Vec::new(),
                        sways: Vec::new(),
                    });
                    groups
                        .last_mut()
                        .ok_or_else(|| template("no group".into()))?
                }
            };
            group.meshes.push(id.clone());
            let Some(part) = parts.get(id) else {
                continue;
            };
            let Some((sway, input)) = sway_of(part.label) else {
                continue;
            };
            let has = |p: &str| usable.contains(&ParamId::new(p));
            let input = match input {
                SwayInput::Body if has(standard::BODY_ANGLE_X) => standard::BODY_ANGLE_X,
                _ if has(standard::ANGLE_X) => standard::ANGLE_X,
                _ => {
                    skipped.push(format!(
                        "{} (`{}`) は本体に `AngleX` が無いので揺らさない",
                        part.label,
                        mesh_name(model, id)
                    ));
                    continue;
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
        Ok(Self {
            anchor,
            groups,
            labels: parts.clone(),
            bounds,
            skipped,
        })
    }

    fn union(&self, ids: &[MeshId]) -> Option<Rect> {
        ids.iter()
            .filter_map(|id| self.bounds.get(id).copied())
            .reduce(Rect::union)
    }
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
        let world = ParentWorld::new(model, Some(&deformer.id))
            .map_err(|e| template(format!("deformer `{}`: {e}", deformer.id)))?;
        let samples: Vec<Vec2> = (0..=4)
            .flat_map(|j| (0..=4).map(move |i| Vec2::new(i as f32 / 4.0, j as f32 / 4.0)))
            .collect();
        let Some(region) = Rect::of(&world.to_canvas(&samples)) else {
            continue;
        };
        let cells: usize = axes.iter().map(|(_, k)| k.len()).product();
        if cells > MAX_FOLLOW_CELLS {
            // 衣装と重なるものだけ理由に書く。
            if costume.iter().any(|r| overlap(*r, region) >= MIN_OVERLAP) {
                skipped.push(format!(
                    "本体のワープ `{}` はキーの組が {cells} 個あり多すぎるので、それに従うワープは作らない",
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
/// ([`module_mesh_parts`] でレイヤーの分類から作れる。ラベルのないメッシュは追従だけ)。返す提案は `doc` に
/// `Command::Batch` として適用でき、パラメータを足さず本体を変えない。組むものがなければ [`AiError::InvalidRequest`]。
pub fn propose_module_rig(
    doc: &Document,
    parts: &BTreeMap<MeshId, MeshPart>,
    params: &ModuleRigParams,
) -> Result<Proposal, AiError> {
    let layout = ModuleLayout::new(doc, parts)?;
    build(doc, &layout, params)
}

fn build(
    doc: &Document,
    layout: &ModuleLayout,
    params: &ModuleRigParams,
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
    };
    let anchor_name = layout
        .anchor
        .as_ref()
        .map_or_else(|| "キャンバス".to_owned(), |a| format!("`{a}`"));
    let mut rationale = vec![format!(
        "衣装はスロット `{}` の取り付け先 {anchor_name} の子なので、その動きはそのまま受け継ぐ。パラメータは足さない",
        context.slot().id
    )];
    for group in &layout.groups {
        rig_group(&mut rig, context, layout, group, params, &mut rationale)?;
    }
    if rig.b.commands.is_empty() {
        return Err(AiError::InvalidRequest(format!(
            "nothing to rig: the costume already follows the slot's anchor, no base warp under it moves the costume, \
             and no costume part is labeled to sway{}",
            if layout.skipped.is_empty() {
                String::new()
            } else {
                format!(" ({})", layout.skipped.join("; "))
            }
        )));
    }
    rationale.extend(
        layout
            .skipped
            .iter()
            .map(|s| format!("組まなかったもの: {s}")),
    );
    rig.b.validate()?;
    if let Some(Err(e)) = rig.b.doc.split_module() {
        return Err(AiError::InvalidProposal(format!(
            "the proposal cannot be kept as a module: {e}"
        )));
    }
    Ok(Proposal {
        title: "モジュールの自動リギング (取り付け先への追従・揺れ)".into(),
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
    rationale: &mut Vec<String>,
) -> Result<(), AiError> {
    let names = group
        .meshes
        .iter()
        .map(|id| format!("`{}`", mesh_name(rig.model(), id)))
        .collect::<Vec<_>>()
        .join("・");
    let sways = params.sway_scale > 0.0 && !group.sways.is_empty();
    if group.followed.is_none() && !sways {
        rationale.push(format!(
            "{}: 取り付け先の下に覆う本体のワープがないので、取り付け先の動きだけに従う",
            names
        ));
        return Ok(());
    }
    // 追従のワープ (揺れの振り子の取り付け先も兼ねる)。
    let (base, name) = match &group.followed {
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
    let follow = rig.create_warp_as(
        id,
        &name,
        &group.meshes,
        FOLLOW_CELLS,
        FOLLOW_CELLS,
        layout.anchor.as_ref(),
    )?;
    match &group.followed {
        Some(followed) => {
            follow_keyforms(rig, &follow, followed)?;
            let axes: Vec<String> = followed
                .axes
                .iter()
                .map(|(p, keys)| format!("`{p}` ({} キー)", keys.len()))
                .collect();
            let mut line = format!(
                "追従: {} を包むワープ `{follow}` に、本体のワープ `{}` の {} での変形を写す",
                names,
                followed.deformer,
                axes.join("・")
            );
            if !followed.ignored.is_empty() {
                let _ = write!(
                    line,
                    " (スロットが公開していない {} には従わない)",
                    followed
                        .ignored
                        .iter()
                        .map(|p| format!("`{p}`"))
                        .collect::<Vec<_>>()
                        .join("・")
                );
            }
            rationale.push(line);
        }
        None => rationale.push(format!(
            "{}: 覆う本体のワープがないので、揺れの取り付け先のワープ `{follow}` だけを作る",
            names
        )),
    }
    if !sways {
        if !group.sways.is_empty() {
            rationale.push("揺れ: 揺れの大きさが 0 なので揺らさない".into());
        }
        return Ok(());
    }
    for sway in &group.sways {
        rig_sway(rig, context, layout, &follow, sway, params, rationale)?;
    }
    Ok(())
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
    let mut world = ParentWorld::new(rig.model(), Some(&followed.deformer))
        .map_err(|e| template(format!("deformer `{}`: {e}", followed.deformer)))?;
    let guesses: Vec<Vec2> = rest.iter().map(|p| followed.region.normalize(*p)).collect();
    let inner = world
        .from_canvas(&rest, &guesses)
        .map_err(|e| template(format!("deformer `{}`: {e}", followed.deformer)))?;
    let defaults: BTreeMap<&ParamId, f32> = rig
        .model()
        .parameters
        .iter()
        .map(|p| (&p.id, p.default))
        .collect();
    let defaults: Vec<f32> = followed
        .axes
        .iter()
        .map(|(p, _)| defaults.get(p).copied().unwrap_or(0.0))
        .collect();
    for cell in cells(&followed.axes) {
        if cell.iter().zip(&defaults).all(|(v, d)| v == d) {
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
            .map_err(|e| template(format!("{target}: {e}")))?;
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
            _ => Err(template(format!("{target} is not a warp deformer"))),
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

/// 揺れ: 揺れる衣装を包む揺れ用のワープを追従のワープの中に作り、追従のワープに取り付けた振り子の
/// 先の横の変位を、制御点に重み付きで足す (`crate::rig` の髪揺れと同じ)。
#[allow(clippy::too_many_arguments)]
fn rig_sway(
    rig: &mut Rig,
    context: &ModuleContext,
    layout: &ModuleLayout,
    follow: &DeformerId,
    group: &SwayGroup,
    params: &ModuleRigParams,
    rationale: &mut Vec<String>,
) -> Result<(), AiError> {
    let rect = layout
        .union(&group.meshes)
        .ok_or_else(|| template(format!("{} has no bounds", group.label)))?;
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
        Some(follow),
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
        .map(|p| p.rigs.iter().map(|r| r.id.clone()).collect())
        .unwrap_or_default();
    let rig_id = fresh_id(context, &base, existing.iter().map(String::as_str));
    let physics = rig.pendulum(
        &rig_id,
        &format!("Sway {} {}", group.label, group.side.as_str()),
        follow,
        &target,
        group.input.as_str(),
        root,
        &lengths,
        sway,
        shift,
        VertexOwner::Warp(warp),
        weights,
    )?;
    rig.apply(Command::AddPhysicsRig { rig: physics })?;
    rationale.push(format!(
        "揺れ: {} ({}、{} 枚) を、追従のワープ `{follow}` に取り付けた {} 区間 (長さ {length:.0} px) の振り子で揺らす (重みの最大 {max:.2}、`{}` の端で根元が {shift:.0} px 動く)",
        group.label,
        group.side.as_str(),
        group.meshes.len(),
        sway.sections,
        group.input,
    ));
    Ok(())
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
    let mut properties = serde_json::Map::new();
    for (name, min, max) in ModuleRigParams::RANGES {
        properties.insert(
            name.to_owned(),
            json!({ "type": "number", "description": format!("From {min} to {max}.") }),
        );
    }
    properties.insert(
        "reason".into(),
        json!({ "type": "string", "description": "One or two sentences on why you chose these values." }),
    );
    let required: Vec<&str> = ModuleRigParams::RANGES
        .iter()
        .map(|(name, ..)| *name)
        .chain(["reason"])
        .collect();
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    })
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
    let mut s = String::new();
    if guidance.previous.is_some() {
        s.push_str(
            "\n\nThis is a revision. Your previous answer is given in the prompt. Start from its \
             values and change only what the user's instructions call for.",
        );
    }
    if guidance.instructions.is_some() {
        let labels = if guidance.labels_follow_instructions {
            "Parts about layer labels were already applied when the layers were classified; do \
             not list them."
        } else {
            "Layer labels cannot change in this step, so list requests to relabel layers as not \
             applied too (the user can classify the layers again)."
        };
        s.push_str(&system_rules(&format!(
            "reflect them only through the two values above. For every part you cannot reflect \
             that way (for example new parameters, which a module cannot add, other parts, a \
             different rig structure, motions, or changes to the drawing), do not pretend: end \
             `reason` with one line per such part, starting with \"{}\" and saying what was asked \
             and why it cannot be done. {labels}",
            crate::instructions::NOT_APPLIED
        )));
    }
    s
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
    for (i, r) in rejected.iter().enumerate() {
        let _ = write!(
            s,
            "\nAttempt {} was rejected.\nAnswer: {}\nErrors: {}\n",
            i + 1,
            r.answer,
            r.error
        );
    }
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
        max_attempts,
        &ModuleRigGuidance::default(),
    )
}

/// [`propose_module_rig_with_ai`] の、利用者の注文と前の答え (作り直し) を添える版
/// (`docs/ai-integration.md#利用者の注文`)。2 つの値で反映できない注文は [`AiModuleRig::unapplied`] に返す。
pub fn propose_module_rig_with_ai_guided(
    provider: &dyn Provider,
    doc: &Document,
    parts: &BTreeMap<MeshId, MeshPart>,
    image: &[u8],
    max_attempts: usize,
    guidance: &ModuleRigGuidance,
) -> Result<AiModuleRig, AiError> {
    let layout = ModuleLayout::new(doc, parts)?;
    let schema = module_rig_output_schema();
    let mut rejected: Vec<RejectedAnswer> = Vec::new();
    for _ in 0..max_attempts {
        let request = request(doc, &layout, image, &rejected, guidance);
        let value = provider.complete_json(&request, &schema)?;
        let (params, reason) = match module_params_from_answer(&value) {
            Ok(answer) => answer,
            Err(e) => {
                rejected.push(RejectedAnswer {
                    answer: value,
                    error: match e {
                        AiError::InvalidProposal(error) => error,
                        e => e.to_string(),
                    },
                });
                continue;
            }
        };
        match build(doc, &layout, &params) {
            Ok(mut proposal) => {
                let (reason, unapplied) = split_unapplied(&reason);
                proposal.rationale = format!(
                    "AI が決めた値: 揺れの大きさ {:.2}、振り子の長さ {:.2} ({})\n{}",
                    params.sway_scale, params.sway_length, reason, proposal.rationale
                );
                return Ok(AiModuleRig {
                    proposal,
                    params,
                    reason,
                    unapplied,
                    rejected,
                });
            }
            Err(AiError::InvalidProposal(error)) => {
                rejected.push(RejectedAnswer {
                    answer: value,
                    error,
                });
            }
            Err(e) => return Err(e),
        }
    }
    Err(AiError::InvalidProposal(format!(
        "no valid rig after {max_attempts} attempts{}",
        rejected
            .last()
            .map(|r| format!(": {}", r.error))
            .unwrap_or_default()
    )))
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
