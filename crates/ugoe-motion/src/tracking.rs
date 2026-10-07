//! フェイストラッキングの値からアバタープロファイルのパラメータを計算する。
//! 仕様: `docs/spec/avatar-profile.md#フェイストラッキングとの対応`。
//!
//! 入力は頭の回転、体の入力 (回転と肩の上下。省略できる) と ARKit の 52 ブレンドシェイプ ([`TrackingInput`])。
//! モデルごとに [`TrackingMap`] を 1 度作り (`Model.avatar.tracking` の上書きを解決する)、
//! 毎フレーム [`TrackingMap::apply`] でパラメータに書き込む。
//!
//! 平滑化・キャリブレーション (中立の姿勢の差し引き)・トラッキングが途切れたときのフェードは
//! 行わない。カメラやトラッカーごとに適切な処理が違うので、入力を作るホスト側で行う。

use alloc::vec::Vec;
use ugoe_model::avatar::{
    ARKIT_BLEND_SHAPES, AvatarProfile, HEAD_PITCH, HEAD_ROLL, HEAD_YAW, TrackingSource,
};
use ugoe_model::{Model, standard};

use ugoe_runtime::{ParamIndex, ParameterLayout, ParameterValues};

/// ARKit のブレンドシェイプの数 ([`ARKIT_BLEND_SHAPES`] の長さ)。
pub const BLEND_SHAPE_COUNT: usize = ARKIT_BLEND_SHAPES.len();

/// 体のトラッキングがないとき、体の回転に使う頭の回転の係数。
pub const BODY_FROM_HEAD: f32 = 0.3;

/// 名前が `name` の ARKit のブレンドシェイプの、[`ARKIT_BLEND_SHAPES`] での添字。
#[expect(
    clippy::indexing_slicing,
    reason = "i < BLEND_SHAPE_COUNT (const fn では get を使えない)"
)]
pub const fn blend_shape_index(name: &str) -> Option<usize> {
    let mut i = 0;
    while i < BLEND_SHAPE_COUNT {
        if bytes_eq(ARKIT_BLEND_SHAPES[i].as_bytes(), name.as_bytes()) {
            return Some(i);
        }
        i += 1;
    }
    None
}

#[expect(
    clippy::indexing_slicing,
    reason = "i < a.len() == b.len() (const fn では get を使えない)"
)]
const fn bytes_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// 対応表で使うブレンドシェイプの添字。名前を間違えるとコンパイルエラーになる。
#[expect(clippy::panic, reason = "定数の初期化 (コンパイル時の評価) でだけ呼ぶ")]
const fn bs(name: &str) -> usize {
    match blend_shape_index(name) {
        Some(i) => i,
        None => panic!("unknown ARKit blend shape"),
    }
}

const EYE_BLINK_LEFT: usize = bs("eyeBlinkLeft");
const EYE_BLINK_RIGHT: usize = bs("eyeBlinkRight");
const EYE_WIDE_LEFT: usize = bs("eyeWideLeft");
const EYE_WIDE_RIGHT: usize = bs("eyeWideRight");
const EYE_SQUINT_LEFT: usize = bs("eyeSquintLeft");
const EYE_SQUINT_RIGHT: usize = bs("eyeSquintRight");
const EYE_LOOK_IN_LEFT: usize = bs("eyeLookInLeft");
const EYE_LOOK_IN_RIGHT: usize = bs("eyeLookInRight");
const EYE_LOOK_OUT_LEFT: usize = bs("eyeLookOutLeft");
const EYE_LOOK_OUT_RIGHT: usize = bs("eyeLookOutRight");
const EYE_LOOK_UP_LEFT: usize = bs("eyeLookUpLeft");
const EYE_LOOK_UP_RIGHT: usize = bs("eyeLookUpRight");
const EYE_LOOK_DOWN_LEFT: usize = bs("eyeLookDownLeft");
const EYE_LOOK_DOWN_RIGHT: usize = bs("eyeLookDownRight");
const BROW_DOWN_LEFT: usize = bs("browDownLeft");
const BROW_DOWN_RIGHT: usize = bs("browDownRight");
const BROW_INNER_UP: usize = bs("browInnerUp");
const BROW_OUTER_UP_LEFT: usize = bs("browOuterUpLeft");
const BROW_OUTER_UP_RIGHT: usize = bs("browOuterUpRight");
const JAW_OPEN: usize = bs("jawOpen");
const JAW_LEFT: usize = bs("jawLeft");
const JAW_RIGHT: usize = bs("jawRight");
const MOUTH_CLOSE: usize = bs("mouthClose");
const MOUTH_SMILE_LEFT: usize = bs("mouthSmileLeft");
const MOUTH_SMILE_RIGHT: usize = bs("mouthSmileRight");
const MOUTH_FROWN_LEFT: usize = bs("mouthFrownLeft");
const MOUTH_FROWN_RIGHT: usize = bs("mouthFrownRight");
const MOUTH_LEFT: usize = bs("mouthLeft");
const MOUTH_RIGHT: usize = bs("mouthRight");
const MOUTH_PUCKER: usize = bs("mouthPucker");
const MOUTH_FUNNEL: usize = bs("mouthFunnel");
const MOUTH_PRESS_LEFT: usize = bs("mouthPressLeft");
const MOUTH_PRESS_RIGHT: usize = bs("mouthPressRight");
const MOUTH_SHRUG_UPPER: usize = bs("mouthShrugUpper");
const MOUTH_SHRUG_LOWER: usize = bs("mouthShrugLower");
const CHEEK_PUFF: usize = bs("cheekPuff");
const TONGUE_OUT: usize = bs("tongueOut");
const NOSE_SNEER_LEFT: usize = bs("noseSneerLeft");
const NOSE_SNEER_RIGHT: usize = bs("noseSneerRight");

/// 回転 (度)。符号はプロファイルの「向きの約束」に合わせる
/// (`docs/spec/avatar-profile.md#向きと左右の約束`)。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Rotation {
    /// 正でキャラクターから見て左を向く (`AngleX` の向き)。
    pub yaw: f32,
    /// 正で上を向く (`AngleY` の向き)。
    pub pitch: f32,
    /// 正で時計回りに傾く (Y 下向きの画面で見て。`AngleZ` の向き)。
    pub roll: f32,
}

/// 体 (上半身) のトラッキングの入力。`docs/spec/avatar-profile.md#頭と体`。
///
/// どの値も省略できる (体が映っていない・信頼度が低い軸は `None`)。回転 (度) の符号は [`Rotation`] と同じで、
/// 省略した軸は頭の回転 × [`BODY_FROM_HEAD`] を使う。肩の上下は 0 が中立、1 ですくめた状態、負で下げた状態で、
/// 左右はトラッキングされた人自身の左右。省略した肩のパラメータには書き込まない。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BodyInput {
    /// 体のヨー (`BodyAngleX` の向き)。
    pub yaw: Option<f32>,
    /// 体のピッチ (`BodyAngleY` の向き)。
    pub pitch: Option<f32>,
    /// 体のロール (`BodyAngleZ` の向き)。
    pub roll: Option<f32>,
    /// 左肩の上下 (`ShoulderLY`)。
    pub shoulder_left: Option<f32>,
    /// 右肩の上下 (`ShoulderRY`)。
    pub shoulder_right: Option<f32>,
}

