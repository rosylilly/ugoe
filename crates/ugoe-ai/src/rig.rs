//! 意味ラベルからの自動リギングの提案 (`docs/ai-integration.md#自動リギング`)。
//!
//! PSD を取り込んだ直後のモデル (メッシュだけでデフォーマのないもの) と、メッシュごとの意味ラベル
//! ([`mesh_parts`] でレイヤーの分類から作る) から、アバタープロファイルのレベル 1 のパラメータ
//! (`docs/spec/avatar-profile.md`) を動かすリグを [`Command`] の列として提案する。
//!
//! - **テンプレート** ([`propose_rig`]): ラベルの組み合わせからデフォーマの構造とキーフォームの作り方を
//!   決める (決定的)。
//!   - 顔の角度: 頭の回転デフォーマ (`AngleZ`) と、頭のメッシュを包むワープ (`AngleX` / `AngleY`)。
//!     ワープは深度からの回転で振り向かせる ([`crate::turn`])。深度は推定した深度マップか、
//!     なければ顔の外接矩形に内接する楕円体 (規則) を使い、どちらも意味ラベルで補正する。
//!   - 目の開閉 (`EyeLOpen` / `EyeROpen`): 開いた目のメッシュを包むワープを、閉じる線へ縦につぶす。
//!     閉じた目の絵 (`eye_closed`) があれば、つぶれた所で不透明度を入れ替える。
//!   - 口の開閉 (`MouthOpenY`): 開いた口と閉じた口の絵があれば、開いた口を閉じた口の大きさの細い
//!     隙間から開いて入れ替える。1 枚の口なら、口を包むワープの下側を下げて縦に開く。
//! - **AI が決める値** ([`RigParams`]): テンプレートでは決められない値 (振り向きの量・目の閉じ方・
//!   口の開き方) を [`Provider`] に画像から決めさせる ([`propose_rig_with_ai`])。
//!   答えが範囲の外であったり、組んだリグが検証を通らなかったりしたら、エラーを添えて問い直す
//!   (回数の上限あり)。
//!
//! 提案は複製したドキュメントに適用し、`ugoe_model::validate` を通ることを確かめてから返す (ADR 0006)。

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde::{Deserialize, Serialize};
use serde_json::json;
use ugoe_core::Vec2;
use ugoe_editor_core::ids::new_deformer_id;
use ugoe_editor_core::space::{ParameterValues, canvas_to_local, local_to_canvas};
use ugoe_editor_core::{ChildRef, Command, Keyform, KeyformCell, KeyformTarget};
use ugoe_model::deformer::{RotationKeyform, WarpKeyform};
use ugoe_model::mesh::MeshKeyform;
use ugoe_model::{
    DeformerId, DeformerKind, KeyformGrid, KeyformInterpolation, MeshId, Model, ParamId, standard,
};
use ugoe_runtime::{CompiledModel, ModelInstance};

use crate::classify::{Label, LabelCategory, LayerLabel, Side};
use crate::depth::DepthMap;
use crate::turn::{Builder, TurnRole, TurnTarget, plan_turn, push_turn};
use crate::{AiError, Proposal, Provider, ProviderRequest};

/// メッシュの意味ラベル (レイヤーの分類の結果をメッシュに対応付けたもの)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshPart {
    pub label: Label,
    /// キャラクター自身から見た左右。
    pub side: Side,
}

/// レイヤーの分類の結果 ([`crate::classify`]) を、名前でメッシュに対応付ける。
///
/// PSD の取り込み (`ugoe_editor_core::import_psd`) はメッシュの名前をレイヤーの名前にするので、
/// `layer_path` の最後の名前と同じ名前のメッシュにラベルを付ける。同じ名前のメッシュやレイヤーが
/// 複数あって決められないものは付けない。
pub fn mesh_parts(model: &Model, layers: &[LayerLabel]) -> BTreeMap<MeshId, MeshPart> {
    let mut by_name: BTreeMap<&str, Option<MeshPart>> = BTreeMap::new();
    for layer in layers {
        if let Some(name) = layer.layer_path.last() {
            let part = MeshPart {
                label: layer.label,
                side: layer.side,
            };
            by_name
                .entry(name.as_str())
                .and_modify(|p| {
                    if *p != Some(part) {
                        *p = None;
                    }
                })
                .or_insert(Some(part));
        }
    }
    let mut parts = BTreeMap::new();
    for mesh in &model.meshes {
        let same_name = model.meshes.iter().filter(|m| m.name == mesh.name).count();
        if same_name != 1 {
            continue;
        }
        if let Some(&Some(part)) = by_name.get(mesh.name.as_str()) {
            parts.insert(mesh.id.clone(), part);
        }
    }
    parts
}

/// テンプレートでは決められない、リグの数値。AI が画像から決めるか、既定値を使う。
///
/// 範囲は [`RigParams::RANGES`]。範囲の外の値は丸めずにエラーにする ([`RigParams::check`])。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RigParams {
    /// 振り向きの量: `AngleX` / `AngleY` の値 (度) に掛けて、深度からの回転の角度にする係数
    /// (`DepthRotation::angle_scale`)。絵の顔は大きく回すと中身がずれすぎるので 1 より小さくする。
    pub turn_angle_scale: f32,
    /// 目が閉じる線の高さ (開いた目の外接矩形の上端 0〜下端 1)。閉じた目の絵がない目にだけ使う
    /// (絵があれば、その中心の高さへ閉じる)。
    pub eye_close_line: f32,
    /// 閉じた目の弧の曲がり (開いた目の高さに対する割合)。正で中ほどが下がる (下向きの弧、ふつうの
    /// 閉じ目)、負で上がる (笑った閉じ目)。
    pub eye_close_bend: f32,
    /// 1 枚の口を開いたときに下唇を下げる量 (口の幅に対する割合)。開いた口の絵があれば使わない。
    pub mouth_open_height: f32,
}

impl Default for RigParams {
    fn default() -> Self {
        Self {
            turn_angle_scale: 0.7,
            eye_close_line: 0.7,
            eye_close_bend: 0.04,
            mouth_open_height: 0.35,
        }
    }
}

