//! 意味ラベルからの自動リギングの提案 (`docs/ai-integration.md#自動リギング`)。
//!
//! PSD を取り込んだ直後のモデル (メッシュだけでデフォーマのないもの) と、メッシュごとの意味ラベル
//! ([`mesh_parts`] でレイヤーの分類から作る) から、アバタープロファイルのレベル 1 のパラメータ
//! (`docs/spec/avatar-profile.md`) と呼吸 (`Breath`) を動かすリグと、髪揺れの物理、目パチ・呼吸・
//! リップシンクの設定を [`Command`] の列として提案する。
//!
//! - **テンプレート** ([`propose_rig`]): ラベルの組み合わせからデフォーマの構造とキーフォームの作り方を
//!   決める (決定的)。
//!   - 呼吸 (`Breath`): すべてのメッシュを包む体のワープを、上ほど大きく持ち上げる。
//!   - 顔の角度: 頭の回転デフォーマ (`AngleZ`) と、頭のメッシュを包むワープ (`AngleX` / `AngleY`)。
//!     ワープは深度からの回転で振り向かせる ([`crate::turn`])。深度は推定した深度マップか、
//!     なければ顔の外接矩形に内接する楕円体 (規則) を使い、どちらも意味ラベルで補正する。
//!   - 目の開閉 (`EyeLOpen` / `EyeROpen`): 開いた目のメッシュを包むワープを、閉じる線へ縦につぶす。
//!     閉じた目の絵 (`eye_closed`) があれば、つぶれた所で不透明度を入れ替える。
//!   - 口の開閉 (`MouthOpenY`): 開いた口と閉じた口の絵があれば、開いた口を閉じた口の大きさの細い
//!     隙間から開いて入れ替える。1 枚の口なら、口を包むワープの下側を下げて縦に開く。
//!   - 髪揺れ: 前髪・横髪・後ろ髪を包む揺れ用のワープに、頭の回転デフォーマに取り付けた振り子の
//!     動的物理 (`docs/spec/physics.md#動的物理-揺れ用パラメータのいらない物理`) の出力を足す。
//!   - エフェクト (`Model::effects`): 目パチ・呼吸・リップシンクの標準の設定を、作ったパラメータに向ける。
//! - **AI が決める値** ([`RigParams`]): テンプレートでは決められない値 (振り向きの量・目の閉じ方・
//!   口の開き方) を [`Provider`] に画像から決めさせる ([`propose_rig_with_ai`])。
//!   答えが範囲の外であったり、組んだリグが検証を通らなかったりしたら、エラーを添えて問い直す
//!   (回数の上限あり)。利用者の注文と前の答え (作り直し) を添えるなら [`propose_rig_with_ai_guided`]
//!   ([`RigGuidance`]、`docs/ai-integration.md#利用者の注文`)。4 つの値で反映できない注文は、
//!   答えの理由に `Not applied:` として示させ、[`AiRig::unapplied`] に取り出す。
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
use ugoe_model::effects::Effects;
use ugoe_model::mesh::MeshKeyform;
use ugoe_model::physics::{
    NormalizationRange, PhysicsBlend, PhysicsInput, PhysicsNormalization, PhysicsOutput,
    PhysicsParticle, PhysicsRig, PhysicsSource, PhysicsTarget, VertexOwner,
};
use ugoe_model::{
    DeformerId, DeformerKind, KeyformGrid, KeyformInterpolation, Mesh, MeshId, Model, ParamId,
    standard,
};
use ugoe_runtime::{CompiledModel, ModelInstance};

use crate::classify::{Label, LabelCategory, LayerLabel, Side};
use crate::depth::DepthMap;
use crate::grid;
use crate::instructions::{Instructions, split_unapplied, system_rules};
use crate::turn::{Builder, TurnRole, TurnTarget, plan_turn, push_turn};
use crate::{AiError, Proposal, Provider, ProviderRequest};

/// メッシュの意味ラベル (レイヤーの分類の結果をメッシュに対応付けたもの)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshPart {
    pub label: Label,
    /// キャラクター自身から見た左右。
    pub side: Side,
    /// AI が同じ部品の別の絵とみなしたレイヤーの組の番号 ([`LayerLabel::variant`])。
    pub variant: Option<u32>,
}

/// レイヤーの分類の結果 ([`crate::classify`]) を、名前でメッシュに対応付ける。
///
/// PSD の取り込み (`ugoe_editor_core::import_psd`) はメッシュの名前をレイヤーの名前にするので、
/// `layer_path` の最後の名前と同じ名前のメッシュにラベルを付ける。同じ名前のメッシュやレイヤーが
/// 複数あって決められないものは付けない。
pub fn mesh_parts(model: &Model, layers: &[LayerLabel]) -> BTreeMap<MeshId, MeshPart> {
    mesh_parts_where(model, layers, |_| true)
}