/// 1 フレーム分のトラッキングの入力。
///
/// ブレンドシェイプの `Left` / `Right` はトラッキングされた人自身の左右
/// (キャラクター自身の左右と同じ向き)。鏡像にした映像から得た値は、ホストが入れ替えてから渡す。
/// 頭の回転はカメラに対する回転で、体の回転を差し引かない (`AngleX` などはカメラに対する頭の向き)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackingInput {
    /// 頭の回転。
    pub head: Rotation,
    /// 体の入力。既定 (すべて `None`) なら、体の回転は頭の回転 × [`BODY_FROM_HEAD`] を使う。
    pub body: BodyInput,
    /// ARKit のブレンドシェイプの値 (0〜1)。添字は [`ARKIT_BLEND_SHAPES`] の順。
    pub blend_shapes: [f32; BLEND_SHAPE_COUNT],
}

impl Default for TrackingInput {
    fn default() -> Self {
        Self {
            head: Rotation::default(),
            body: BodyInput::default(),
            blend_shapes: [0.0; BLEND_SHAPE_COUNT],
        }
    }
}

impl TrackingInput {
    /// 名前が `name` のブレンドシェイプの値。知らない名前なら `None`。
    pub fn blend_shape(&self, name: &str) -> Option<f32> {
        blend_shape_index(name).and_then(|i| self.blend_shapes.get(i).copied())
    }

    /// 名前が `name` のブレンドシェイプの値を設定する。知らない名前 (MediaPipe の `_neutral` など) なら
    /// 何もせず `false` を返す。
    pub fn set_blend_shape(&mut self, name: &str, value: f32) -> bool {
        match blend_shape_index(name).and_then(|i| self.blend_shapes.get_mut(i)) {
            Some(v) => {
                *v = value;
                true
            }
            None => false,
        }
    }

    /// (名前, 値) の列からブレンドシェイプを設定する。MediaPipe Face Landmarker の
    /// カテゴリ名はそのまま渡せる。知らない名前は無視し、設定した数を返す。
    pub fn set_blend_shapes<S: AsRef<str>>(
        &mut self,
        values: impl IntoIterator<Item = (S, f32)>,
    ) -> usize {
        values
            .into_iter()
            .filter(|(name, value)| self.set_blend_shape(name.as_ref(), *value))
            .count()
    }

    /// 体の回転。省略した軸は頭の回転 × [`BODY_FROM_HEAD`]。
    fn body_rotation(&self) -> Rotation {
        Rotation {
            yaw: self.body.yaw.unwrap_or(self.head.yaw * BODY_FROM_HEAD),
            pitch: self.body.pitch.unwrap_or(self.head.pitch * BODY_FROM_HEAD),
            roll: self.body.roll.unwrap_or(self.head.roll * BODY_FROM_HEAD),
        }
    }

    fn head_value(&self, axis: Axis) -> f32 {
        match axis {
            Axis::Yaw => self.head.yaw,
            Axis::Pitch => self.head.pitch,
            Axis::Roll => self.head.roll,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    Yaw,
    Pitch,
    Roll,
}

impl Axis {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            HEAD_YAW => Some(Self::Yaw),
            HEAD_PITCH => Some(Self::Pitch),
            HEAD_ROLL => Some(Self::Roll),
            _ => None,
        }
    }
}

/// 標準の対応表の式 (`ugoe_model::avatar::TRACKED_PARAMETERS` の 1 つ)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Formula {
    AngleX,
    AngleY,
    AngleZ,
    BodyAngleX,
    BodyAngleY,
    BodyAngleZ,
    ShoulderLY,
    ShoulderRY,
    EyeLOpen,
    EyeROpen,
    EyeLWide,
    EyeRWide,
    EyeLSmile,
    EyeRSmile,
    EyeBallX,
    EyeBallY,
    BrowLY,
    BrowRY,
    BrowInnerUp,
    BrowLAngle,
    BrowRAngle,
    MouthOpenY,
    MouthForm,
    MouthX,
    JawX,
    MouthPucker,
    MouthFunnel,
    MouthPress,
    MouthShrug,
    CheekPuff,
    TongueOut,
    NoseSneer,
}

impl Formula {
    fn from_parameter(id: &str) -> Option<Self> {
        Some(match id {
            standard::ANGLE_X => Self::AngleX,
            standard::ANGLE_Y => Self::AngleY,
            standard::ANGLE_Z => Self::AngleZ,
            standard::BODY_ANGLE_X => Self::BodyAngleX,
            standard::BODY_ANGLE_Y => Self::BodyAngleY,
            standard::BODY_ANGLE_Z => Self::BodyAngleZ,
            standard::SHOULDER_L_Y => Self::ShoulderLY,
            standard::SHOULDER_R_Y => Self::ShoulderRY,
            standard::EYE_L_OPEN => Self::EyeLOpen,
            standard::EYE_R_OPEN => Self::EyeROpen,
            standard::EYE_L_WIDE => Self::EyeLWide,
            standard::EYE_R_WIDE => Self::EyeRWide,
            standard::EYE_L_SMILE => Self::EyeLSmile,
            standard::EYE_R_SMILE => Self::EyeRSmile,
            standard::EYE_BALL_X => Self::EyeBallX,
            standard::EYE_BALL_Y => Self::EyeBallY,
            standard::BROW_L_Y => Self::BrowLY,
            standard::BROW_R_Y => Self::BrowRY,
            standard::BROW_INNER_UP => Self::BrowInnerUp,
            standard::BROW_L_ANGLE => Self::BrowLAngle,
            standard::BROW_R_ANGLE => Self::BrowRAngle,
            standard::MOUTH_OPEN_Y => Self::MouthOpenY,
            standard::MOUTH_FORM => Self::MouthForm,
            standard::MOUTH_X => Self::MouthX,
            standard::JAW_X => Self::JawX,
            standard::MOUTH_PUCKER => Self::MouthPucker,
            standard::MOUTH_FUNNEL => Self::MouthFunnel,
            standard::MOUTH_PRESS => Self::MouthPress,
            standard::MOUTH_SHRUG => Self::MouthShrug,
            standard::CHEEK_PUFF => Self::CheekPuff,
            standard::TONGUE_OUT => Self::TongueOut,
            standard::NOSE_SNEER => Self::NoseSneer,
            _ => return None,
        })
    }