impl RigParams {
    /// 値の名前と、許す範囲 (両端を含む)。
    pub const RANGES: [(&'static str, f32, f32); 4] = [
        ("turn_angle_scale", 0.3, 1.2),
        ("eye_close_line", 0.3, 0.9),
        ("eye_close_bend", -0.15, 0.15),
        ("mouth_open_height", 0.1, 1.0),
    ];

    fn values(&self) -> [f32; 4] {
        [
            self.turn_angle_scale,
            self.eye_close_line,
            self.eye_close_bend,
            self.mouth_open_height,
        ]
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

/// 振り向き (`AngleX` / `AngleY`) の深度の出どころ。
#[derive(Debug, Clone, Default, PartialEq)]
pub enum TurnDepth {
    /// 顔の外接矩形に内接する楕円体を深度とする (推定器を使わない)。
    #[default]
    Rules,
    /// 推定した深度マップ (キャンバス全体に対応する。`crate::depth::DepthEstimator` で作る)。
    Map(DepthMap),
}

/// テンプレートの設定。
#[derive(Debug, Clone, PartialEq)]
pub struct RigOptions {
    /// 頭のワープの分割数 (一辺のセルの数)。
    pub head_warp_cells: u16,
    pub turn: TurnDepth,
}

impl Default for RigOptions {
    fn default() -> Self {
        Self {
            head_warp_cells: 6,
            turn: TurnDepth::Rules,
        }
    }
}

/// 目の開閉の切り替え (値は `EyeLOpen` / `EyeROpen`)。
///
/// 開いた目は閉じる線へ縦につぶれていく (1 から `SQUASH_KEY` までで大きく)。つぶれて細くなった所で、閉じた目を
/// 先に不透明にしてから開いた目を消すので、途中の値で目が消えたり、半透明の 2 つの目が重なったりしない
/// (`crates/ugoe-cli/src/sample.rs` の見本と同じ値)。
mod eye_fade {
    /// 開いた目を大きくつぶすキーと、そこでの高さ (開いた目に対する割合)。
    pub const SQUASH_KEY: f32 = 0.35;
    pub const SQUASH_HEIGHT: f32 = 0.22;
    /// 閉じた (0) ときの高さ。
    pub const CLOSED_HEIGHT: f32 = 0.04;
    /// 閉じた目が現れ始める値 (これより上では透明)。
    pub const CLOSED_IN: f32 = 0.36;
    /// 閉じた目が不透明になる値。開いた目はここまで不透明。
    pub const CLOSED_FULL: f32 = 0.3;
    /// 開いた目が消える値。
    pub const OPEN_OUT: f32 = 0.22;
}

/// 口の開閉の切り替え (値は `MouthOpenY`)。開いた口が細い隙間で不透明になってから、閉じた口を消す。
mod mouth_fade {
    /// 開いた口 (細い隙間) が不透明になる値。
    pub const OPEN_FULL: f32 = 0.12;
    /// 閉じた口が消える値。
    pub const CLOSED_OUT: f32 = 0.3;
}

/// 頭 (`AngleZ`) の傾き: パラメータの値 (度) に対する回転の角度の比。
const HEAD_TILT: f32 = 1.0 / 3.0;

/// 頭のメッシュ (頭の回転と振り向きに付いていく) か。首は体に残す。
fn is_head(label: Label) -> bool {
    match label.category() {
        LabelCategory::Face => label != Label::Neck,
        LabelCategory::Eye | LabelCategory::Mouth | LabelCategory::Hair => true,
        LabelCategory::Accessory => {
            matches!(label, Label::Headwear | Label::Earwear | Label::Eyewear)
        }
        _ => false,
    }
}

/// 開いた目の部位 (つぶして閉じる)。眉は目の開閉では動かさない。
fn is_open_eye(label: Label) -> bool {
    matches!(
        label,
        Label::Eye
            | Label::EyeWhite
            | Label::Iris
            | Label::EyeHighlight
            | Label::EyelashUpper
            | Label::EyelashLower
    )
}

/// キャンバス座標の軸に平行な矩形。
#[derive(Debug, Clone, Copy, PartialEq)]
struct Rect {
    min: Vec2,
    max: Vec2,
}

impl Rect {
    fn of(points: &[Vec2]) -> Option<Self> {
        let first = *points.first()?;
        let (min, max) = points
            .iter()
            .fold((first, first), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
        Some(Self { min, max })
    }

    fn union(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    fn size(self) -> Vec2 {
        self.max - self.min
    }

    fn center(self) -> Vec2 {
        (self.min + self.max) * 0.5
    }

    /// 点 `p` の、矩形の中での位置 (左上 (0,0)〜右下 (1,1))。
    fn normalize(self, p: Vec2) -> Vec2 {
        let size = self.size().max(Vec2::splat(f32::EPSILON));
        (p - self.min) / size
    }
}

/// 横の位置 `x` (0〜1) での、中ほどが 1・両端が 0 の弧。
fn arc(x: f32) -> f32 {
    1.0 - (2.0 * x - 1.0).powi(2)
}

/// 片目の部位。
#[derive(Debug, Clone)]
struct EyeParts {
    side: Side,
    open: Vec<MeshId>,
    closed: Vec<MeshId>,
}

impl EyeParts {
    fn parameter(&self) -> &'static str {
        match self.side {
            Side::Right => standard::EYE_R_OPEN,
            _ => standard::EYE_L_OPEN,
        }
    }
}

/// 口の部位。
#[derive(Debug, Clone)]
enum MouthParts {
    /// 開いた口 (と口の中・歯・舌) と閉じた口の絵がある。
    Pair {
        open: Vec<MeshId>,
        closed: Vec<MeshId>,
    },
    /// 開き・閉じに分かれていない口。
    Single(Vec<MeshId>),
}

/// ラベルから決めた、リグの対象の部位と基準の姿の外接矩形。
#[derive(Debug, Clone)]
struct Layout {
    face: Rect,
    head: Vec<MeshId>,
    eyes: Vec<EyeParts>,
    mouth: Option<MouthParts>,
    /// メッシュの基準の姿の外接矩形 (キャンバス座標)。
    bounds: BTreeMap<MeshId, Rect>,
    /// 組めなかった部位 (提案の理由に書く)。
    skipped: Vec<String>,
}

impl Layout {
    fn new(model: &Model, parts: &BTreeMap<MeshId, MeshPart>) -> Result<Self, AiError> {
        let compiled = CompiledModel::compile(model.clone())
            .map_err(|e| AiError::InvalidRequest(format!("the model cannot be evaluated: {e}")))?;
        let mut instance = ModelInstance::new(compiled);
        instance.update();
        let mut bounds = BTreeMap::new();
        for mesh in &model.meshes {
            let Some(index) = instance.compiled().mesh_index(mesh.id.as_str()) else {
                continue;
            };
            if let Some(rect) = Rect::of(&instance.drawables()[index].positions) {
                bounds.insert(mesh.id.clone(), rect);
            }
        }
        // 取り込みの順 (描画順) に並べる。
        let labeled: Vec<(&MeshId, MeshPart)> = model
            .meshes
            .iter()
            .filter(|m| !m.mask_only && bounds.contains_key(&m.id))
            .filter_map(|m| parts.get(&m.id).map(|p| (&m.id, *p)))
            .collect();
        let with = |pred: &dyn Fn(Label) -> bool| -> Vec<MeshId> {
            labeled
                .iter()
                .filter(|(_, p)| pred(p.label))
                .map(|(id, _)| (*id).clone())
                .collect()
        };
        let union = |ids: &[MeshId]| {
            ids.iter()
                .filter_map(|id| bounds.get(id).copied())
                .reduce(Rect::union)
        };

        let face_ids = with(&|l| l == Label::Face);
        let face = union(&face_ids).ok_or_else(|| {
            AiError::InvalidRequest("no mesh is labeled `face`; cannot place the head".into())
        })?;
        let head = with(&is_head);
        let mut skipped = Vec::new();

        // 目: 左右はラベルの左右、中央のものは顔の中心との位置で決める (画像の右がキャラクターの左)。
        let mut eyes: Vec<EyeParts> = [Side::Left, Side::Right]
            .into_iter()
            .map(|side| EyeParts {
                side,
                open: Vec::new(),
                closed: Vec::new(),
            })
            .collect();
        for (id, part) in &labeled {
            let closed = part.label == Label::EyeClosed;
            if !closed && !is_open_eye(part.label) {
                continue;
            }
            let side = match part.side {
                Side::Center => {
                    let rect = bounds[*id];
                    let cx = face.center().x;
                    if rect.min.x >= cx {
                        Side::Left
                    } else if rect.max.x <= cx {
                        Side::Right
                    } else {
                        skipped.push(format!(
                            "{} (`{}`) は左右の目にまたがるので開閉を付けない",
                            part.label,
                            mesh_name(model, id)
                        ));
                        continue;
                    }
                }
                side => side,
            };
            if let Some(eye) = eyes.iter_mut().find(|e| e.side == side) {
                if closed {
                    eye.closed.push((*id).clone());
                } else {
                    eye.open.push((*id).clone());
                }
            }
        }
        eyes.retain(|eye| {
            if eye.open.is_empty() {
                skipped.push(format!(
                    "{} の目 (開いた目の部位) がないので `{}` を作らない",
                    if eye.side == Side::Left { "左" } else { "右" },
                    eye.parameter()
                ));
            }
            !eye.open.is_empty()
        });

        let open = with(&|l| {
            matches!(
                l,
                Label::MouthOpen | Label::MouthInner | Label::Teeth | Label::Tongue
            )
        });
        let closed = with(&|l| l == Label::MouthClosed);
        let mouth = if with(&|l| l == Label::MouthOpen).is_empty() || closed.is_empty() {
            let all = with(&|l| l.category() == LabelCategory::Mouth);
            if all.is_empty() {
                skipped.push("口がないので `MouthOpenY` を作らない".into());
                None
            } else {
                Some(MouthParts::Single(all))
            }
        } else {
            Some(MouthParts::Pair { open, closed })
        };

        Ok(Self {
            face,
            head,
            eyes,
            mouth,
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

fn mesh_name<'a>(model: &'a Model, id: &'a MeshId) -> &'a str {
    model
        .meshes
        .iter()
        .find(|m| &m.id == id)
        .map_or(id.as_str(), |m| m.name.as_str())
}

/// ラベルのテンプレートでリグを提案する (AI を使わない)。
///
/// `parts` はメッシュの意味ラベル ([`mesh_parts`] でレイヤーの分類から作れる)。`face` のラベルの
/// メッシュが必要。`params` はテンプレートで決められない値 (AI を使わないなら `RigParams::default()`)。
/// 返す提案は `model` に `Command::Batch` として適用でき、`ugoe_model::validate` を通る。
pub fn propose_rig(
    model: &Model,
    parts: &BTreeMap<MeshId, MeshPart>,
    params: &RigParams,
    options: &RigOptions,
) -> Result<Proposal, AiError> {
    let layout = Layout::new(model, parts)?;
    build(model, parts, &layout, params, options)
}

fn build(
    model: &Model,
    parts: &BTreeMap<MeshId, MeshPart>,
    layout: &Layout,
    params: &RigParams,
    options: &RigOptions,
) -> Result<Proposal, AiError> {
    let errors = params.check();
    if !errors.is_empty() {
        return Err(AiError::InvalidProposal(errors.join("; ")));
    }
    let mut rig = Rig {
        b: Builder::new(model),
    };
    let mut rationale = Vec::new();
    rig.rig_head(layout, parts, params, options, &mut rationale)?;
    for eye in &layout.eyes {
        rig.rig_eye(layout, eye, params, &mut rationale)?;
    }
    if let [left, right] = layout.eyes.as_slice() {
        rig.apply(Command::SetParameterMirror {
            id: ParamId::new(left.parameter()),
            mirror: Some(ParamId::new(right.parameter())),
        })?;
    }
    match &layout.mouth {
        Some(MouthParts::Pair { open, closed }) => {
            rig.rig_mouth_pair(layout, open, closed)?;
            rationale.push(format!(
                "口: 開いた口 ({} 枚) を閉じた口の大きさの細い隙間から開き、`MouthOpenY` {} で閉じた口 ({} 枚) と入れ替える",
                open.len(),
                mouth_fade::OPEN_FULL,
                closed.len()
            ));
        }
        Some(MouthParts::Single(ids)) => {
            rig.rig_mouth_single(layout, ids, params)?;
            rationale.push(format!(
                "口: 開き・閉じの絵がないので、口 ({} 枚) を包むワープの下側を口の幅の {:.2} 倍下げて開く",
                ids.len(),
                params.mouth_open_height
            ));
        }
        None => {}
    }
    rationale.extend(
        layout
            .skipped
            .iter()
            .map(|s| format!("組まなかったもの: {s}")),
    );
    rig.b.validate()?;
    Ok(Proposal {
        title: "自動リギング (顔の角度・目・口)".into(),
        rationale: rationale.join("\n"),
        commands: rig.b.commands,
    })
}

/// 顔の外接矩形に内接する楕円体の深度マップ (中心 1・楕円の外 0)。
fn dome_depth(model: &Model, face: Rect) -> Result<DepthMap, AiError> {
    const SIZE: u32 = 128;
    let canvas = Vec2::new(model.meta.canvas.width, model.meta.canvas.height);
    let radius = (face.size() * 0.5).max(Vec2::splat(1.0));
    let values = (0..SIZE)
        .flat_map(|j| (0..SIZE).map(move |i| (i, j)))
        .map(|(i, j)| {
            let p = (Vec2::new(i as f32, j as f32) + 0.5) / SIZE as f32 * canvas;
            let e = (p - face.center()) / radius;
            (1.0 - e.dot(e)).max(0.0).sqrt()
        })
        .collect();
    DepthMap::new(SIZE, SIZE, values)
}

/// 組み立て中のドキュメントと、要素の探索・形の編集の補助 (`crates/ugoe-cli/src/sample.rs` の見本と同じ方法)。
struct Rig {
    b: Builder,
}

impl Rig {
    fn model(&self) -> &Model {
        self.b.doc.model()
    }

    fn apply(&mut self, command: Command) -> Result<(), AiError> {
        self.b.apply(command)
    }

    fn new_deformer(&self, base: &str) -> DeformerId {
        new_deformer_id(self.model(), base)
    }

    fn create_warp(
        &mut self,
        base: &str,
        name: &str,
        children: &[MeshId],
        rows: u16,
        cols: u16,
    ) -> Result<DeformerId, AiError> {
        let id = self.new_deformer(base);
        self.apply(Command::CreateWarpDeformer {
            id: Some(id.clone()),
            name: name.to_owned(),
            children: children.iter().cloned().map(ChildRef::Mesh).collect(),
            parts: Vec::new(),
            rows,
            cols,
            parent: None,
        })?;
        Ok(id)
    }

    /// 要素の格子のセル `cell` (書かない軸は既定値) の点 (親の座標系)・キーフォームと、すべての軸を書いたセル。
    fn keyform_at(
        &self,
        target: &KeyformTarget,
        cell: &[(&str, f32)],
    ) -> Result<(Vec<Vec2>, Keyform, KeyformCell), AiError> {
        let model = self.model();
        let mut values: BTreeMap<ParamId, f32> = model
            .parameters
            .iter()
            .map(|p| (p.id.clone(), p.default))
            .collect();
        for (parameter, value) in cell {
            values.insert(ParamId::new(*parameter), *value);
        }
        fn find<T: Clone>(
            grid: &KeyformGrid<T>,
            values: &BTreeMap<ParamId, f32>,
        ) -> Result<(T, KeyformCell), AiError> {
            let mut index = 0;
            let mut stride = 1;
            let mut cell = KeyformCell::new();
            for binding in &grid.bindings {
                let value = values.get(&binding.parameter).copied().unwrap_or(0.0);
                let key = binding
                    .keys
                    .iter()
                    .position(|k| *k == value)
                    .ok_or_else(|| {
                        template(format!("axis {} has no key {value}", binding.parameter))
                    })?;
                index += key * stride;
                stride *= binding.keys.len();
                cell.insert(binding.parameter.clone(), value);
            }
            let keyform = grid
                .keyforms
                .get(index)
                .cloned()
                .ok_or_else(|| template(format!("cell {index} is missing")))?;
            Ok((keyform, cell))
        }
        match target {
            KeyformTarget::Mesh(id) => {
                let mesh = model
                    .meshes
                    .iter()
                    .find(|m| m.id == *id)
                    .ok_or_else(|| template(format!("no mesh `{id}`")))?;
                let (keyform, cell) = find(&mesh.keyforms, &values)?;
                Ok((keyform.positions.clone(), Keyform::Mesh(keyform), cell))
            }
            KeyformTarget::Deformer(id) => {
                let deformer = model
                    .deformers
                    .iter()
                    .find(|d| d.id == *id)
                    .ok_or_else(|| template(format!("no deformer `{id}`")))?;
                match &deformer.kind {
                    DeformerKind::Warp(warp) => {
                        let (keyform, cell) = find(&warp.keyforms, &values)?;
                        Ok((keyform.points.clone(), Keyform::Warp(keyform), cell))
                    }
                    DeformerKind::Rotation(rotation) => {
                        let (keyform, cell) = find(&rotation.keyforms, &values)?;
                        Ok((vec![keyform.origin], Keyform::Rotation(keyform), cell))
                    }
                    DeformerKind::Unknown => {
                        Err(template(format!("deformer `{id}` has an unknown kind")))
                    }
                }
            }
            _ => Err(template(format!("{target} is not supported"))),
        }
    }

    /// 要素の格子の `parameter` の軸にキー `keys` を加える (既定値のキーを先に。あるキーは飛ばす)。
    fn add_keys(
        &mut self,
        target: &KeyformTarget,
        parameter: &str,
        keys: &[f32],
    ) -> Result<(), AiError> {
        let id = ParamId::new(parameter);
        let default = self.b.parameter(&id)?.default;
        let mut ordered = vec![default];
        ordered.extend(keys.iter().copied().filter(|k| *k != default));
        for key in ordered {
            if self.has_key(target, &id, key) {
                continue;
            }
            self.apply(Command::AddKeyformKey {
                target: target.clone(),
                parameter: id.clone(),
                key,
            })?;
        }
        Ok(())
    }

    fn has_key(&self, target: &KeyformTarget, id: &ParamId, key: f32) -> bool {
        fn keys<T>(grid: &KeyformGrid<T>, id: &ParamId) -> Vec<f32> {
            grid.bindings
                .iter()
                .find(|b| &b.parameter == id)
                .map(|b| b.keys.clone())
                .unwrap_or_default()
        }
        let model = self.model();
        let keys = match target {
            KeyformTarget::Mesh(mesh) => model
                .meshes
                .iter()
                .find(|m| &m.id == mesh)
                .map(|m| keys(&m.keyforms, id)),
            KeyformTarget::Deformer(d) => {
                model
                    .deformers
                    .iter()
                    .find(|x| &x.id == d)
                    .and_then(|x| match &x.kind {
                        DeformerKind::Warp(w) => Some(keys(&w.keyforms, id)),
                        DeformerKind::Rotation(r) => Some(keys(&r.keyforms, id)),
                        DeformerKind::Unknown => None,
                    })
            }
            _ => None,
        };
        keys.unwrap_or_default().contains(&key)
    }

    /// 格子のセル `cell` (書かない軸は既定値) を、そのセルの今のキーフォームを `edit` で変えたものにする。
    fn set_cell(
        &mut self,
        target: &KeyformTarget,
        cell: &[(&str, f32)],
        edit: impl FnOnce(Keyform) -> Result<Keyform, AiError>,
    ) -> Result<(), AiError> {
        let (_, current, full_cell) = self.keyform_at(target, cell)?;
        let keyform = edit(current)?;
        self.apply(Command::SetKeyformCell {
            target: target.clone(),
            cell: full_cell,
            keyform,
        })
    }

    /// 格子のセル `cell` の形を、基準の姿の点をキャンバス座標で `offset` だけずらしたものにする。
    /// `offset` は点のキャンバス座標と、要素の外接矩形の中での位置 (0〜1) を受け取る。
    fn shape(
        &mut self,
        target: &KeyformTarget,
        cell: &[(&str, f32)],
        offset: impl Fn(Vec2, Vec2) -> Vec2,
    ) -> Result<(), AiError> {
        let (local, ..) = self.keyform_at(target, &[])?;
        let values = ParameterValues::new();
        let canvas = local_to_canvas(self.model(), target, &values, &local)
            .map_err(|e| template(format!("{target}: {e}")))?;
        let rect = Rect::of(&canvas).ok_or_else(|| template(format!("{target} has no points")))?;
        let moved: Vec<Vec2> = canvas
            .iter()
            .map(|p| *p + offset(*p, rect.normalize(*p)))
            .collect();
        if moved.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
            return Err(AiError::InvalidProposal(format!(
                "{target}: the shape has non-finite points"
            )));
        }
        let points = canvas_to_local(self.model(), target, &values, &moved, &local)
            .map_err(|e| template(format!("{target}: {e}")))?;
        self.set_cell(target, cell, |base| match base {
            Keyform::Mesh(keyform) => Ok(Keyform::Mesh(MeshKeyform {
                positions: points,
                ..keyform
            })),
            Keyform::Warp(keyform) => Ok(Keyform::Warp(WarpKeyform { points, ..keyform })),
            _ => Err(template(format!(
                "{target} is not a mesh or a warp deformer"
            ))),
        })
    }

    /// メッシュ `id` の不透明度を、`parameter` のキーごとに `(キー, 不透明度)` の値にする
    /// (他の軸は既定値のセル。形は変えない)。
    fn fade(&mut self, id: &MeshId, parameter: &str, keys: &[(f32, f32)]) -> Result<(), AiError> {
        let target = KeyformTarget::Mesh(id.clone());
        let values: Vec<f32> = keys.iter().map(|(key, _)| *key).collect();
        self.add_keys(&target, parameter, &values)?;
        for (key, opacity) in keys {
            self.set_cell(&target, &[(parameter, *key)], |base| match base {
                Keyform::Mesh(keyform) => Ok(Keyform::Mesh(MeshKeyform {
                    opacity: *opacity,
                    ..keyform
                })),
                _ => Err(template(format!("{target} is not a mesh"))),
            })?;
        }
        Ok(())
    }

    /// 頭: 傾き (`AngleZ`) の回転デフォーマと、向き (`AngleX` / `AngleY`) のワープ。
    fn rig_head(
        &mut self,
        layout: &Layout,
        parts: &BTreeMap<MeshId, MeshPart>,
        params: &RigParams,
        options: &RigOptions,
        rationale: &mut Vec<String>,
    ) -> Result<(), AiError> {
        let face = layout.face;
        for id in [standard::ANGLE_X, standard::ANGLE_Y, standard::ANGLE_Z] {
            self.b.parameter(&ParamId::new(id))?;
        }
        let head = self.new_deformer("HeadRotation");
        self.apply(Command::CreateRotationDeformer {
            id: Some(head.clone()),
            name: "Head rotation".to_owned(),
            children: layout.head.iter().cloned().map(ChildRef::Mesh).collect(),
            parts: Vec::new(),
            // あごを中心に傾ける。
            origin: Some(Vec2::new(face.center().x, face.max.y)),
            parent: None,
        })?;
        let target = KeyformTarget::Deformer(head.clone());
        self.add_keys(&target, standard::ANGLE_Z, &[-30.0, 30.0])?;
        for value in [-30.0, 30.0] {
            self.set_cell(&target, &[(standard::ANGLE_Z, value)], |base| match base {
                Keyform::Rotation(keyform) => Ok(Keyform::Rotation(RotationKeyform {
                    angle: value * HEAD_TILT,
                    ..keyform
                })),
                _ => Err(template(format!("{target} is not a rotation deformer"))),
            })?;
        }

        let cells = options.head_warp_cells.max(1);
        let warp = self.create_warp("FaceWarp", "Face warp", &layout.head, cells, cells)?;
        // 目・口のワープは顔のワープの中に作る (子の共通の親が顔のワープになる)。
        let (map, source) = match &options.turn {
            TurnDepth::Rules => (dome_depth(self.model(), face)?, "顔に内接する楕円体"),
            TurnDepth::Map(map) => (map.clone(), "推定した深度マップ"),
        };
        let labels: BTreeMap<MeshId, Label> =
            parts.iter().map(|(id, p)| (id.clone(), p.label)).collect();
        let mut plan = plan_turn(
            self.model(),
            &map,
            &labels,
            &TurnTarget {
                deformer: warp.clone(),
                role: Some(TurnRole::Head),
            },
        )?;
        plan.rotation.angle_scale = params.turn_angle_scale;
        let line = push_turn(&mut self.b, &plan)?;
        // キーの間の角度でも回転の道筋に沿うように曲線補間にする (`docs/spec/evaluation.md#曲線補間`)。
        self.apply(Command::SetKeyformInterpolation {
            target: KeyformTarget::Deformer(warp),
            blend_shape: None,
            interpolation: KeyformInterpolation::Smooth,
        })?;
        for id in [standard::ANGLE_X, standard::ANGLE_Z] {
            self.apply(Command::SetParameterMirror {
                id: ParamId::new(id),
                mirror: Some(ParamId::new(id)),
            })?;
        }
        rationale.push(format!(
            "頭: `AngleZ` ±30 で ±{:.0} 度傾け、`AngleX` / `AngleY` は深度 ({source}) からの回転で振り向かせる。{line}",
            30.0 * HEAD_TILT
        ));
        Ok(())
    }

    /// 目の開閉: 開いた目を包むワープを閉じる線へつぶし、閉じた目の絵があれば不透明度で入れ替える。
    fn rig_eye(
        &mut self,
        layout: &Layout,
        eye: &EyeParts,
        params: &RigParams,
        rationale: &mut Vec<String>,
    ) -> Result<(), AiError> {
        use eye_fade::*;
        let parameter = eye.parameter();
        let side = if eye.side == Side::Left { "L" } else { "R" };
        let rect = layout
            .union(&eye.open)
            .ok_or_else(|| template(format!("eye {side} has no bounds")))?;
        let h = rect.size().y;
        let closed = layout.union(&eye.closed);
        // 閉じる線: 閉じた目の絵があればその中心の高さ、なければ AI が決めた高さ。
        let line = match closed {
            Some(c) => c.center().y.clamp(rect.min.y, rect.max.y),
            None => rect.min.y + params.eye_close_line * h,
        };
        let bend = params.eye_close_bend;

        let warp = self.create_warp(
            &format!("Eye{side}Warp"),
            &format!("Eye {side} warp"),
            &eye.open,
            2,
            4,
        )?;
        let target = KeyformTarget::Deformer(warp);
        self.add_keys(&target, parameter, &[0.0, SQUASH_KEY])?;
        for (key, height) in [(0.0, CLOSED_HEIGHT), (SQUASH_KEY, SQUASH_HEIGHT)] {
            // つぶれるほど弧を強くする (開いた目では 0)。
            let weight = (1.0 - height) / (1.0 - CLOSED_HEIGHT);
            self.shape(&target, &[(parameter, key)], |p, n| {
                let y = line + (p.y - line) * height + weight * bend * h * arc(n.x);
                Vec2::new(0.0, y - p.y)
            })?;
        }

        if eye.closed.is_empty() {
            rationale.push(format!(
                "目 {side}: `{parameter}` で開いた目 ({} 枚) を高さ {:.2} の線へつぶす (弧 {:+.2})",
                eye.open.len(),
                params.eye_close_line,
                bend
            ));
            return Ok(());
        }
        for id in &eye.open {
            self.fade(
                id,
                parameter,
                &[(0.0, 0.0), (OPEN_OUT, 0.0), (CLOSED_FULL, 1.0), (1.0, 1.0)],
            )?;
        }
        for id in &eye.closed {
            self.fade(
                id,
                parameter,
                &[(0.0, 1.0), (CLOSED_FULL, 1.0), (CLOSED_IN, 0.0), (1.0, 0.0)],
            )?;
        }
        rationale.push(format!(
            "目 {side}: `{parameter}` で開いた目 ({} 枚) を閉じた目の絵の高さへつぶし (弧 {:+.2})、{CLOSED_IN}〜{OPEN_OUT} で閉じた目 ({} 枚) と入れ替える",
            eye.open.len(),
            bend,
            eye.closed.len()
        ));
        Ok(())
    }

    /// 口 (開いた口と閉じた口の絵): 開いた口は閉じた口の大きさの細い隙間から開き、閉じた口と入れ替える。
    fn rig_mouth_pair(
        &mut self,
        layout: &Layout,
        open: &[MeshId],
        closed: &[MeshId],
    ) -> Result<(), AiError> {
        use mouth_fade::*;
        let parameter = standard::MOUTH_OPEN_Y;
        let open_rect = layout
            .union(open)
            .ok_or_else(|| template("the open mouth has no bounds".into()))?;
        let closed_rect = layout
            .union(closed)
            .ok_or_else(|| template("the closed mouth has no bounds".into()))?;
        // 細い隙間: 閉じた口の横幅・高さの半分で、閉じた口の中心に置く。
        let slit = Vec2::new(
            closed_rect.size().x / open_rect.size().x.max(f32::EPSILON),
            0.5 * closed_rect.size().y / open_rect.size().y.max(f32::EPSILON),
        );
        for id in open {
            let target = KeyformTarget::Mesh(id.clone());
            self.add_keys(&target, parameter, &[OPEN_FULL, 1.0])?;
            // 基準のセル (開き 0) は最後に書く (`shape` は基準のセルからのずれで書くため)。
            for amount in [OPEN_FULL, 0.0] {
                self.shape(&target, &[(parameter, amount)], |p, _| {
                    closed_rect.center() + (p - open_rect.center()) * slit - p
                })?;
            }
            self.fade(id, parameter, &[(0.0, 0.0), (OPEN_FULL, 1.0), (1.0, 1.0)])?;
        }
        for id in closed {
            self.fade(
                id,
                parameter,
                &[(0.0, 1.0), (OPEN_FULL, 1.0), (CLOSED_OUT, 0.0)],
            )?;
        }
        Ok(())
    }

    /// 口 (1 枚): 口を包むワープの下側を下げて縦に開き、少しすぼめる。
    fn rig_mouth_single(
        &mut self,
        layout: &Layout,
        ids: &[MeshId],
        params: &RigParams,
    ) -> Result<(), AiError> {
        let rect = layout
            .union(ids)
            .ok_or_else(|| template("the mouth has no bounds".into()))?;
        let width = rect.size().x;
        let warp = self.create_warp("MouthWarp", "Mouth warp", ids, 2, 2)?;
        let target = KeyformTarget::Deformer(warp);
        self.add_keys(&target, standard::MOUTH_OPEN_Y, &[1.0])?;
        let open = params.mouth_open_height * width;
        self.shape(&target, &[(standard::MOUTH_OPEN_Y, 1.0)], |p, _| {
            let n = rect.normalize(p);
            Vec2::new(-(n.x - 0.5) * 0.1 * width, open * n.y.clamp(0.0, 1.0))
        })?;
        Ok(())
    }
}

/// テンプレートの内部の失敗 (座標変換が解けない、など)。
fn template(message: String) -> AiError {
    AiError::InvalidProposal(message)
}

/// [`propose_rig_with_ai`] の結果。
#[derive(Debug, Clone, PartialEq)]
pub struct AiRig {
    pub proposal: Proposal,
    /// 採用した値。
    pub params: RigParams,
    /// 捨てた答え (古い順)。
    pub rejected: Vec<RejectedAnswer>,
}

/// 検証を通らなかった AI の答えと、その理由 (次の問い合わせで AI に送り返す)。
#[derive(Debug, Clone, PartialEq)]
pub struct RejectedAnswer {
    pub answer: serde_json::Value,
    pub error: String,
}

/// [`propose_rig_with_ai`] の問い合わせの回数の既定の上限。
pub const DEFAULT_MAX_ATTEMPTS: usize = 3;

/// AI の答えの JSON Schema: [`RigParams`] の 4 つの数と、理由 (`reason`)。
///
/// 構造化出力は数値の範囲を書けないので、範囲は説明に書き、受け取った後に確かめる。
pub fn rig_output_schema() -> serde_json::Value {
    let mut properties = serde_json::Map::new();
    for (name, min, max) in RigParams::RANGES {
        properties.insert(
            name.to_owned(),
            json!({ "type": "number", "description": format!("From {min} to {max}.") }),
        );
    }
    properties.insert(
        "reason".into(),
        json!({ "type": "string", "description": "One or two sentences on why you chose these values." }),
    );
    let required: Vec<&str> = RigParams::RANGES
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
    turn_angle_scale: f32,
    eye_close_line: f32,
    eye_close_bend: f32,
    mouth_open_height: f32,
    reason: String,
}

/// [`rig_output_schema`] の形の答え (AI や外部のエージェントが決めた値) を読む。値と理由を返す。
///
/// 形が違えば [`AiError::InvalidProposal`]。範囲は確かめない ([`propose_rig`] が確かめる)。
pub fn params_from_answer(answer: &serde_json::Value) -> Result<(RigParams, String), AiError> {
    let answer: Answer = serde_json::from_value(answer.clone()).map_err(|e| {
        AiError::InvalidProposal(format!("the answer does not match the schema: {e}"))
    })?;
    let params = RigParams {
        turn_angle_scale: answer.turn_angle_scale,
        eye_close_line: answer.eye_close_line,
        eye_close_bend: answer.eye_close_bend,
        mouth_open_height: answer.mouth_open_height,
    };
    Ok((params, answer.reason))
}

fn system_prompt() -> String {
    let d = RigParams::default();
    format!(
        "You tune a 2D puppet rig for a layered character illustration. A fixed template already \
         decides the deformers and keyforms; you only choose the numbers it cannot infer from \
         layer labels, by looking at the illustration.\n\n\
         - turn_angle_scale ({t0} to {t1}, default {}): the head turns by AngleX/AngleY degrees \
           times this factor, projected with a depth map. Use smaller values for flat, stylized \
           faces with large eyes (the features slide too far when turned) and larger ones for \
           realistic, deep faces.\n\
         - eye_close_line ({l0} to {l1}, default {}): where the eye closes, as a height in the \
           open eye's bounding box (0 = top, 1 = bottom). The upper lid usually comes down to \
           meet the lower lid, so it is below the middle. Only used for eyes without a separate \
           closed-eye drawing.\n\
         - eye_close_bend ({b0} to {b1}, default {}): the curve of the closed eye as a fraction \
           of the eye height. Positive bends the middle down (a relaxed closed eye), negative \
           bends it up (a smiling closed eye).\n\
         - mouth_open_height ({m0} to {m1}, default {}): how far the lower lip drops when the \
           mouth opens, as a fraction of the mouth width. Only used when the mouth is a single \
           drawing without separate open and closed mouths.\n\n\
         Stay within the ranges. If your previous answer was rejected, fix the reported errors.",
        d.turn_angle_scale,
        d.eye_close_line,
        d.eye_close_bend,
        d.mouth_open_height,
        t0 = RigParams::RANGES[0].1,
        t1 = RigParams::RANGES[0].2,
        l0 = RigParams::RANGES[1].1,
        l1 = RigParams::RANGES[1].2,
        b0 = RigParams::RANGES[2].1,
        b1 = RigParams::RANGES[2].2,
        m0 = RigParams::RANGES[3].1,
        m1 = RigParams::RANGES[3].2,
    )
}

fn prompt(model: &Model, layout: &Layout, rejected: &[RejectedAnswer]) -> String {
    let canvas = Vec2::new(model.meta.canvas.width, model.meta.canvas.height);
    let bbox = |r: Rect| {
        let (min, size) = (r.min / canvas, r.size() / canvas);
        format!("{:.3}, {:.3}, {:.3}, {:.3}", min.x, min.y, size.x, size.y)
    };
    let mut s = String::from(
        "The image is the whole illustration. Bounding boxes are fractions of the image as \
         x, y, width, height.\n\n",
    );
    let _ = writeln!(s, "- face: {}", bbox(layout.face));
    for eye in &layout.eyes {
        let side = if eye.side == Side::Left {
            "left"
        } else {
            "right"
        };
        if let Some(r) = layout.union(&eye.open) {
            let _ = writeln!(
                s,
                "- character's {side} eye: {}; {}",
                bbox(r),
                if eye.closed.is_empty() {
                    "no closed-eye drawing"
                } else {
                    "has a closed-eye drawing"
                }
            );
        }
    }
    match &layout.mouth {
        Some(MouthParts::Pair { open, .. }) => {
            if let Some(r) = layout.union(open) {
                let _ = writeln!(s, "- mouth: {}; has open and closed drawings", bbox(r));
            }
        }
        Some(MouthParts::Single(ids)) => {
            if let Some(r) = layout.union(ids) {
                let _ = writeln!(s, "- mouth: {}; a single drawing", bbox(r));
            }
        }
        None => s.push_str("- no mouth\n"),
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

/// [`propose_rig_with_ai`] が送るリクエスト (`rejected` は前の答えと理由)。エディタは送る前に、
/// これを [`Provider::disclosure`] に渡して利用者に確認を取る。
pub fn rig_request(
    model: &Model,
    parts: &BTreeMap<MeshId, MeshPart>,
    image: &[u8],
    rejected: &[RejectedAnswer],
) -> Result<ProviderRequest, AiError> {
    let layout = Layout::new(model, parts)?;
    Ok(ProviderRequest {
        system: system_prompt(),
        prompt: prompt(model, &layout, rejected),
        images: vec![image.to_vec()],
    })
}

/// テンプレートでリグを組み、テンプレートで決められない値 ([`RigParams`]) を `provider` に決めさせる。
///
/// - `image` は原画の PNG (キャンバス全体。`crate::classify::overview_png` で PSD から作れる)。
/// - AI の答えが範囲の外か、組んだリグが適用・検証を通らなければ、答えとエラーを添えて問い直す。
///   `max_attempts` 回で通らなければ [`AiError::InvalidProposal`] を返す (呼び出し側は
///   [`propose_rig`] と既定値に戻せる)。
/// - 形式の合わない答え (JSON が読めない) も同じく問い直す。それ以外のエラー (送信の失敗など) は
///   そのまま返す。
pub fn propose_rig_with_ai(
    provider: &dyn Provider,
    model: &Model,
    parts: &BTreeMap<MeshId, MeshPart>,
    image: &[u8],
    options: &RigOptions,
    max_attempts: usize,
) -> Result<AiRig, AiError> {
    let layout = Layout::new(model, parts)?;
    let schema = rig_output_schema();
    let mut rejected: Vec<RejectedAnswer> = Vec::new();
    for _ in 0..max_attempts {
        let request = ProviderRequest {
            system: system_prompt(),
            prompt: prompt(model, &layout, &rejected),
            images: vec![image.to_vec()],
        };
        let value = provider.complete_json(&request, &schema)?;
        let (params, reason) = match params_from_answer(&value) {
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
        match build(model, parts, &layout, &params, options) {
            Ok(mut proposal) => {
                proposal.rationale = format!(
                    "AI が決めた値: 振り向き {:.2}、目の閉じる線 {:.2}、弧 {:+.2}、口の開き {:.2} ({})\n{}",
                    params.turn_angle_scale,
                    params.eye_close_line,
                    params.eye_close_bend,
                    params.mouth_open_height,
                    reason,
                    proposal.rationale
                );
                return Ok(AiRig {
                    proposal,
                    params,
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
        assert!(RigParams::default().check().is_empty());
        let bad = RigParams {
            turn_angle_scale: 2.5,
            eye_close_bend: f32::NAN,
            ..RigParams::default()
        };
        let errors = bad.check();
        assert_eq!(errors.len(), 2);
        assert!(errors[0].contains("turn_angle_scale is 2.5"), "{errors:?}");
        assert!(errors[1].contains("eye_close_bend"), "{errors:?}");
    }

    #[test]
    fn answers_are_read_without_checking_ranges() {
        let (params, reason) = params_from_answer(&json!({
            "turn_angle_scale": 2.5,
            "eye_close_line": 0.7,
            "eye_close_bend": -0.05,
            "mouth_open_height": 0.4,
            "reason": "why",
        }))
        .unwrap();
        assert_eq!(params.turn_angle_scale, 2.5);
        assert_eq!(params.eye_close_bend, -0.05);
        assert_eq!(reason, "why");
        assert_eq!(params.check().len(), 1);
        let err = params_from_answer(&json!({ "turn_angle_scale": 0.6 })).unwrap_err();
        assert!(
            matches!(&err, AiError::InvalidProposal(m) if m.contains("does not match the schema")),
            "{err}"
        );
    }

    #[test]
    fn the_schema_requires_every_value() {
        let schema = rig_output_schema();
        assert_eq!(schema["required"].as_array().unwrap().len(), 5);
        assert_eq!(schema["additionalProperties"], json!(false));
        assert_eq!(schema["properties"]["eye_close_line"]["type"], "number");
    }

    #[test]
    fn the_dome_peaks_at_the_face_center() {
        let mut model = ugoe_model::fixtures::single_quad();
        model.meta.canvas.width = 256.0;
        model.meta.canvas.height = 256.0;
        let face = Rect {
            min: Vec2::new(64.0, 64.0),
            max: Vec2::new(192.0, 192.0),
        };
        let map = dome_depth(&model, face).unwrap();
        assert!(map.sample(0.5, 0.5) > 0.99);
        assert!(map.sample(0.5, 0.3) < map.sample(0.5, 0.45));
        assert_eq!(map.sample(0.05, 0.05), 0.0);
    }
}