/// [`mesh_parts`] の、`keep` が真のメッシュだけを対応付ける版 (名前が同じかどうかも、その中で数える)。
pub(crate) fn mesh_parts_where(
    model: &Model,
    layers: &[LayerLabel],
    keep: impl Fn(&Mesh) -> bool,
) -> BTreeMap<MeshId, MeshPart> {
    let mut by_name: BTreeMap<&str, Option<MeshPart>> = BTreeMap::new();
    for layer in layers {
        if let Some(name) = layer.layer_path.last() {
            let part = MeshPart {
                label: layer.label,
                side: layer.side,
                variant: layer.variant,
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
    let meshes: Vec<&Mesh> = model.meshes.iter().filter(|m| keep(m)).collect();
    let mut parts = BTreeMap::new();
    for mesh in &meshes {
        let same_name = meshes.iter().filter(|m| m.name == mesh.name).count();
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
        range_errors(&Self::RANGES, &self.values())
    }
}

/// 範囲 `ranges` (名前・下限・上限) の外の値 `values` (同じ並び) のエラー (英語。AI に送り返す)。
/// [`RigParams::check`] と `crate::module_rig::ModuleRigParams::check` で共有する。
pub(crate) fn range_errors(ranges: &[(&str, f32, f32)], values: &[f32]) -> Vec<String> {
    ranges
        .iter()
        .zip(values)
        .filter(|((_, min, max), v)| !(v.is_finite() && *min <= **v && **v <= *max))
        .map(|((name, min, max), v)| {
            format!("{name} is {v}, but it must be between {min} and {max}")
        })
        .collect()
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

/// 呼吸 (`Breath` = 1) の持ち上げ。
mod breath {
    /// 絵の上端を持ち上げる量 (顔の高さに対する割合。下端は動かさない)。
    pub const LIFT: f32 = 0.025;
}

/// 髪揺れの振り子 (`crates/ugoe-cli/src/sample.rs` の見本の値を、顔の大きさに対する割合にしたもの)。
pub(crate) mod hair {
    use super::Label;

    #[derive(Debug, Clone, Copy)]
    pub struct Sway {
        /// 房の上から、揺れの重みが 0 のままの割合。そこから下端へ 2 乗で `max` まで増える。
        pub start: f32,
        pub max: f32,
        /// 振り子の長さ (根元から房の下端までに対する割合) と区間の数。
        pub length: f32,
        pub sections: usize,
        /// 粒子の係数 (`docs/spec/physics.md#係数の意味`)。
        pub mobility: f32,
        pub delay: f32,
        pub acceleration: f32,
        /// `AngleX` = ±30 での根元の平行移動 (顔の幅に対する割合)。
        pub shift: f32,
    }

    /// 前髪: 毛先だけを小さく揺らす (顔が読めるように)。
    const FRONT: Sway = Sway {
        start: 0.4,
        max: 0.5,
        length: 0.8,
        sections: 1,
        mobility: 0.9,
        delay: 0.8,
        acceleration: 1.5,
        shift: 0.04,
    };
    /// 横髪: 前髪より長く揺らす。
    const SIDE: Sway = Sway {
        start: 0.3,
        max: 0.7,
        length: 1.0,
        sections: 1,
        mobility: 0.92,
        delay: 0.7,
        acceleration: 1.5,
        shift: 0.08,
    };
    /// 後ろ髪: 2 区間で、大きくゆっくり揺らす。
    const BACK: Sway = Sway {
        start: 0.3,
        max: 0.8,
        length: 1.0,
        sections: 2,
        mobility: 0.93,
        delay: 0.6,
        acceleration: 1.5,
        shift: 0.09,
    };

    pub fn of(label: Label) -> Sway {
        match label {
            Label::HairFront => FRONT,
            Label::HairSide => SIDE,
            _ => BACK,
        }
    }
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
pub(crate) struct Rect {
    pub(crate) min: Vec2,
    pub(crate) max: Vec2,
}

impl Rect {
    pub(crate) fn of(points: &[Vec2]) -> Option<Self> {
        let first = *points.first()?;
        let (min, max) = points
            .iter()
            .fold((first, first), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
        Some(Self { min, max })
    }

    pub(crate) fn union(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    pub(crate) fn size(self) -> Vec2 {
        self.max - self.min
    }

    pub(crate) fn center(self) -> Vec2 {
        (self.min + self.max) * 0.5
    }

    /// 点 `p` の、矩形の中での位置 (左上 (0,0)〜右下 (1,1))。
    pub(crate) fn normalize(self, p: Vec2) -> Vec2 {
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

/// 揺らす髪の房 (同じラベルと左右のメッシュをまとめたもの)。
#[derive(Debug, Clone)]
struct HairGroup {
    label: Label,
    side: Side,
    meshes: Vec<MeshId>,
}

/// ラベルから決めた、リグの対象の部位と基準の姿の外接矩形。
#[derive(Debug, Clone)]
struct Layout {
    face: Rect,
    /// 呼吸で動かすメッシュ (背景・その他のラベルを除くすべて)。
    body: Vec<MeshId>,
    head: Vec<MeshId>,
    eyes: Vec<EyeParts>,
    mouth: Option<MouthParts>,
    hair: Vec<HairGroup>,
    /// メッシュの基準の姿の外接矩形 (キャンバス座標)。
    bounds: BTreeMap<MeshId, Rect>,
    /// 組めなかった部位 (提案の理由に書く)。
    skipped: Vec<String>,
}

impl Layout {
    fn new(model: &Model, parts: &BTreeMap<MeshId, MeshPart>) -> Result<Self, AiError> {
        let bounds = mesh_bounds(model)?;
        let labeled = Labeled::new(model, parts, &bounds);
        let face = union_of(&bounds, &labeled.with(|l| l == Label::Face)).ok_or_else(|| {
            template(
                RigTemplate::Character,
                "no mesh is labeled `face`; cannot place the head".into(),
            )
        })?;
        let head = labeled.with(is_head);
        let body = body_meshes(model, parts, &bounds);
        let mut skipped = Vec::new();
        let eyes = eye_parts(model, &labeled, &bounds, face, &mut skipped);
        let mouth = mouth_parts(&labeled, &mut skipped);
        let hair = hair_groups(&labeled, &mut skipped);
        Ok(Self {
            face,
            body,
            head,
            eyes,
            mouth,
            hair,
            bounds,
            skipped,
        })
    }

    fn union(&self, ids: &[MeshId]) -> Option<Rect> {
        union_of(&self.bounds, ids)
    }
}

/// `ids` のメッシュの外接矩形を合わせたもの (外接矩形のないメッシュは飛ばす)。
fn union_of(bounds: &BTreeMap<MeshId, Rect>, ids: &[MeshId]) -> Option<Rect> {
    ids.iter()
        .filter_map(|id| bounds.get(id).copied())
        .reduce(Rect::union)
}

/// ラベルの付いた (マスク用でなく、外接矩形のある) メッシュ。取り込みの順 (描画順) に並べる。
struct Labeled<'a> {
    meshes: Vec<(&'a MeshId, MeshPart)>,
}

impl<'a> Labeled<'a> {
    fn new(
        model: &'a Model,
        parts: &BTreeMap<MeshId, MeshPart>,
        bounds: &BTreeMap<MeshId, Rect>,
    ) -> Self {
        let meshes = model
            .meshes
            .iter()
            .filter(|m| !m.mask_only && bounds.contains_key(&m.id))
            .filter_map(|m| parts.get(&m.id).map(|p| (&m.id, *p)))
            .collect();
        Self { meshes }
    }

    /// ラベルが `pred` に当てはまるメッシュ。
    fn with(&self, pred: impl Fn(Label) -> bool) -> Vec<MeshId> {
        self.meshes
            .iter()
            .filter(|(_, p)| pred(p.label))
            .map(|(id, _)| (*id).clone())
            .collect()
    }
}

/// 呼吸で動かすメッシュ (背景・その他のラベルを除くすべて)。
fn body_meshes(
    model: &Model,
    parts: &BTreeMap<MeshId, MeshPart>,
    bounds: &BTreeMap<MeshId, Rect>,
) -> Vec<MeshId> {
    // 呼吸の体のワープはマスク用のメッシュも含める (マスクされるメッシュと一緒に動かす)。
    model
        .meshes
        .iter()
        .filter(|m| bounds.contains_key(&m.id))
        .filter(|m| {
            parts
                .get(&m.id)
                .is_none_or(|p| p.label.category() != LabelCategory::Other)
        })
        .map(|m| m.id.clone())
        .collect()
}

/// 左右の目の部位。開いた目の部位のない目は除き、`skipped` に書く。
fn eye_parts(
    model: &Model,
    labeled: &Labeled<'_>,
    bounds: &BTreeMap<MeshId, Rect>,
    face: Rect,
    skipped: &mut Vec<String>,
) -> Vec<EyeParts> {
    // 目: 左右はラベルの左右、中央のものは顔の中心との位置で決める (画像の右がキャラクターの左)。
    let mut eyes: Vec<EyeParts> = [Side::Left, Side::Right]
        .into_iter()
        .map(|side| EyeParts {
            side,
            open: Vec::new(),
            closed: Vec::new(),
        })
        .collect();
    for (id, part) in &labeled.meshes {
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
                        "{} (`{}`) spans both eyes, so it does not open and close",
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
                "no {} eye (open eye parts), so `{}` is not made",
                if eye.side == Side::Left {
                    "left"
                } else {
                    "right"
                },
                eye.parameter()
            ));
        }
        !eye.open.is_empty()
    });
    eyes
}

/// 口の部位。開いた口と閉じた口の絵がそろっていれば組、なければ口のラベルのものすべて。
fn mouth_parts(labeled: &Labeled<'_>, skipped: &mut Vec<String>) -> Option<MouthParts> {
    let open = labeled.with(|l| {
        matches!(
            l,
            Label::MouthOpen | Label::MouthInner | Label::Teeth | Label::Tongue
        )
    });
    let closed = labeled.with(|l| l == Label::MouthClosed);
    if labeled.with(|l| l == Label::MouthOpen).is_empty() || closed.is_empty() {
        let all = labeled.with(|l| l.category() == LabelCategory::Mouth);
        if all.is_empty() {
            skipped.push("no mouth, so `MouthOpenY` is not made".into());
            None
        } else {
            Some(MouthParts::Single(all))
        }
    } else {
        Some(MouthParts::Pair { open, closed })
    }
}

/// 髪: ラベルと左右ごとに 1 つの房にまとめる (前髪・横髪・後ろ髪の順)。
fn hair_groups(labeled: &Labeled<'_>, skipped: &mut Vec<String>) -> Vec<HairGroup> {
    let mut hair: Vec<HairGroup> = Vec::new();
    for label in [Label::HairFront, Label::HairSide, Label::HairBack] {
        for (id, part) in labeled.meshes.iter().filter(|(_, p)| p.label == label) {
            match hair
                .iter_mut()
                .find(|g| g.label == label && g.side == part.side)
            {
                Some(group) => group.meshes.push((*id).clone()),
                None => hair.push(HairGroup {
                    label,
                    side: part.side,
                    meshes: vec![(*id).clone()],
                }),
            }
        }
    }
    if hair.is_empty() {
        skipped.push("no meshes are labeled as hair, so there is no hair sway physics".into());
    }
    hair
}

/// メッシュの基準の姿 (すべてのパラメータが既定値) の外接矩形 (キャンバス座標)。
pub(crate) fn mesh_bounds(model: &Model) -> Result<BTreeMap<MeshId, Rect>, AiError> {
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
    Ok(bounds)
}

pub(crate) fn mesh_name<'a>(model: &'a Model, id: &'a MeshId) -> &'a str {
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
        template: RigTemplate::Character,
    };
    let mut rationale = Vec::new();
    // 体のワープを先に作り、頭の回転デフォーマをその中に入れる (呼吸で頭も持ち上がる)。
    rig.rig_breath(layout, &mut rationale)?;
    let (head, face_warp) = rig.rig_head(layout, parts, params, options, &mut rationale)?;
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
                "Mouth: opens the open mouth ({} meshes) from a thin slit the size of the closed mouth, and at `MouthOpenY` {} it has replaced the closed mouth ({} meshes)",
                open.len(),
                mouth_fade::OPEN_FULL,
                closed.len()
            ));
        }
        Some(MouthParts::Single(ids)) => {
            rig.rig_mouth_single(layout, ids, params)?;
            rationale.push(format!(
                "Mouth: no open and closed drawings, so opens the mouth ({} meshes) by lowering the bottom of its warp by {:.2} × the mouth width",
                ids.len(),
                params.mouth_open_height
            ));
        }
        None => {}
    }
    rig.rig_hair(layout, &head, &face_warp, &mut rationale)?;
    rig.set_effects(&mut rationale)?;
    rationale.extend(layout.skipped.iter().map(|s| format!("Not rigged: {s}")));
    rig.b.validate()?;
    Ok(Proposal {
        title: "Auto rig (face angles, eyes, mouth, breath, hair sway)".into(),
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
pub(crate) struct Rig {
    pub(crate) b: Builder,
    /// 組んでいるテンプレート (組めないときのエラー [`AiError::Template`] に書く)。
    pub(crate) template: RigTemplate,
}

impl Rig {
    pub(crate) fn model(&self) -> &Model {
        self.b.doc.model()
    }

    pub(crate) fn apply(&mut self, command: Command) -> Result<(), AiError> {
        self.b.apply(command)
    }

    fn new_deformer(&self, base: &str) -> DeformerId {
        new_deformer_id(self.model(), base)
    }

    pub(crate) fn create_warp(
        &mut self,
        base: &str,
        name: &str,
        children: &[MeshId],
        rows: u16,
        cols: u16,
        parent: Option<&DeformerId>,
    ) -> Result<DeformerId, AiError> {
        let id = self.new_deformer(base);
        self.create_warp_as(id, name, children, rows, cols, parent)
    }

    /// [`Rig::create_warp`] の、ID を決めて作る版。
    pub(crate) fn create_warp_as(
        &mut self,
        id: DeformerId,
        name: &str,
        children: &[MeshId],
        rows: u16,
        cols: u16,
        parent: Option<&DeformerId>,
    ) -> Result<DeformerId, AiError> {
        self.apply(Command::CreateWarpDeformer {
            id: Some(id.clone()),
            name: name.to_owned(),
            children: children.iter().cloned().map(ChildRef::Mesh).collect(),
            parts: Vec::new(),
            rows,
            cols,
            parent: parent.cloned(),
        })?;
        Ok(id)
    }

    /// 要素の格子のセル `cell` (書かない軸は既定値) の点 (親の座標系)・キーフォームと、すべての軸を書いたセル。
    pub(crate) fn keyform_at(
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
            kind: RigTemplate,
        ) -> Result<(T, KeyformCell), AiError> {
            let (index, cell) =
                grid::cell_index(&grid.bindings, |p| values.get(p).copied().unwrap_or(0.0))
                    .map_err(|(parameter, value)| {
                        template(kind, format!("axis {parameter} has no key {value}"))
                    })?;
            let keyform = grid
                .keyforms
                .get(index)
                .cloned()
                .ok_or_else(|| template(kind, format!("cell {index} is missing")))?;
            Ok((keyform, cell))
        }
        match target {
            KeyformTarget::Mesh(id) => {
                let mesh = model
                    .meshes
                    .iter()
                    .find(|m| m.id == *id)
                    .ok_or_else(|| template(self.template, format!("no mesh `{id}`")))?;
                let (keyform, cell) = find(&mesh.keyforms, &values, self.template)?;
                Ok((keyform.positions.clone(), Keyform::Mesh(keyform), cell))
            }
            KeyformTarget::Deformer(id) => {
                let deformer = model
                    .deformers
                    .iter()
                    .find(|d| d.id == *id)
                    .ok_or_else(|| template(self.template, format!("no deformer `{id}`")))?;
                match &deformer.kind {
                    DeformerKind::Warp(warp) => {
                        let (keyform, cell) = find(&warp.keyforms, &values, self.template)?;
                        Ok((keyform.points.clone(), Keyform::Warp(keyform), cell))
                    }
                    DeformerKind::Rotation(rotation) => {
                        let (keyform, cell) = find(&rotation.keyforms, &values, self.template)?;
                        Ok((vec![keyform.origin], Keyform::Rotation(keyform), cell))
                    }
                    DeformerKind::Unknown => Err(template(
                        self.template,
                        format!("deformer `{id}` has an unknown kind"),
                    )),
                }
            }
            _ => Err(template(
                self.template,
                format!("{target} is not supported"),
            )),
        }
    }

    /// 要素の格子の `parameter` の軸にキー `keys` を加える (既定値のキーを先に。あるキーは飛ばす)。
    pub(crate) fn add_keys(
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
            if grid::has_key(grid::axis_keys(self.model(), target, &id), key) {
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

    /// 格子のセル `cell` (書かない軸は既定値) を、そのセルの今のキーフォームを `edit` で変えたものにする。
    pub(crate) fn set_cell(
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
            .map_err(|e| template(self.template, format!("{target}: {e}")))?;
        let rect = Rect::of(&canvas)
            .ok_or_else(|| template(self.template, format!("{target} has no points")))?;
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
            .map_err(|e| template(self.template, format!("{target}: {e}")))?;
        let kind = self.template;
        self.set_cell(target, cell, |base| match base {
            Keyform::Mesh(keyform) => Ok(Keyform::Mesh(MeshKeyform {
                positions: points,
                ..keyform
            })),
            Keyform::Warp(keyform) => Ok(Keyform::Warp(WarpKeyform { points, ..keyform })),
            _ => Err(template(
                kind,
                format!("{target} is not a mesh or a warp deformer"),
            )),
        })
    }

    /// メッシュ `id` の不透明度を、`parameter` のキーごとに `(キー, 不透明度)` の値にする
    /// (他の軸は既定値のセル。形は変えない)。
    fn fade(&mut self, id: &MeshId, parameter: &str, keys: &[(f32, f32)]) -> Result<(), AiError> {
        let target = KeyformTarget::Mesh(id.clone());
        let values: Vec<f32> = keys.iter().map(|(key, _)| *key).collect();
        self.add_keys(&target, parameter, &values)?;
        let kind = self.template;
        for (key, opacity) in keys {
            self.set_cell(&target, &[(parameter, *key)], |base| match base {
                Keyform::Mesh(keyform) => Ok(Keyform::Mesh(MeshKeyform {
                    opacity: *opacity,
                    ..keyform
                })),
                _ => Err(template(kind, format!("{target} is not a mesh"))),
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
    ) -> Result<(DeformerId, DeformerId), AiError> {
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
        let kind = self.template;
        for value in [-30.0, 30.0] {
            self.set_cell(&target, &[(standard::ANGLE_Z, value)], |base| match base {
                Keyform::Rotation(keyform) => Ok(Keyform::Rotation(RotationKeyform {
                    angle: value * HEAD_TILT,
                    ..keyform
                })),
                _ => Err(template(
                    kind,
                    format!("{target} is not a rotation deformer"),
                )),
            })?;
        }

        let cells = options.head_warp_cells.max(1);
        // 親 (頭の回転) を明示して、提案の一部だけを採用するときに頭の回転を外すと顔のワープも外れるようにする
        // (体のワープの直下では深度からの回転を推定できない)。
        let warp = self.create_warp(
            "FaceWarp",
            "Face warp",
            &layout.head,
            cells,
            cells,
            Some(&head),
        )?;
        // 目・口のワープは顔のワープの中に作る (子の共通の親が顔のワープになる)。
        let (map, source) = match &options.turn {
            TurnDepth::Rules => (
                dome_depth(self.model(), face)?,
                "an ellipsoid inscribed in the face",
            ),
            TurnDepth::Map(map) => (map.clone(), "the estimated depth map"),
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
            target: KeyformTarget::Deformer(warp.clone()),
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
            "Head: tilts ±{:.0}° at `AngleZ` ±30, and turns with `AngleX` / `AngleY` by rotating the depth ({source}). {line}",
            30.0 * HEAD_TILT
        ));
        Ok((head, warp))
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
            .ok_or_else(|| template(self.template, format!("eye {side} has no bounds")))?;
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
            None,
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
                "Eye {side}: `{parameter}` squashes the open eye ({} meshes) onto a line at height {:.2} (bend {:+.2})",
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
            "Eye {side}: `{parameter}` squashes the open eye ({} meshes) to the height of the closed eye drawing (bend {:+.2}), and swaps it with the closed eye ({} meshes) between {CLOSED_IN} and {OPEN_OUT}",
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
            .ok_or_else(|| template(self.template, "the open mouth has no bounds".into()))?;
        let closed_rect = layout
            .union(closed)
            .ok_or_else(|| template(self.template, "the closed mouth has no bounds".into()))?;
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
            .ok_or_else(|| template(self.template, "the mouth has no bounds".into()))?;
        let width = rect.size().x;
        let warp = self.create_warp("MouthWarp", "Mouth warp", ids, 2, 2, None)?;
        let target = KeyformTarget::Deformer(warp);
        self.add_keys(&target, standard::MOUTH_OPEN_Y, &[1.0])?;
        let open = params.mouth_open_height * width;
        self.shape(&target, &[(standard::MOUTH_OPEN_Y, 1.0)], |p, _| {
            let n = rect.normalize(p);
            Vec2::new(-(n.x - 0.5) * 0.1 * width, open * n.y.clamp(0.0, 1.0))
        })?;
        Ok(())
    }

    /// 要素の基準の姿の点 (キャンバス座標)。
    pub(crate) fn canvas_points(&self, target: &KeyformTarget) -> Result<Vec<Vec2>, AiError> {
        let (local, ..) = self.keyform_at(target, &[])?;
        local_to_canvas(self.model(), target, &ParameterValues::new(), &local)
            .map_err(|e| template(self.template, format!("{target}: {e}")))
    }

    /// 呼吸: すべてのメッシュを包む体のワープを、`Breath` = 1 で上 (頭の側) ほど大きく持ち上げる。
    fn rig_breath(&mut self, layout: &Layout, rationale: &mut Vec<String>) -> Result<(), AiError> {
        let Some(rect) = layout.union(&layout.body) else {
            return Ok(());
        };
        self.b.parameter(&ParamId::new(standard::BREATH))?;
        let warp = self.create_warp("BodyWarp", "Body warp", &layout.body, 4, 4, None)?;
        let target = KeyformTarget::Deformer(warp);
        self.add_keys(&target, standard::BREATH, &[1.0])?;
        let lift = breath::LIFT * layout.face.size().y;
        self.shape(&target, &[(standard::BREATH, 1.0)], |p, _| {
            // 絵の下端は動かさず、上ほど大きく持ち上げる。
            let t = ((rect.max.y - p.y) / rect.size().y.max(f32::EPSILON)).clamp(0.0, 1.0);
            Vec2::new(0.0, -lift * t)
        })?;
        rationale.push(format!(
            "Breath: `Breath` = 1 lifts the body warp around all meshes, more toward the top (up to {lift:.1} px, {:.1}% of the face height)",
            breath::LIFT * 100.0
        ));
        Ok(())
    }

    /// 髪揺れ: 房ごとに揺れ用のワープを作り、頭の回転デフォーマ `head` に取り付けた振り子の先の横の
    /// 変位を、ワープの制御点に重み付きで足す (根元側の重み 0・毛先 1。生え際は動かない)。
    ///
    /// 入力は `AngleX` (頭を横に向ける動きはデフォーマを動かさないので、根元の平行移動で与える)。
    /// `AngleZ`・呼吸は頭のデフォーマを動かすので、それだけで根元が動く (`crates/ugoe-cli/src/sample.rs` の見本と同じ方法)。
    fn rig_hair(
        &mut self,
        layout: &Layout,
        head: &DeformerId,
        face_warp: &DeformerId,
        rationale: &mut Vec<String>,
    ) -> Result<(), AiError> {
        let face = layout.face;
        for group in &layout.hair {
            let rect = layout
                .union(&group.meshes)
                .ok_or_else(|| template(self.template, "the hair has no bounds".into()))?;
            let (kind, side) = (
                match group.label {
                    Label::HairFront => "Front",
                    Label::HairSide => "Side",
                    _ => "Back",
                },
                match group.side {
                    Side::Left => "L",
                    Side::Right => "R",
                    Side::Center => "",
                },
            );
            let sway = hair::of(group.label);
            let warp = self.create_warp(
                &format!("Hair{kind}{side}SwayWarp"),
                &format!("Hair {kind} {side} sway warp").replace("  ", " "),
                &group.meshes,
                3,
                3,
                Some(face_warp),
            )?;
            let target = KeyformTarget::Deformer(warp.clone());
            let weights = self
                .canvas_points(&target)?
                .iter()
                .map(|p| {
                    let t =
                        ((rect.normalize(*p).y - sway.start) / (1.0 - sway.start)).clamp(0.0, 1.0);
                    sway.max * t * t
                })
                .collect();
            // 根元: 前髪・横髪は房の上端。後ろ髪は顔の中ほどの高さ (頭頂に置くと、あごを中心に傾けた
            // ときに根元が大きく動きすぎる)。
            let top = if group.label == Label::HairBack {
                face.center().y.clamp(rect.min.y, rect.max.y)
            } else {
                rect.min.y
            };
            let root = Vec2::new(rect.center().x, top);
            let length = (rect.max.y - top) * sway.length;
            let lengths = vec![length / sway.sections as f32; sway.sections];
            let shift = sway.shift * face.size().x;
            let rig = self.pendulum(
                &format!("Hair{kind}{side}"),
                format!("Hair {kind} {side}").trim_end(),
                head,
                &KeyformTarget::Deformer(face_warp.clone()),
                standard::ANGLE_X,
                root,
                &lengths,
                sway,
                shift,
                VertexOwner::Warp(warp),
                weights,
            )?;
            self.apply(Command::AddPhysicsRig { rig })?;
            rationale.push(format!(
                "Hair sway: {} ({} meshes) sways with a {}-section pendulum ({length:.0} px long) attached to the head (`AngleX` ±30 moves the root by {shift:.0} px)",
                group.label,
                group.meshes.len(),
                sway.sections,
            ));
        }
        Ok(())
    }

    /// 真下に垂れる振り子 (根元 `root` はキャンバス座標) の動的物理のリグ。根元は `frame` に取り付け、
    /// 先の粒子の横の変位を `owner` の頂点に `weights` の重みで足す。`space` は親が `frame` の要素
    /// (根元の位置をその親の座標系で書く)。入力は `input` の値で根元を横に `±shift` だけ動かす。
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn pendulum(
        &self,
        id: &str,
        name: &str,
        frame: &DeformerId,
        space: &KeyformTarget,
        input: &str,
        root: Vec2,
        lengths: &[f32],
        sway: hair::Sway,
        shift: f32,
        owner: VertexOwner,
        weights: Vec<f32>,
    ) -> Result<PhysicsRig, AiError> {
        // 根元の位置は `frame` の局所座標で書く (リグの髪なら頭の回転デフォーマ。その子の顔のワープの親の座標系)。
        let local = canvas_to_local(
            self.model(),
            space,
            &ParameterValues::new(),
            &[root],
            &[root],
        )
        .map_err(|e| template(self.template, format!("the root of {id}: {e}")))?[0];
        let mut particles = vec![PhysicsParticle {
            position: local,
            mobility: 1.0,
            delay: 1.0,
            acceleration: 1.0,
            radius: 0.0,
            collision_radius: 0.0,
        }];
        let mut y = 0.0;
        for length in lengths {
            y += length;
            particles.push(PhysicsParticle {
                position: Vec2::new(0.0, y),
                mobility: sway.mobility,
                delay: sway.delay,
                acceleration: sway.acceleration,
                radius: *length,
                collision_radius: 0.0,
            });
        }
        Ok(PhysicsRig {
            id: id.into(),
            name: name.to_owned(),
            inputs: vec![PhysicsInput {
                parameter: ParamId::new(input),
                source: PhysicsSource::X,
                weight: 100.0,
                reflect: false,
            }],
            outputs: vec![PhysicsOutput {
                target: PhysicsTarget::Vertices { owner, weights },
                particle: lengths.len() as u32,
                source: PhysicsSource::X,
                scale: 1.0,
                weight: 100.0,
                reflect: false,
                blend: PhysicsBlend::Overwrite,
            }],
            particles,
            normalization: PhysicsNormalization {
                position: NormalizationRange {
                    min: -shift,
                    default: 0.0,
                    max: shift,
                },
                angle: NormalizationRange {
                    min: -10.0,
                    default: 0.0,
                    max: 10.0,
                },
            },
            colliders: Vec::new(),
            frame: Some(frame.clone()),
        })
    }

    /// エフェクト: 目パチ・呼吸・リップシンクの標準の設定 (`Effects::standard`) のうち、モデルにある
    /// パラメータを対象にするものを足す (すでに対象のパラメータは、その設定を残す)。
    fn set_effects(&mut self, rationale: &mut Vec<String>) -> Result<(), AiError> {
        let model = self.model();
        let has = |id: &ParamId| model.parameters.iter().any(|p| &p.id == id);
        let mut effects = model.effects.clone();
        let standard = Effects::standard();
        for id in standard.eye_blink.parameters {
            if has(&id) && !effects.eye_blink.parameters.contains(&id) {
                effects.eye_blink.parameters.push(id);
            }
        }
        for b in standard.breath.parameters {
            if has(&b.parameter)
                && !effects
                    .breath
                    .parameters
                    .iter()
                    .any(|x| x.parameter == b.parameter)
            {
                effects.breath.parameters.push(b);
            }
        }
        for id in standard.lip_sync.parameters {
            if has(&id) && !effects.lip_sync.parameters.contains(&id) {
                effects.lip_sync.parameters.push(id);
            }
        }
        if effects == model.effects {
            return Ok(());
        }
        let list = |ids: Vec<&ParamId>| {
            ids.iter()
                .map(|id| format!("`{id}`"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        rationale.push(format!(
            "Effects: eye blink ({}); breath ({}); lip sync ({})",
            list(effects.eye_blink.parameters.iter().collect()),
            list(
                effects
                    .breath
                    .parameters
                    .iter()
                    .map(|b| &b.parameter)
                    .collect()
            ),
            list(effects.lip_sync.parameters.iter().collect()),
        ));
        self.apply(Command::SetEffects { effects })
    }
}

/// 自動リギングのテンプレート ([`AiError::Template`] で、どれが組めなかったかを示す)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RigTemplate {
    /// 1 枚絵のキャラクターのテンプレート ([`propose_rig`])。
    Character,
    /// 着せ替えのモジュールのテンプレート ([`crate::module_rig::propose_module_rig`])。
    Module,
}

impl RigTemplate {
    /// 識別子 (`character`・`module`)。エラーの文言とエディタの文言の引数に使う。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Character => "character",
            Self::Module => "module",
        }
    }
}

impl std::fmt::Display for RigTemplate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// テンプレートがモデルを組めない失敗 (足りないもの・座標変換が解けない、など)。AI の値を変えても直らないので、
/// 問い直さない [`AiError::Template`] にする ([`ask_until_valid`])。
pub(crate) fn template(template: RigTemplate, reason: String) -> AiError {
    AiError::Template { template, reason }
}

/// [`propose_rig_with_ai`] の結果。
#[derive(Debug, Clone, PartialEq)]
pub struct AiRig {
    pub proposal: Proposal,
    /// 採用した値。
    pub params: RigParams,
    /// 採用した答えの理由 (反映できなかった注文を除いた本文)。
    pub reason: String,
    /// 4 つの値では反映できなかった注文 (答えの理由の `Not applied:` の部分。AI の言葉のまま)。
    pub unapplied: Vec<String>,
    /// 捨てた答え (古い順)。
    pub rejected: Vec<RejectedAnswer>,
}

/// 作り直しで AI に示す、前の答え。
#[derive(Debug, Clone, PartialEq)]
pub struct PreviousRig {
    pub params: RigParams,
    /// 前の答えの理由。
    pub reason: String,
}

/// リグの値を決めるときに添える、利用者の注文と前の答え (作り直し)。既定 (どちらもない) なら
/// [`propose_rig_with_ai`] と同じ。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RigGuidance {
    pub instructions: Option<Instructions>,
    pub previous: Option<PreviousRig>,
    /// 同じ注文でレイヤーを分類し直した (ラベルに関わる注文は分類で反映した)。偽なら、ラベルの付け替えを
    /// 求める注文も「反映できない」として示させる (この段階ではラベルを変えられないため)。
    pub labels_follow_instructions: bool,
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
    values_schema(&RigParams::RANGES)
}

/// 範囲 `ranges` の数と理由 (`reason`) を答えさせる JSON Schema (すべて必須)。
pub(crate) fn values_schema(ranges: &[(&str, f32, f32)]) -> serde_json::Value {
    let mut properties = serde_json::Map::new();
    for (name, min, max) in ranges {
        properties.insert(
            (*name).to_owned(),
            json!({ "type": "number", "description": format!("From {min} to {max}.") }),
        );
    }
    properties.insert(
        "reason".into(),
        json!({ "type": "string", "description": "One or two sentences on why you chose these values." }),
    );
    let required: Vec<&str> = ranges
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

/// 注文か前の答えがあるときにシステムプロンプトに足す規則。
fn guidance_rules(guidance: &RigGuidance) -> String {
    revision_rules(
        guidance.previous.is_some(),
        guidance.instructions.is_some(),
        guidance.labels_follow_instructions,
        "the four values above",
        "other parameters, a different rig structure, physics, motions, or changes to the drawing",
    )
}

/// 注文か前の答えがあるときにシステムプロンプトに足す規則 (リグとモジュールのリグで共有する)。`values` は値の呼び名、
/// `examples` は反映できない注文の例。
pub(crate) fn revision_rules(
    previous: bool,
    instructions: bool,
    labels_follow_instructions: bool,
    values: &str,
    examples: &str,
) -> String {
    let mut s = String::new();
    if previous {
        s.push_str(
            "\n\nThis is a revision. Your previous answer is given in the prompt. Start from its \
             values and change only what the user's instructions call for.",
        );
    }
    if instructions {
        let labels = if labels_follow_instructions {
            "Parts about layer labels were already applied when the layers were classified; do \
             not list them."
        } else {
            "Layer labels cannot change in this step, so list requests to relabel layers as not \
             applied too (the user can classify the layers again)."
        };
        s.push_str(&system_rules(&format!(
            "reflect them only through {values}. For every part you cannot reflect \
             that way (for example {examples}), do not pretend: end `reason` with one line per such part, \
             starting with \"{}\" and saying what was asked and why it cannot be done. {labels}",
            crate::instructions::NOT_APPLIED
        )));
    }
    s
}

/// 捨てた答えとエラーをプロンプトに書く (問い直し)。
pub(crate) fn push_rejected(s: &mut String, rejected: &[RejectedAnswer]) {
    for (i, r) in rejected.iter().enumerate() {
        let _ = write!(
            s,
            "\nAttempt {} was rejected.\nAnswer: {}\nErrors: {}\n",
            i + 1,
            r.answer,
            r.error
        );
    }
}

fn prompt(
    model: &Model,
    layout: &Layout,
    rejected: &[RejectedAnswer],
    guidance: &RigGuidance,
) -> String {
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
    if let Some(previous) = &guidance.previous {
        let p = previous.params;
        let _ = writeln!(
            s,
            "\nYour previous answer: {}",
            json!({
                "turn_angle_scale": p.turn_angle_scale,
                "eye_close_line": p.eye_close_line,
                "eye_close_bend": p.eye_close_bend,
                "mouth_open_height": p.mouth_open_height,
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

/// [`propose_rig_with_ai`] が送るリクエスト (`rejected` は前の答えと理由)。エディタは送る前に、
/// これを [`Provider::disclosure`] に渡して利用者に確認を取る。
pub fn rig_request(
    model: &Model,
    parts: &BTreeMap<MeshId, MeshPart>,
    image: &[u8],
    rejected: &[RejectedAnswer],
) -> Result<ProviderRequest, AiError> {
    rig_request_guided(model, parts, image, rejected, &RigGuidance::default())
}

/// [`rig_request`] の、注文と前の答えを添える版 ([`propose_rig_with_ai_guided`] が送るもの)。
pub fn rig_request_guided(
    model: &Model,
    parts: &BTreeMap<MeshId, MeshPart>,
    image: &[u8],
    rejected: &[RejectedAnswer],
    guidance: &RigGuidance,
) -> Result<ProviderRequest, AiError> {
    let layout = Layout::new(model, parts)?;
    Ok(request(model, &layout, image, rejected, guidance))
}

fn request(
    model: &Model,
    layout: &Layout,
    image: &[u8],
    rejected: &[RejectedAnswer],
    guidance: &RigGuidance,
) -> ProviderRequest {
    ProviderRequest {
        system: system_prompt() + &guidance_rules(guidance),
        prompt: prompt(model, layout, rejected, guidance),
        images: vec![image.to_vec()],
    }
}

/// テンプレートでリグを組み、テンプレートで決められない値 ([`RigParams`]) を `provider` に決めさせる。
///
/// - `image` は原画の PNG (キャンバス全体。`crate::classify::overview_png` で PSD から作れる)。
/// - AI の答えが範囲の外か、組んだリグが適用・検証を通らなければ、答えとエラーを添えて問い直す。
///   `max_attempts` 回で通らなければ [`AiError::InvalidProposal`] を返す (呼び出し側は
///   [`propose_rig`] と既定値に戻せる)。
/// - 形式の合わない答え (JSON が読めない) も同じく問い直す。それ以外のエラー (送信の失敗・値を変えても
///   直らないテンプレートの失敗 ([`AiError::Template`]) など) はそのまま返す。
/// - `max_attempts` が 0 なら、問い合わせずに [`AiError::InvalidRequest`] を返す。
pub fn propose_rig_with_ai(
    provider: &dyn Provider,
    model: &Model,
    parts: &BTreeMap<MeshId, MeshPart>,
    image: &[u8],
    options: &RigOptions,
    max_attempts: usize,
) -> Result<AiRig, AiError> {
    propose_rig_with_ai_guided(
        provider,
        model,
        parts,
        image,
        options,
        max_attempts,
        &RigGuidance::default(),
    )
}

/// [`propose_rig_with_ai`] の、利用者の注文と前の答え (作り直し) を添える版
/// (`docs/ai-integration.md#利用者の注文`)。
///
/// 出力の形は [`rig_output_schema`] のまま。4 つの値で反映できない注文は、答えの理由に `Not applied:` で
/// 始まる行として書かせ、[`AiRig::unapplied`] に分けて返す (提案の理由には本文だけを書く)。
pub fn propose_rig_with_ai_guided(
    provider: &dyn Provider,
    model: &Model,
    parts: &BTreeMap<MeshId, MeshPart>,
    image: &[u8],
    options: &RigOptions,
    max_attempts: usize,
    guidance: &RigGuidance,
) -> Result<AiRig, AiError> {
    let layout = Layout::new(model, parts)?;
    let answered = ask_until_valid(
        provider,
        &rig_output_schema(),
        max_attempts,
        |rejected| request(model, &layout, image, rejected, guidance),
        params_from_answer,
        |params| build(model, parts, &layout, params, options),
    )?;
    let Answered {
        mut proposal,
        params,
        reason,
        unapplied,
        rejected,
    } = answered;
    proposal.rationale = format!(
        "Values chosen by the AI: turn {:.2}, eye close line {:.2}, bend {:+.2}, mouth open {:.2} ({})\n{}",
        params.turn_angle_scale,
        params.eye_close_line,
        params.eye_close_bend,
        params.mouth_open_height,
        reason,
        proposal.rationale
    );
    Ok(AiRig {
        proposal,
        params,
        reason,
        unapplied,
        rejected,
    })
}

/// [`ask_until_valid`] の結果: 採用した提案・値・理由 (反映できなかった注文を除いた本文)・反映できなかった注文・
/// 捨てた答え。
pub(crate) struct Answered<P> {
    pub(crate) proposal: Proposal,
    pub(crate) params: P,
    pub(crate) reason: String,
    pub(crate) unapplied: Vec<String>,
    pub(crate) rejected: Vec<RejectedAnswer>,
}

/// テンプレートで決められない値を `provider` に決めさせ、組めるまで問い直す (リグとモジュールのリグで共有する)。
///
/// `request` は捨てた答えを添えたリクエスト、`parse` は答えを値と理由に読む、`build` は値で提案を組む。
/// 形の合わない答え (`parse` の失敗) と、AI の値が原因で組めない答え (`build` の [`AiError::InvalidProposal`]。
/// 範囲の外・適用や検証を通らない・組むものがなくなる値) は答えとエラーを添えて問い直す。それ以外のエラー
/// (テンプレートの内部の失敗 ([`template`]) は [`AiError::Template`]) はそのまま返す。`max_attempts` 回で
/// 通らなければ [`AiError::InvalidProposal`]。`max_attempts` が 0 なら問い合わせずに [`AiError::InvalidRequest`]。
pub(crate) fn ask_until_valid<P>(
    provider: &dyn Provider,
    schema: &serde_json::Value,
    max_attempts: usize,
    request: impl Fn(&[RejectedAnswer]) -> ProviderRequest,
    parse: impl Fn(&serde_json::Value) -> Result<(P, String), AiError>,
    build: impl Fn(&P) -> Result<Proposal, AiError>,
) -> Result<Answered<P>, AiError> {
    if max_attempts == 0 {
        return Err(AiError::InvalidRequest(
            "max_attempts is 0; the AI must be asked at least once".into(),
        ));
    }
    let mut rejected: Vec<RejectedAnswer> = Vec::new();
    for _ in 0..max_attempts {
        let value = provider.complete_json(&request(&rejected), schema)?;
        let error = match parse(&value) {
            Ok((params, reason)) => match build(&params) {
                Ok(proposal) => {
                    let (reason, unapplied) = split_unapplied(&reason);
                    return Ok(Answered {
                        proposal,
                        params,
                        reason,
                        unapplied,
                        rejected,
                    });
                }
                Err(AiError::InvalidProposal(error)) => error,
                Err(e) => return Err(e),
            },
            Err(AiError::InvalidProposal(error)) => error,
            Err(e) => e.to_string(),
        };
        rejected.push(RejectedAnswer {
            answer: value,
            error,
        });
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

    /// 答えを数 1 つとして読み、`build` に渡す (問い直しの規則だけを確かめる)。
    fn ask(
        fake: &crate::FakeProvider,
        max_attempts: usize,
        build: impl Fn(&f64) -> Result<Proposal, AiError>,
    ) -> Result<Answered<f64>, AiError> {
        ask_until_valid(
            fake,
            &json!({}),
            max_attempts,
            |_| ProviderRequest::default(),
            |value| {
                value["v"]
                    .as_f64()
                    .map(|v| (v, "why".to_owned()))
                    .ok_or_else(|| {
                        AiError::InvalidProposal("the answer does not match the schema".into())
                    })
            },
            build,
        )
    }

    fn proposal() -> Proposal {
        Proposal {
            title: "t".into(),
            rationale: String::new(),
            commands: Vec::new(),
        }
    }

    #[test]
    fn a_wrong_value_is_asked_again() {
        let fake = crate::FakeProvider::new();
        fake.push(json!({ "v": 5.0 })).push(json!({ "v": 1.0 }));
        let answered = ask(&fake, 3, |v| {
            if *v > 2.0 {
                Err(AiError::InvalidProposal(format!("v is {v}")))
            } else {
                Ok(proposal())
            }
        })
        .unwrap();
        assert_eq!(answered.params, 1.0);
        assert_eq!(answered.rejected.len(), 1);
        assert_eq!(answered.rejected[0].error, "v is 5");
        assert_eq!(fake.calls().len(), 2);
    }

    /// テンプレートの内部の失敗は値を変えても直らないので、問い直さず、内部のエラー文を AI に送らない。
    #[test]
    fn a_template_failure_is_not_asked_again() {
        let fake = crate::FakeProvider::new();
        fake.push(json!({ "v": 1.0 })).push(json!({ "v": 1.0 }));
        let err = ask(&fake, 3, |_| {
            Err(template(
                RigTemplate::Character,
                "axis A has no key 1".into(),
            ))
        })
        .map(|a| a.params)
        .unwrap_err();
        assert!(
            matches!(&err, AiError::Template { template: RigTemplate::Character, reason } if reason == "axis A has no key 1"),
            "{err}"
        );
        assert_eq!(
            err.to_string(),
            "the character template cannot rig this model: axis A has no key 1"
        );
        assert_eq!(fake.calls().len(), 1);
    }

    #[test]
    fn zero_attempts_is_an_invalid_request() {
        let fake = crate::FakeProvider::new();
        fake.push(json!({ "v": 1.0 }));
        let err = ask(&fake, 0, |_| Ok(proposal()))
            .map(|a| a.params)
            .unwrap_err();
        assert!(
            matches!(&err, AiError::InvalidRequest(m) if m.contains("max_attempts is 0")),
            "{err}"
        );
        assert!(fake.calls().is_empty());
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