    /// 式の値 (クランプ前)。値が無い (肩の入力を省略した) ときは NaN で、書き込まれない。
    fn eval(self, input: &TrackingInput) -> f32 {
        let b = &input.blend_shapes;
        match self {
            Self::AngleX => input.head.yaw,
            Self::AngleY => input.head.pitch,
            Self::AngleZ => input.head.roll,
            Self::BodyAngleX => input.body_rotation().yaw,
            Self::BodyAngleY => input.body_rotation().pitch,
            Self::BodyAngleZ => input.body_rotation().roll,
            Self::ShoulderLY => input.body.shoulder_left.unwrap_or(f32::NAN),
            Self::ShoulderRY => input.body.shoulder_right.unwrap_or(f32::NAN),
            Self::EyeLOpen => 1.0 - b[EYE_BLINK_LEFT],
            Self::EyeROpen => 1.0 - b[EYE_BLINK_RIGHT],
            Self::EyeLWide => b[EYE_WIDE_LEFT],
            Self::EyeRWide => b[EYE_WIDE_RIGHT],
            Self::EyeLSmile => b[EYE_SQUINT_LEFT],
            Self::EyeRSmile => b[EYE_SQUINT_RIGHT],
            Self::EyeBallX => {
                ((b[EYE_LOOK_OUT_LEFT] - b[EYE_LOOK_IN_LEFT])
                    - (b[EYE_LOOK_OUT_RIGHT] - b[EYE_LOOK_IN_RIGHT]))
                    / 2.0
            }
            Self::EyeBallY => {
                (b[EYE_LOOK_UP_LEFT] + b[EYE_LOOK_UP_RIGHT]
                    - b[EYE_LOOK_DOWN_LEFT]
                    - b[EYE_LOOK_DOWN_RIGHT])
                    / 2.0
            }
            Self::BrowLY => (b[BROW_OUTER_UP_LEFT] + b[BROW_INNER_UP]) / 2.0 - b[BROW_DOWN_LEFT],
            Self::BrowRY => (b[BROW_OUTER_UP_RIGHT] + b[BROW_INNER_UP]) / 2.0 - b[BROW_DOWN_RIGHT],
            Self::BrowInnerUp => b[BROW_INNER_UP],
            Self::BrowLAngle => b[BROW_OUTER_UP_LEFT] - b[BROW_INNER_UP],
            Self::BrowRAngle => b[BROW_OUTER_UP_RIGHT] - b[BROW_INNER_UP],
            Self::MouthOpenY => {
                // `f32::max` は NaN を捨てるので使わない (NaN の入力は書き込まずに残す)。
                let v = b[JAW_OPEN] - b[MOUTH_CLOSE];
                if v < 0.0 { 0.0 } else { v }
            }
            Self::MouthForm => {
                (b[MOUTH_SMILE_LEFT] + b[MOUTH_SMILE_RIGHT]
                    - b[MOUTH_FROWN_LEFT]
                    - b[MOUTH_FROWN_RIGHT])
                    / 2.0
            }
            Self::MouthX => b[MOUTH_LEFT] - b[MOUTH_RIGHT],
            Self::JawX => b[JAW_LEFT] - b[JAW_RIGHT],
            Self::MouthPucker => b[MOUTH_PUCKER],
            Self::MouthFunnel => b[MOUTH_FUNNEL],
            Self::MouthPress => (b[MOUTH_PRESS_LEFT] + b[MOUTH_PRESS_RIGHT]) / 2.0,
            Self::MouthShrug => (b[MOUTH_SHRUG_UPPER] + b[MOUTH_SHRUG_LOWER]) / 2.0,
            Self::CheekPuff => b[CHEEK_PUFF],
            Self::TongueOut => b[TONGUE_OUT],
            Self::NoseSneer => (b[NOSE_SNEER_LEFT] + b[NOSE_SNEER_RIGHT]) / 2.0,
        }
    }
}

/// 標準の対応表で計算したパラメータ `id` の値 (クランプ前)。`id` が表にないパラメータなら `None`。
/// 表にあっても入力に値が無い (肩の入力を省略した) ときは NaN (そのパラメータには書き込まない)。
pub fn standard_value(id: &str, input: &TrackingInput) -> Option<f32> {
    Formula::from_parameter(id).map(|f| f.eval(input))
}

/// 上書きの入力。
#[derive(Debug, Clone, Copy, PartialEq)]
enum Source {
    Standard(Formula),
    BlendShape(usize),
    Head(Axis),
}

impl Source {
    fn eval(self, input: &TrackingInput) -> f32 {
        match self {
            Self::Standard(f) => f.eval(input),
            // 添字は `blend_shape_index` で引いたもの (いつも範囲内)。無ければ有限でない値として書き込まない。
            Self::BlendShape(i) => input.blend_shapes.get(i).copied().unwrap_or(f32::NAN),
            Self::Head(axis) => input.head_value(axis),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Mapping {
    source: Source,
    scale: f32,
    offset: f32,
}

/// 1 つのモデルについて解決したトラッキングの対応。`docs/spec/avatar-profile.md#トラッキングの対応の上書き`。
///
/// 作るときに `Model.avatar.tracking` の上書きを解決し、パラメータの添字を引いておくので、
/// [`TrackingMap::apply`] は文字列を比べず、メモリも確保しない。
///
/// 作ったときと並びの違う `ParameterValues` に書くときは、添字を書き込み先ごとに ID で引き直して
/// 同じ ID のパラメータに書く (T0325。並びが同じかは定数時間で見分ける)。引き直した添字は控えない
/// (T0181。控えを `&self` のまま取り替えるにはロックが要り、`std` の無い環境に `Mutex` は無い。
/// 今の呼び出し側は作ったときと同じ並びに書くので、引き直しは起きない)。
#[derive(Debug, Clone, Default)]
pub struct TrackingMap {
    /// 上書きされていない標準の対応。
    standard: Vec<(Formula, ParamIndex)>,
    /// 上書き。書き込み先ごとにまとめ、`groups` の範囲で引く。
    overrides: Vec<Mapping>,
    /// 書き込み先と、`overrides` の範囲 (開始, 終了)。
    groups: Vec<(ParamIndex, usize, usize)>,
    /// 添字を解決したときのパラメータの並び。
    layout: Option<ParameterLayout>,
}

/// 対応が同じか。並びは手札でなく ID の列で比べる。
impl PartialEq for TrackingMap {
    fn eq(&self, other: &Self) -> bool {
        self.standard == other.standard
            && self.overrides == other.overrides
            && self.groups == other.groups
            && self.layout.as_ref().map(ParameterLayout::ids)
                == other.layout.as_ref().map(ParameterLayout::ids)
    }
}

impl TrackingMap {
    /// `avatar` の上書き (`None` なら上書きなし) を解決する。`parameters` はモデルのパラメータ。
    ///
    /// 次の上書きは無視し、標準の対応の置き換えも起こさない: 知らない `source`、`source` の種類の
    /// 入力として知らない `input`、書き込み先がない (`standard` 以外で `parameter` を省略した) もの、
    /// 書き込み先のパラメータがモデルにないもの、`scale` か `offset` が有限値でないもの。
    /// 標準の対応も、パラメータがモデルにないものは作らない。
    pub fn new(avatar: Option<&AvatarProfile>, parameters: &ParameterValues) -> Self {
        let mut resolved: Vec<(ParamIndex, Mapping)> = Vec::new();
        let mut replaced: Vec<Formula> = Vec::new();
        for o in avatar.map_or(&[][..], |a| &a.tracking) {
            let source = match o.source {
                TrackingSource::Standard => Formula::from_parameter(&o.input).map(Source::Standard),
                TrackingSource::BlendShape => blend_shape_index(&o.input).map(Source::BlendShape),
                TrackingSource::Head => Axis::from_name(&o.input).map(Source::Head),
                TrackingSource::Unknown => None,
            };
            let Some(source) = source else { continue };
            let Some(target) = o.target().and_then(|id| parameters.index_of(id)) else {
                continue;
            };
            if !o.scale.is_finite() || !o.offset.is_finite() {
                continue;
            }
            if let Source::Standard(f) = source {
                replaced.push(f);
            }
            if let Some(f) = o.target().and_then(Formula::from_parameter) {
                replaced.push(f);
            }
            resolved.push((
                target,
                Mapping {
                    source,
                    scale: o.scale,
                    offset: o.offset,
                },
            ));
        }

        let standard = ugoe_model::avatar::TRACKED_PARAMETERS
            .iter()
            .filter_map(|id| Some((Formula::from_parameter(id)?, parameters.index_of(id)?)))
            .filter(|(f, _)| !replaced.contains(f))
            .collect();

        // 書き込み先ごとにまとめる (同じ書き込み先の中では上書きの順を保つ)。
        resolved.sort_by_key(|(target, _)| target.0);
        let mut groups: Vec<(ParamIndex, usize, usize)> = Vec::new();
        for (i, (target, _)) in resolved.iter().enumerate() {
            match groups.last_mut() {
                Some((last, _, end)) if last == target => *end = i + 1,
                _ => groups.push((*target, i, i + 1)),
            }
        }
        let overrides = resolved.into_iter().map(|(_, m)| m).collect();

        Self {
            standard,
            overrides,
            groups,
            layout: Some(parameters.layout()),
        }
    }

    /// `model` の上書き (`Model.avatar.tracking`) を解決する。`parameters` はモデルのパラメータ。
    ///
    /// [`Self::new`] と同じだが、中継用 (`Parameter::internal`) のパラメータには書き込まない
    /// (`docs/spec/model.md#パラメータ`)。標準の対応でも上書きでも同じで、書き込み先が中継用の上書きは、
    /// 標準の対応の置き換えは起こす (その標準の対応も書き込まない)。
    pub fn for_model(model: &Model, parameters: &ParameterValues) -> Self {
        let mut map = Self::new(model.avatar.as_ref(), parameters);
        let internal: Vec<ParamIndex> = model
            .parameters
            .iter()
            .filter(|p| p.internal)
            .filter_map(|p| parameters.index_of(p.id.as_str()))
            .collect();
        if internal.is_empty() {
            return map;
        }
        map.standard.retain(|(_, index)| !internal.contains(index));
        let overrides = core::mem::take(&mut map.overrides);
        let mut kept = Vec::new();
        map.groups.retain_mut(|(index, start, end)| {
            if internal.contains(index) {
                return false;
            }
            let group = overrides.get(*start..*end).unwrap_or_default();
            let range = kept.len()..kept.len() + group.len();
            kept.extend_from_slice(group);
            (*start, *end) = (range.start, range.end);
            true
        });
        map.overrides = kept;
        map
    }

    /// `input` から計算した値を `parameters` に書き込む (値域にクランプする)。
    /// 対応のないパラメータには触らない。計算した値が有限値でなければ、そのパラメータには書き込まない。
    pub fn apply(&self, input: &TrackingInput, parameters: &mut ParameterValues) {
        self.apply_where(input, parameters, |_| true);
    }

    /// [`Self::apply`] と同じだが、`filter` が `true` を返すパラメータにだけ書き込む (T0722。
    /// `Puppet` がモーションの前と後に分けて書き込むため)。`filter` には `parameters` の並びの添字を渡す。
    pub fn apply_where(
        &self,
        input: &TrackingInput,
        parameters: &mut ParameterValues,
        filter: impl FnMut(ParamIndex) -> bool,
    ) {
        let Some(layout) = self
            .layout
            .as_ref()
            .filter(|layout| !parameters.has_layout(layout))
        else {
            return self.write_all(input, parameters, |_, index| Some(index), filter);
        };
        // 作ったときと並びが違えば、書き込み先ごとに添字を ID で引き直す。
        let lookup = |parameters: &ParameterValues, index: ParamIndex| {
            let id = layout.ids().get(index.0 as usize)?;
            parameters.index_of(id.as_str())
        };
        self.write_all(input, parameters, lookup, filter);
    }

    /// 書き込み先を `target` で解決しながら書き込む。`target` には作ったときの添字を渡し、
    /// `parameters` の並びの添字を受け取る。
    fn write_all(
        &self,
        input: &TrackingInput,
        parameters: &mut ParameterValues,
        target: impl Fn(&ParameterValues, ParamIndex) -> Option<ParamIndex>,
        mut filter: impl FnMut(ParamIndex) -> bool,
    ) {
        for (formula, index) in &self.standard {
            if let Some(index) = target(parameters, *index)
                && filter(index)
            {
                write(parameters, index, formula.eval(input));
            }
        }
        for (index, start, end) in &self.groups {
            let Some(index) = target(parameters, *index).filter(|&i| filter(i)) else {
                continue;
            };
            // 範囲は対応表を作るときに `overrides` に積んだもの (いつも範囲内)。
            let value = (self.overrides.get(*start..*end).unwrap_or_default())
                .iter()
                .map(|m| m.source.eval(input) * m.scale + m.offset)
                .sum();
            write(parameters, index, value);
        }
    }

    /// `apply` が書き込むパラメータ (重複なし、順不同)。添字は作ったときに渡した `parameters` の並びのもの。
    pub fn targets(&self) -> impl Iterator<Item = ParamIndex> + '_ {
        self.standard
            .iter()
            .map(|(_, index)| *index)
            .filter(|index| !self.groups.iter().any(|(g, _, _)| g == index))
            .chain(self.groups.iter().map(|(index, _, _)| *index))
    }
}

fn write(parameters: &mut ParameterValues, index: ParamIndex, value: f32) {
    if value.is_finite() {
        parameters.set(index, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ugoe_model::avatar::{TRACKED_PARAMETERS, TrackingOverride};
    use ugoe_model::fixtures;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    /// 名前で指定したブレンドシェイプだけを持つ入力。
    fn input(values: &[(&str, f32)]) -> TrackingInput {
        let mut input = TrackingInput::default();
        assert_eq!(input.set_blend_shapes(values.iter().copied()), values.len());
        input
    }

    fn std_value(id: &str, values: &[(&str, f32)]) -> f32 {
        standard_value(id, &input(values)).unwrap()
    }

    fn params() -> ParameterValues {
        ParameterValues::from_model(&fixtures::avatar_profile())
    }

    /// 体の拡張パラメータ (肩) をモデルに足す (フィクスチャには無い)。
    fn add_shoulders(model: &mut Model) {
        for &(id, min, default, max) in standard::BODY_EXTENDED {
            model.parameters.push(ugoe_model::Parameter {
                id: id.into(),
                min,
                default,
                max,
                ..model.parameters[0].clone()
            });
        }
    }

    fn params_with_shoulders() -> ParameterValues {
        let mut model = fixtures::avatar_profile();
        add_shoulders(&mut model);
        ParameterValues::from_model(&model)
    }

    fn value(params: &ParameterValues, id: &str) -> f32 {
        params.get(params.index_of(id).unwrap())
    }

    fn map_with(overrides: Vec<TrackingOverride>) -> (TrackingMap, ParameterValues) {
        let model = fixtures::avatar_profile();
        let mut avatar = model.avatar.clone().unwrap();
        avatar.tracking = overrides;
        let params = ParameterValues::from_model(&model);
        (TrackingMap::new(Some(&avatar), &params), params)
    }

    fn over(source: TrackingSource, input: &str, parameter: Option<&str>) -> TrackingOverride {
        TrackingOverride {
            source,
            input: input.into(),
            parameter: parameter.map(Into::into),
            scale: 1.0,
            offset: 0.0,
        }
    }

    #[test]
    fn blend_shape_names_resolve() {
        for (i, name) in ARKIT_BLEND_SHAPES.iter().enumerate() {
            assert_eq!(blend_shape_index(name), Some(i));
        }
        assert_eq!(blend_shape_index("_neutral"), None);
        assert_eq!(blend_shape_index("eyeBlink"), None);
    }

    #[test]
    fn unknown_blend_shape_names_are_ignored() {
        let mut input = TrackingInput::default();
        let n = input.set_blend_shapes([("_neutral", 1.0), ("eyeBlinkLeft", 0.25), ("foo", 1.0)]);
        assert_eq!(n, 1);
        assert_eq!(input.blend_shape("eyeBlinkLeft"), Some(0.25));
        assert_eq!(input.blend_shape("_neutral"), None);
        assert!(input.blend_shapes.iter().filter(|v| **v != 0.0).count() == 1);
    }

    #[test]
    fn every_tracked_parameter_has_a_formula() {
        let input = TrackingInput::default();
        for id in TRACKED_PARAMETERS {
            assert!(standard_value(id, &input).is_some(), "{id}");
        }
        assert_eq!(standard_value("Breath", &input), None);
    }

    #[test]
    fn head_and_body_angles() {
        let mut input = TrackingInput {
            head: Rotation {
                yaw: 20.0,
                pitch: -10.0,
                roll: 5.0,
            },
            ..TrackingInput::default()
        };
        let v = |id, input: &TrackingInput| standard_value(id, input).unwrap();
        assert_eq!(v(standard::ANGLE_X, &input), 20.0);
        assert_eq!(v(standard::ANGLE_Y, &input), -10.0);
        assert_eq!(v(standard::ANGLE_Z, &input), 5.0);
        assert!(close(v(standard::BODY_ANGLE_X, &input), 6.0));
        assert!(close(v(standard::BODY_ANGLE_Y, &input), -3.0));
        assert!(close(v(standard::BODY_ANGLE_Z, &input), 1.5));

        input.body = BodyInput {
            yaw: Some(1.0),
            pitch: Some(2.0),
            roll: Some(3.0),
            ..BodyInput::default()
        };
        assert_eq!(v(standard::BODY_ANGLE_X, &input), 1.0);
        assert_eq!(v(standard::BODY_ANGLE_Y, &input), 2.0);
        assert_eq!(v(standard::BODY_ANGLE_Z, &input), 3.0);
        // 頭の角度は体の回転を差し引かない (カメラに対する頭の向き)。
        assert_eq!(v(standard::ANGLE_X, &input), 20.0);
        assert_eq!(v(standard::ANGLE_Z, &input), 5.0);
    }

    #[test]
    fn without_body_input_the_values_are_as_before() {
        // T0645 より前の式: 頭はそのまま、体は頭 × 0.3。肩は値が無い。
        let input = TrackingInput {
            head: Rotation {
                yaw: 12.0,
                pitch: -7.0,
                roll: 3.0,
            },
            ..input(&[("jawOpen", 0.4), ("eyeBlinkLeft", 0.3)])
        };
        let mut params = params_with_shoulders();
        let before = params.clone();
        TrackingMap::new(None, &params).apply(&input, &mut params);
        assert_eq!(value(&params, standard::ANGLE_X), 12.0);
        assert!(close(value(&params, standard::BODY_ANGLE_X), 3.6));
        assert!(close(value(&params, standard::BODY_ANGLE_Y), -2.1));
        assert!(close(value(&params, standard::BODY_ANGLE_Z), 0.9));
        assert!(close(value(&params, standard::MOUTH_OPEN_Y), 0.4));
        for id in [standard::SHOULDER_L_Y, standard::SHOULDER_R_Y] {
            assert_eq!(value(&params, id), value(&before, id), "{id}");
        }
    }

    #[test]
    fn body_axes_fall_back_to_the_head_one_by_one() {
        // 腰が映らずピッチだけ省略した体の入力。
        let input = TrackingInput {
            head: Rotation {
                yaw: 20.0,
                pitch: -10.0,
                roll: 5.0,
            },
            body: BodyInput {
                yaw: Some(4.0),
                roll: Some(-2.0),
                ..BodyInput::default()
            },
            ..TrackingInput::default()
        };
        let v = |id| standard_value(id, &input).unwrap();
        assert_eq!(v(standard::BODY_ANGLE_X), 4.0);
        assert!(close(v(standard::BODY_ANGLE_Y), -3.0));
        assert_eq!(v(standard::BODY_ANGLE_Z), -2.0);
    }

    #[test]
    fn shoulders_are_written_only_when_given() {
        let mut params = params_with_shoulders();
        let map = TrackingMap::new(None, &params);
        let mut input = TrackingInput::default();
        assert!(
            standard_value(standard::SHOULDER_L_Y, &input)
                .unwrap()
                .is_nan()
        );

        // 入力が無ければ書き込まない (ほかから与えた値が残る)。
        let l = params.index_of(standard::SHOULDER_L_Y).unwrap();
        params.set(l, 0.5);
        map.apply(&input, &mut params);
        assert_eq!(value(&params, standard::SHOULDER_L_Y), 0.5);
        assert_eq!(value(&params, standard::SHOULDER_R_Y), 0.0);

        input.body.shoulder_left = Some(0.25);
        input.body.shoulder_right = Some(-3.0);
        map.apply(&input, &mut params);
        assert_eq!(value(&params, standard::SHOULDER_L_Y), 0.25);
        // 値域 (-1〜1) にクランプする。
        assert_eq!(value(&params, standard::SHOULDER_R_Y), -1.0);
    }

    #[test]
    fn override_reading_a_missing_shoulder_writes_nothing() {
        let mut model = fixtures::avatar_profile();
        add_shoulders(&mut model);
        let mut avatar = model.avatar.clone().unwrap();
        avatar.tracking = vec![TrackingOverride {
            scale: 2.0,
            ..over(
                TrackingSource::Standard,
                standard::SHOULDER_L_Y,
                Some(standard::BREATH),
            )
        }];
        let mut params = ParameterValues::from_model(&model);
        let map = TrackingMap::new(Some(&avatar), &params);
        let breath = params.index_of(standard::BREATH).unwrap();
        params.set(breath, 0.3);
        map.apply(&TrackingInput::default(), &mut params);
        assert_eq!(value(&params, standard::BREATH), 0.3);

        let mut input = TrackingInput::default();
        input.body.shoulder_left = Some(0.25);
        map.apply(&input, &mut params);
        assert_eq!(value(&params, standard::BREATH), 0.5);
    }

    #[test]
    fn eye_formulas() {
        assert!(close(
            std_value(standard::EYE_L_OPEN, &[("eyeBlinkLeft", 0.75)]),
            0.25
        ));
        assert!(close(
            std_value(
                standard::EYE_R_OPEN,
                &[("eyeBlinkRight", 0.75), ("eyeBlinkLeft", 1.0)]
            ),
            0.25
        ));
        assert_eq!(
            std_value(standard::EYE_L_WIDE, &[("eyeWideLeft", 0.5)]),
            0.5
        );
        assert_eq!(
            std_value(standard::EYE_R_WIDE, &[("eyeWideRight", 0.5)]),
            0.5
        );
        assert_eq!(
            std_value(standard::EYE_L_SMILE, &[("eyeSquintLeft", 0.5)]),
            0.5
        );
        assert_eq!(
            std_value(standard::EYE_R_SMILE, &[("eyeSquintRight", 0.5)]),
            0.5
        );
    }

    #[test]
    fn eye_ball_formulas() {
        // キャラクターの左を見る: 左目は外、右目は内。
        let look_left = [("eyeLookOutLeft", 0.8), ("eyeLookInRight", 0.6)];
        assert!(close(std_value(standard::EYE_BALL_X, &look_left), 0.7));
        let look_right = [("eyeLookInLeft", 0.8), ("eyeLookOutRight", 0.6)];
        assert!(close(std_value(standard::EYE_BALL_X, &look_right), -0.7));
        let all = [
            ("eyeLookOutLeft", 0.4),
            ("eyeLookInLeft", 0.1),
            ("eyeLookOutRight", 0.2),
            ("eyeLookInRight", 0.3),
        ];
        assert!(close(
            std_value(standard::EYE_BALL_X, &all),
            ((0.4 - 0.1) - (0.2 - 0.3)) / 2.0
        ));

        let up = [
            ("eyeLookUpLeft", 0.6),
            ("eyeLookUpRight", 0.4),
            ("eyeLookDownLeft", 0.1),
            ("eyeLookDownRight", 0.3),
        ];
        assert!(close(
            std_value(standard::EYE_BALL_Y, &up),
            (0.6 + 0.4 - 0.1 - 0.3) / 2.0
        ));
    }

    #[test]
    fn brow_formulas() {
        let b = [
            ("browOuterUpLeft", 0.6),
            ("browOuterUpRight", 0.2),
            ("browInnerUp", 0.4),
            ("browDownLeft", 0.1),
            ("browDownRight", 0.5),
        ];
        assert!(close(
            std_value(standard::BROW_L_Y, &b),
            (0.6 + 0.4) / 2.0 - 0.1
        ));
        assert!(close(
            std_value(standard::BROW_R_Y, &b),
            (0.2 + 0.4) / 2.0 - 0.5
        ));
        assert!(close(std_value(standard::BROW_INNER_UP, &b), 0.4));
        assert!(close(std_value(standard::BROW_L_ANGLE, &b), 0.6 - 0.4));
        assert!(close(std_value(standard::BROW_R_ANGLE, &b), 0.2 - 0.4));
    }

    #[test]
    fn mouth_formulas() {
        assert!(close(
            std_value(
                standard::MOUTH_OPEN_Y,
                &[("jawOpen", 0.7), ("mouthClose", 0.2)]
            ),
            0.5
        ));
        // 口を閉じたままあごを下げても負にならない。
        assert_eq!(
            std_value(
                standard::MOUTH_OPEN_Y,
                &[("jawOpen", 0.1), ("mouthClose", 0.3)]
            ),
            0.0
        );
        let form = [
            ("mouthSmileLeft", 0.8),
            ("mouthSmileRight", 0.6),
            ("mouthFrownLeft", 0.1),
            ("mouthFrownRight", 0.3),
        ];
        assert!(close(
            std_value(standard::MOUTH_FORM, &form),
            (0.8 + 0.6 - 0.1 - 0.3) / 2.0
        ));
        assert!(close(
            std_value(
                standard::MOUTH_X,
                &[("mouthLeft", 0.7), ("mouthRight", 0.2)]
            ),
            0.5
        ));
        assert!(close(
            std_value(standard::JAW_X, &[("jawLeft", 0.1), ("jawRight", 0.4)]),
            -0.3
        ));
        assert_eq!(
            std_value(standard::MOUTH_PUCKER, &[("mouthPucker", 0.5)]),
            0.5
        );
        assert_eq!(
            std_value(standard::MOUTH_FUNNEL, &[("mouthFunnel", 0.5)]),
            0.5
        );
        assert!(close(
            std_value(
                standard::MOUTH_PRESS,
                &[("mouthPressLeft", 0.2), ("mouthPressRight", 0.6)]
            ),
            0.4
        ));
        assert!(close(
            std_value(
                standard::MOUTH_SHRUG,
                &[("mouthShrugUpper", 0.2), ("mouthShrugLower", 0.6)]
            ),
            0.4
        ));
    }

    #[test]
    fn face_formulas() {
        assert_eq!(std_value(standard::CHEEK_PUFF, &[("cheekPuff", 0.5)]), 0.5);
        assert_eq!(std_value(standard::TONGUE_OUT, &[("tongueOut", 0.5)]), 0.5);
        assert!(close(
            std_value(
                standard::NOSE_SNEER,
                &[("noseSneerLeft", 0.2), ("noseSneerRight", 0.6)]
            ),
            0.4
        ));
    }

    #[test]
    fn apply_writes_standard_mapping_with_clamp() {
        let mut params = params();
        let map = TrackingMap::new(None, &params);
        let mut input = input(&[("eyeBlinkLeft", 0.75), ("mouthLeft", 1.0)]);
        input.head.yaw = 100.0;
        map.apply(&input, &mut params);

        assert_eq!(value(&params, standard::ANGLE_X), 30.0);
        assert!(close(value(&params, standard::BODY_ANGLE_X), 10.0));
        assert!(close(value(&params, standard::EYE_L_OPEN), 0.25));
        assert_eq!(value(&params, standard::EYE_R_OPEN), 1.0);
        assert_eq!(value(&params, standard::MOUTH_X), 1.0);
        // 対応のないパラメータ (呼吸) には触らない。
        assert_eq!(value(&params, standard::BREATH), 0.0);
    }

    #[test]
    fn apply_where_writes_only_the_filtered_parameters() {
        let mut params = params();
        let map = TrackingMap::new(None, &params);
        let mut input = input(&[("eyeBlinkLeft", 0.75)]);
        input.head.yaw = 20.0;
        let eye = params.index_of(standard::EYE_L_OPEN).unwrap();
        map.apply_where(&input, &mut params, |i| i == eye);
        assert!(close(value(&params, standard::EYE_L_OPEN), 0.25));
        assert_eq!(value(&params, standard::ANGLE_X), 0.0);
        // 残りを書けば `apply` と同じ。
        map.apply_where(&input, &mut params, |i| i != eye);
        let mut all = self::params();
        map.apply(&input, &mut all);
        assert_eq!(params.as_slice(), all.as_slice());
    }

    #[test]
    fn apply_skips_parameters_missing_from_model() {
        let mut model = fixtures::avatar_profile();
        model.parameters.retain(|p| p.id != standard::MOUTH_X);
        let mut params = ParameterValues::from_model(&model);
        let map = TrackingMap::new(model.avatar.as_ref(), &params);
        map.apply(&input(&[("mouthLeft", 1.0)]), &mut params);
        assert_eq!(params.index_of(standard::MOUTH_X), None);
        let targets: Vec<_> = map.targets().collect();
        assert_eq!(targets.len(), map.standard.len());
    }

    #[test]
    fn apply_ignores_non_finite_values() {
        let mut params = params();
        let map = TrackingMap::new(None, &params);
        let mut input = TrackingInput::default();
        input.head.yaw = f32::NAN;
        params.set(params.index_of(standard::ANGLE_X).unwrap(), 12.0);
        map.apply(&input, &mut params);
        assert_eq!(value(&params, standard::ANGLE_X), 12.0);

        // 0 で抑える式 (`MouthOpenY`) も、NaN の入力では書き込まない (0 にしない)。
        let mouth = params.index_of(standard::MOUTH_OPEN_Y).unwrap();
        params.set(mouth, 0.5);
        let mut input = TrackingInput::default();
        input.set_blend_shape("jawOpen", f32::NAN);
        map.apply(&input, &mut params);
        assert_eq!(value(&params, standard::MOUTH_OPEN_Y), 0.5);
    }

    #[test]
    fn standard_override_flips_sign() {
        let mut o = over(TrackingSource::Standard, standard::EYE_BALL_X, None);
        o.scale = -1.0;
        let (map, mut params) = map_with(vec![o]);
        map.apply(&input(&[("eyeLookOutLeft", 0.8)]), &mut params);
        assert!(close(value(&params, standard::EYE_BALL_X), -0.4));
    }

    #[test]
    fn standard_override_redirects_to_other_parameter() {
        // `MouthForm` の代わりに `MouthPucker` を動かす (モデルにある別のパラメータを使う)。
        let o = over(
            TrackingSource::Standard,
            standard::MOUTH_FORM,
            Some(standard::MOUTH_PUCKER),
        );
        let (map, mut params) = map_with(vec![o]);
        let form = params.index_of(standard::MOUTH_FORM).unwrap();
        params.set(form, -0.5);
        map.apply(
            &input(&[
                ("mouthSmileLeft", 0.6),
                ("mouthSmileRight", 0.4),
                ("mouthPucker", 1.0),
            ]),
            &mut params,
        );
        // 標準の `MouthForm` の対応は行わない (上書きが置き換える)。
        assert_eq!(value(&params, standard::MOUTH_FORM), -0.5);
        // `MouthPucker` にはその上書きだけが書き込まれる (`mouthPucker` の標準の対応も置き換わる)。
        assert!(close(value(&params, standard::MOUTH_PUCKER), 0.5));
    }

    #[test]
    fn blend_shape_and_head_overrides_with_scale_and_offset() {
        let mut jaw = over(
            TrackingSource::BlendShape,
            "jawForward",
            Some(standard::CHEEK_PUFF),
        );
        jaw.scale = 0.5;
        jaw.offset = 0.1;
        let mut yaw = over(TrackingSource::Head, HEAD_YAW, Some(standard::ANGLE_Z));
        yaw.scale = -1.0;
        let (map, mut params) = map_with(vec![jaw, yaw]);
        let mut input = input(&[("jawForward", 0.6), ("cheekPuff", 1.0)]);
        input.head.yaw = 12.0;
        input.head.roll = 7.0;
        map.apply(&input, &mut params);
        assert!(close(value(&params, standard::CHEEK_PUFF), 0.4));
        assert!(close(value(&params, standard::ANGLE_Z), -12.0));
        assert!(close(value(&params, standard::ANGLE_X), 12.0));
    }

    #[test]
    fn overrides_to_the_same_parameter_are_summed_then_clamped() {
        let a = over(
            TrackingSource::BlendShape,
            "cheekSquintLeft",
            Some(standard::EYE_L_SMILE),
        );
        let b = over(
            TrackingSource::BlendShape,
            "eyeSquintLeft",
            Some(standard::EYE_L_SMILE),
        );
        let (map, mut params) = map_with(vec![a, b]);
        map.apply(
            &input(&[("cheekSquintLeft", 0.3), ("eyeSquintLeft", 0.4)]),
            &mut params,
        );
        assert!(close(value(&params, standard::EYE_L_SMILE), 0.7));
        map.apply(
            &input(&[("cheekSquintLeft", 0.8), ("eyeSquintLeft", 0.8)]),
            &mut params,
        );
        assert_eq!(value(&params, standard::EYE_L_SMILE), 1.0);
        assert_eq!(map.targets().count(), map.standard.len() + 1);
    }

    #[test]
    fn standard_override_uses_unclamped_value() {
        // 値域 0〜1 の `EyeLOpen` の式は、クランプ前の値を上書きの入力にする。
        let mut o = over(
            TrackingSource::Standard,
            standard::ANGLE_X,
            Some(standard::EYE_L_OPEN),
        );
        o.scale = 0.01;
        let (map, mut params) = map_with(vec![o]);
        let mut input = TrackingInput::default();
        input.head.yaw = 50.0;
        map.apply(&input, &mut params);
        assert!(close(value(&params, standard::EYE_L_OPEN), 0.5));
        // 標準の `AngleX` の対応は置き換わっている。
        assert_eq!(value(&params, standard::ANGLE_X), 0.0);
    }

    #[test]
    fn invalid_overrides_are_ignored() {
        let unknown_source = TrackingOverride {
            source: TrackingSource::Unknown,
            ..over(TrackingSource::Standard, standard::MOUTH_X, None)
        };
        let unknown_input = over(
            TrackingSource::BlendShape,
            "_neutral",
            Some(standard::EYE_L_OPEN),
        );
        let no_target = over(TrackingSource::Head, HEAD_ROLL, None);
        let missing_target = over(TrackingSource::Standard, standard::JAW_X, Some("Missing"));
        let mut not_finite = over(TrackingSource::Standard, standard::MOUTH_FORM, None);
        not_finite.scale = f32::INFINITY;
        let (map, mut params) = map_with(vec![
            unknown_source,
            unknown_input,
            no_target,
            missing_target,
            not_finite,
        ]);
        assert_eq!(map, TrackingMap::new(None, &params));

        let mut input = input(&[
            ("mouthLeft", 0.5),
            ("eyeBlinkLeft", 1.0),
            ("jawLeft", 0.5),
            ("mouthSmileLeft", 1.0),
        ]);
        input.head.roll = 10.0;
        map.apply(&input, &mut params);
        assert!(close(value(&params, standard::MOUTH_X), 0.5));
        assert_eq!(value(&params, standard::EYE_L_OPEN), 0.0);
        assert!(close(value(&params, standard::JAW_X), 0.5));
        assert!(close(value(&params, standard::MOUTH_FORM), 0.5));
        assert_eq!(value(&params, standard::ANGLE_Z), 10.0);
    }

    #[test]
    fn head_override_input_names() {
        for (name, expected) in [(HEAD_YAW, 1.0), (HEAD_PITCH, 2.0), (HEAD_ROLL, 3.0)] {
            let o = over(TrackingSource::Head, name, Some(standard::ANGLE_X));
            let (map, mut params) = map_with(vec![o]);
            let input = TrackingInput {
                head: Rotation {
                    yaw: 1.0,
                    pitch: 2.0,
                    roll: 3.0,
                },
                ..TrackingInput::default()
            };
            map.apply(&input, &mut params);
            assert_eq!(value(&params, standard::ANGLE_X), expected, "{name}");
        }
    }

    /// 中継用 (`internal`) のパラメータには、標準の対応でも上書きでも書き込まない。
    #[test]
    fn for_model_skips_internal_parameters() {
        let mut model = fixtures::avatar_profile();
        for p in &mut model.parameters {
            if [standard::ANGLE_X, standard::ANGLE_Y].contains(&p.id.as_str()) {
                p.internal = true;
            }
        }
        model.avatar.as_mut().unwrap().tracking = vec![
            over(TrackingSource::Head, HEAD_ROLL, Some(standard::ANGLE_Y)),
            over(
                TrackingSource::BlendShape,
                "jawLeft",
                Some(standard::MOUTH_X),
            ),
        ];
        let mut params = ParameterValues::from_model(&model);
        let map = TrackingMap::for_model(&model, &params);
        let targets: Vec<ParamIndex> = map.targets().collect();
        for id in [standard::ANGLE_X, standard::ANGLE_Y] {
            assert!(!targets.contains(&params.index_of(id).unwrap()), "{id}");
        }
        let mut input = input(&[("jawLeft", 0.5)]);
        input.head = Rotation {
            yaw: 10.0,
            pitch: 20.0,
            roll: 5.0,
        };
        map.apply(&input, &mut params);
        assert_eq!(value(&params, standard::ANGLE_X), 0.0);
        assert_eq!(value(&params, standard::ANGLE_Y), 0.0);
        assert_eq!(value(&params, standard::ANGLE_Z), 5.0);
        assert!(close(value(&params, standard::MOUTH_X), 0.5));

        // 中継用のパラメータがなければ `new` と同じ。
        let model = fixtures::avatar_profile();
        let params = ParameterValues::from_model(&model);
        assert_eq!(
            TrackingMap::for_model(&model, &params),
            TrackingMap::new(model.avatar.as_ref(), &params)
        );
    }

    /// T0325: 作ったときと並びの違う `ParameterValues` に書くときも、同じ ID のパラメータに書く。
    #[test]
    fn applies_to_parameters_in_a_different_layout() {
        let mut model = fixtures::avatar_profile();
        model.avatar.as_mut().unwrap().tracking = vec![over(
            TrackingSource::BlendShape,
            "jawLeft",
            Some(standard::MOUTH_X),
        )];
        let map = TrackingMap::for_model(&model, &ParameterValues::from_model(&model));
        let mut input = input(&[("jawLeft", 0.5), ("eyeBlinkLeft", 0.25), ("jawOpen", 0.75)]);
        input.head = Rotation {
            yaw: 10.0,
            pitch: 20.0,
            roll: 5.0,
        };
        let mut expected = ParameterValues::from_model(&model);
        map.apply(&input, &mut expected);

        model.parameters.reverse();
        let mut reversed = ParameterValues::from_model(&model);
        map.apply(&input, &mut reversed);
        for id in reversed.ids() {
            let id = id.as_str();
            assert_eq!(value(&reversed, id), value(&expected, id), "{id}");
        }
        assert!(close(value(&reversed, standard::MOUTH_X), 0.5));

        // フィルタには書き込む先の並びの添字を渡す。
        let mut filtered = ParameterValues::from_model(&model);
        let angle_x = filtered.index_of(standard::ANGLE_X).unwrap();
        map.apply_where(&input, &mut filtered, |index| index == angle_x);
        for id in filtered.ids() {
            let id = id.as_str();
            let want = if id == standard::ANGLE_X {
                value(&expected, id)
            } else {
                value(&ParameterValues::from_model(&model), id)
            };
            assert_eq!(value(&filtered, id), want, "{id}");
        }
    }

    /// 並びの違う `ParameterValues` に続けて書いても、毎回同じ ID のパラメータに同じ値を書く
    /// (T0181。引き直した添字は控えないので、ロックを持たずに `Sync` のまま)。
    #[test]
    fn writes_to_a_different_layout_every_time() {
        let mut model = fixtures::avatar_profile();
        model.avatar.as_mut().unwrap().tracking = vec![over(
            TrackingSource::BlendShape,
            "jawLeft",
            Some(standard::MOUTH_X),
        )];
        let original = ParameterValues::from_model(&model);
        let map = TrackingMap::for_model(&model, &original);
        let input = input(&[("jawLeft", 0.5), ("eyeBlinkLeft", 0.25)]);

        let mut same = original.clone();
        map.apply(&input, &mut same);

        model.parameters.reverse();
        let mut reversed = ParameterValues::from_model(&model);
        map.apply(&input, &mut reversed);
        let mut again = reversed.clone();
        map.apply(&input, &mut again);
        assert!(close(value(&again, standard::MOUTH_X), 0.5));
        for id in reversed.ids() {
            let id = id.as_str();
            assert_eq!(value(&reversed, id), value(&same, id), "{id}");
            assert_eq!(value(&again, id), value(&same, id), "{id}");
        }
        // 元の並びに戻しても、作ったときの添字で書く。
        let mut back = original.clone();
        map.apply(&input, &mut back);
        assert!(close(value(&back, standard::MOUTH_X), 0.5));

        assert_eq!(map.clone(), map);
        // スレッドをまたいで共有できる。
        fn send_sync<T: Send + Sync>() {}
        send_sync::<TrackingMap>();
    }
}
