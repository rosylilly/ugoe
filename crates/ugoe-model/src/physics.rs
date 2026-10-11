//! 物理演算 (振り子チェーン) の設定データ。アルゴリズムは `ugoe-physics`、仕様は `docs/spec/physics.md`。

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};
use ugoe_core::{Id, Vec2};

use crate::{DeformerId, MeshId, ParamId};

/// 物理リグ ([`PhysicsRig`]) の ID。
pub type PhysicsRigId = Id<PhysicsRig>;
/// 衝突体 ([`PhysicsCollider`]) の ID。
pub type ColliderId = Id<PhysicsCollider>;

/// モデル 1 体分の物理の設定 (`*.physics.json`)。全リグに共通の重力・風・ステップの周波数と、リグ・衝突体の一覧。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PhysicsSettings {
    /// 重力のベクトル。物理空間は y 軸下向きで、既定は `(0, 1)` (真下)。
    /// リグの `acceleration` を掛けて使う。
    #[cfg_attr(feature = "ts", ts(as = "crate::Vec2Repr"))]
    #[cfg_attr(feature = "schema", schemars(with = "crate::Vec2Repr"))]
    pub gravity: Vec2,
    /// 風のベクトル。重力と違い `acceleration` は掛からず、入力の角度でも回さない。
    #[cfg_attr(feature = "ts", ts(as = "crate::Vec2Repr"))]
    #[cfg_attr(feature = "schema", schemars(with = "crate::Vec2Repr"))]
    pub wind: Vec2,
    /// 固定ステップ実行の周波数 (Hz)。
    pub fps: f32,
    /// 物理リグ。この並びの順に処理し、前のリグがパラメータに書いた出力を後ろのリグの入力が読める
    /// (`docs/spec/physics.md#リグの順序と中継`)。
    pub rigs: Vec<PhysicsRig>,
    /// 衝突体。どのリグと衝突するかは `PhysicsRig::colliders` で選ぶ
    /// (`docs/spec/physics.md#衝突体`)。省略時はなし。
    #[serde(default)]
    pub colliders: Vec<PhysicsCollider>,
}

/// 物理リグ。入力 → 振り子チェーン → 出力 の 1 組 (`docs/spec/physics.md#概要`)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PhysicsRig {
    /// リグの ID。
    pub id: PhysicsRigId,
    /// 表示名 (エディタ用。評価には使わない)。
    pub name: String,
    /// チェーンの根元の平行移動と回転を作る入力。
    pub inputs: Vec<PhysicsInput>,
    /// 粒子の位置・角度を書き込む出力。
    pub outputs: Vec<PhysicsOutput>,
    /// 振り子チェーンの粒子。先頭 (0 番) が根元で、根元から順に並ぶ。
    pub particles: Vec<PhysicsParticle>,
    /// 入力のパラメータの値を物理空間の位置・角度に写す範囲。
    pub normalization: PhysicsNormalization,
    /// このリグの粒子と衝突する衝突体 (`PhysicsCollider::id`)。省略時はなし。
    #[serde(default)]
    pub colliders: Vec<ColliderId>,
    /// 重力と風を解釈する座標系の基準のデフォーマ。`None` ならキャンバスの座標系
    /// (`docs/spec/physics.md#重力と風の基準`)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<DeformerId>,
}

/// 物理の入力・出力で使う量。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PhysicsSource {
    /// x 成分。入力では根元の平行移動の x、出力では 1 つ前の粒子からの相対位置の x。
    X,
    /// y 成分。入力では根元の平行移動の y、出力では 1 つ前の粒子からの相対位置の y。
    Y,
    /// 角度 (度、時計回りが正)。入力では根元の回転、出力では 1 つ前の区間からの区間の角度。
    Angle,
    /// 未知の値 (新しい版のファイル)。その入力・出力を無視する。
    /// 書き出せない (`docs/spec/file-formats.md#未知の値の読み込み`)。
    #[serde(other, skip_serializing)]
    #[cfg_attr(feature = "ts", ts(skip))]
    #[cfg_attr(feature = "schema", schemars(skip))]
    Unknown,
}

/// 物理の入力。パラメータの値を正規化し、重みを掛けて根元の姿勢に足す (`docs/spec/physics.md#入力`)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PhysicsInput {
    /// 入力元のパラメータ。存在しなければその入力を無視する。
    pub parameter: ParamId,
    /// パラメータの値を根元のどの量 (平行移動の x / y、回転) に足すか。
    pub source: PhysicsSource,
    /// 0..=100 (%)。
    pub weight: f32,
    /// 正規化した値の符号を反転するか。
    #[serde(default)]
    pub reflect: bool,
}

/// 物理の出力。
///
/// 旧形式 (出力先が `parameter` フィールドのパラメータだけ) も読める。読み込みは手で書く
/// ([`PhysicsOutput::deserialize`])。書き出しは常に `target` を使う。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PhysicsOutput {
    /// 出力先 (`docs/spec/physics.md#出力先`)。
    pub target: PhysicsTarget,
    /// 出力元となる粒子インデックス (その粒子と 1 つ前の粒子の成す角を使う)。
    pub particle: u32,
    /// 粒子のどの量 (相対位置の x / y、区間の角度) を出力するか。
    pub source: PhysicsSource,
    /// 出力する値に掛ける倍率。
    pub scale: f32,
    /// 0..=100 (%)。パラメータへは現在の値とのブレンドの割合、動的物理では加算量に掛ける割合。
    /// 0〜1 にクランプして使う。
    pub weight: f32,
    /// 出力する値の符号を反転するか。
    #[serde(default)]
    pub reflect: bool,
    /// パラメータへの合成の方法 (`docs/spec/physics.md#合成の方法`)。省略時は `overwrite`。
    /// パラメータ以外の出力先では使わない (常に加算量として足す)。
    #[serde(default, skip_serializing_if = "PhysicsBlend::is_overwrite")]
    pub blend: PhysicsBlend,
}

impl PhysicsOutput {
    /// 出力先がパラメータなら、その ID。
    pub fn parameter(&self) -> Option<&ParamId> {
        match &self.target {
            PhysicsTarget::Parameter { id } => Some(id),
            _ => None,
        }
    }
}

impl<'de> Deserialize<'de> for PhysicsOutput {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        /// `target` と旧形式の `parameter` の両方を受け付ける。両方あれば `target` を使う。
        #[derive(Deserialize)]
        struct Raw {
            #[serde(default)]
            target: Option<PhysicsTarget>,
            #[serde(default)]
            parameter: Option<ParamId>,
            particle: u32,
            source: PhysicsSource,
            scale: f32,
            weight: f32,
            #[serde(default)]
            reflect: bool,
            #[serde(default)]
            blend: PhysicsBlend,
        }

        let raw = Raw::deserialize(deserializer)?;
        let target = match (raw.target, raw.parameter) {
            (Some(target), _) => target,
            (None, Some(id)) => PhysicsTarget::Parameter { id },
            (None, None) => return Err(de::Error::missing_field("target")),
        };
        Ok(Self {
            target,
            particle: raw.particle,
            source: raw.source,
            scale: raw.scale,
            weight: raw.weight,
            reflect: raw.reflect,
            blend: raw.blend,
        })
    }
}

/// 物理の出力をパラメータへ合成する方法 (`docs/spec/physics.md#合成の方法`)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PhysicsBlend {
    /// 現在の値を物理の値へ重みの割合で近づける (`param + (value - param) * weight`)。
    #[default]
    Overwrite,
    /// 物理を進める前の値に、物理の値の静止時の値からのずれを重み付きで足す
    /// (`param + (value - rest) * weight`)。トラッキングなどの値に揺れを重ねる。
    Add,
    /// 未知の値 (新しい版のファイル)。`overwrite` として扱う。
    /// 書き出せない (`docs/spec/file-formats.md#未知の値の読み込み`)。
    #[serde(other, skip_serializing)]
    #[cfg_attr(feature = "ts", ts(skip))]
    #[cfg_attr(feature = "schema", schemars(skip))]
    Unknown,
}

impl PhysicsBlend {
    /// フォールバックを適用した値 (`Unknown` は `Overwrite`)。
    pub fn effective(self) -> Self {
        match self {
            Self::Unknown => Self::Overwrite,
            blend => blend,
        }
    }

    /// 既定値 (`Overwrite`) か。書き出しの省略に使う (`Unknown` は省略しない)。
    pub fn is_overwrite(&self) -> bool {
        *self == Self::Overwrite
    }
}

/// 物理の出力先。JSON では `{"type": "deformer_angle", "deformer": "HairFront1"}` と書く。
/// パラメータ以外の出力先は動的物理 (`docs/spec/physics.md#動的物理-揺れ用パラメータのいらない物理`)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum PhysicsTarget {
    /// パラメータに書き込む (従来の出力)。
    Parameter {
        /// 書き込む先のパラメータ。
        id: ParamId,
    },
    /// 回転デフォーマの角度に足す。
    DeformerAngle {
        /// 角度を足す回転デフォーマ。
        deformer: DeformerId,
    },
    /// 回転デフォーマの原点をずらす。
    DeformerOffset {
        /// 原点をずらす回転デフォーマ。
        deformer: DeformerId,
    },
    /// ワープデフォーマの制御点、またはメッシュの頂点を、重み付きで粒子の変位に追従させる。
    Vertices {
        /// 動かす頂点の持ち主 (メッシュまたはワープデフォーマ)。
        owner: VertexOwner,
        /// 頂点 (制御点) ごとの重み (0..=1)。並びと長さは、メッシュなら `Mesh::uvs`、
        /// ワープなら `WarpKeyform::points` と同じ。
        weights: Vec<f32>,
    },
    /// 未知の値 (新しい版のファイル)。その出力を無視する。
    /// 書き出せない (`docs/spec/file-formats.md#未知の値の読み込み`)。
    #[serde(other, skip_serializing)]
    #[cfg_attr(feature = "ts", ts(skip))]
    #[cfg_attr(feature = "schema", schemars(skip))]
    Unknown,
}

/// 頂点への出力の対象。JSON では `{"mesh": "Skirt"}` または `{"warp": "SkirtWarp"}` と書く。
///
/// 外部タグの表現では `#[serde(other)]` が使えないので、読み込みは手で書く
/// ([`VertexOwner::deserialize`])。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum VertexOwner {
    /// メッシュの頂点。
    Mesh(MeshId),
    /// ワープデフォーマの制御点。
    Warp(DeformerId),
    /// 未知の対象 (新しい版のファイル)。その出力を無視する。
    /// 書き出せない (`docs/spec/file-formats.md#未知の値の読み込み`)。
    #[serde(skip_serializing)]
    #[cfg_attr(feature = "ts", ts(skip))]
    #[cfg_attr(feature = "schema", schemars(skip))]
    Unknown,
}

impl<'de> Deserialize<'de> for VertexOwner {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = VertexOwner;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a single-key object such as {\"mesh\": id} or {\"warp\": id}")
            }

            /// 中身の無い未知の種類 (新しい版のユニットのバリアント) は文字列で書かれる。
            /// 知っている種類は中身 (ID) が要るのでエラーにする。
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                match v {
                    "mesh" | "warp" => Err(E::invalid_type(de::Unexpected::Str(v), &self)),
                    _ => Ok(VertexOwner::Unknown),
                }
            }

            fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let Some(key) = map.next_key::<String>()? else {
                    return Err(de::Error::invalid_length(0, &self));
                };
                let owner = match key.as_str() {
                    "mesh" => VertexOwner::Mesh(map.next_value()?),
                    "warp" => VertexOwner::Warp(map.next_value()?),
                    _ => {
                        map.next_value::<de::IgnoredAny>()?;
                        VertexOwner::Unknown
                    }
                };
                if map.next_key::<de::IgnoredAny>()?.is_some() {
                    return Err(de::Error::invalid_length(2, &self));
                }
                Ok(owner)
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

/// 衝突体 (`docs/spec/physics.md#衝突体`)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PhysicsCollider {
    /// 衝突体の ID。リグは [`PhysicsRig::colliders`] でこの ID を選ぶ。
    pub id: ColliderId,
    /// 表示名 (エディタ用。評価には使わない)。
    pub name: String,
    /// 取り付け先のデフォーマ。形状の位置はこのデフォーマの局所座標系で表し、
    /// そのワールド変換で動く。`None` ならキャンバスに固定する。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attach: Option<DeformerId>,
    /// 形状。位置は `attach` のデフォーマの局所座標系 (無ければキャンバス座標) で表す。
    pub shape: ColliderShape,
}

/// 衝突体の形状。半径はキャンバスのピクセル単位。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ColliderShape {
    /// `center` から `radius` 以内の円の領域。
    Circle {
        /// 円の中心 (`attach` の局所座標系、無ければキャンバス座標)。
        #[cfg_attr(feature = "ts", ts(as = "crate::Vec2Repr"))]
        #[cfg_attr(feature = "schema", schemars(with = "crate::Vec2Repr"))]
        center: Vec2,
        /// 円の半径 (ピクセル)。
        radius: f32,
    },
    /// 線分 `a`–`b` から `radius` 以内の領域。
    Capsule {
        /// 線分の一端 (`attach` の局所座標系、無ければキャンバス座標)。
        #[cfg_attr(feature = "ts", ts(as = "crate::Vec2Repr"))]
        #[cfg_attr(feature = "schema", schemars(with = "crate::Vec2Repr"))]
        a: Vec2,
        /// 線分のもう一端 (`a` と同じ座標系)。
        #[cfg_attr(feature = "ts", ts(as = "crate::Vec2Repr"))]
        #[cfg_attr(feature = "schema", schemars(with = "crate::Vec2Repr"))]
        b: Vec2,
        /// 線分からの距離で表す半径 (ピクセル)。
        radius: f32,
    },
    /// 未知の値 (新しい版のファイル)。その衝突体を無視する。
    /// 書き出せない (`docs/spec/file-formats.md#未知の値の読み込み`)。
    #[serde(other, skip_serializing)]
    #[cfg_attr(feature = "ts", ts(skip))]
    #[cfg_attr(feature = "schema", schemars(skip))]
    Unknown,
}

/// 振り子チェーンの粒子 1 つ。係数の意味は `docs/spec/physics.md#係数の意味`。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PhysicsParticle {
    /// 初期位置 (根元からの相対)。
    #[cfg_attr(feature = "ts", ts(as = "crate::Vec2Repr"))]
    #[cfg_attr(feature = "schema", schemars(with = "crate::Vec2Repr"))]
    pub position: Vec2,
    /// 基準フレーム (1/30 秒) あたりに残す速度の割合 (0..=1)。1 で減衰なし、小さいほど揺れがすぐ止まる。
    pub mobility: f32,
    /// 時間の進み方の倍率。1 で等速、大きいほど入力に機敏に反応して速く揺れる。
    pub delay: f32,
    /// 重力の倍率。大きいほど元の向きに戻ろうとする力が強く、揺れが速く収まる。風には掛からない。
    pub acceleration: f32,
    /// 1 つ前の粒子との距離 (区間の長さ)。衝突には使わない。
    pub radius: f32,
    /// 衝突用の半径。衝突体からこの半径を含めて押し出す。省略時は 0。
    #[serde(default)]
    pub collision_radius: f32,
}

/// 入力のパラメータの値を物理空間の量に写す範囲 (`docs/spec/physics.md#入力`)。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PhysicsNormalization {
    /// 平行移動 (`source` が `x` / `y` の入力) の範囲。
    pub position: NormalizationRange,
    /// 角度 (`source` が `angle` の入力) の範囲 (度)。
    pub angle: NormalizationRange,
}

/// 正規化の写し先の範囲。パラメータの `min` / `default` / `max` をそれぞれここの値に写し、
/// その間は default の両側で別々に線形補間する。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct NormalizationRange {
    /// パラメータの `min` を写した値。
    pub min: f32,
    /// パラメータの `default` を写した値。
    pub default: f32,
    /// パラメータの `max` を写した値。
    pub max: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use alloc::vec;

    use serde_json::json;

    fn rig_json(outputs: serde_json::Value) -> serde_json::Value {
        json!({
            "gravity": [0.0, 1.0],
            "wind": [0.0, 0.0],
            "fps": 60.0,
            "rigs": [{
                "id": "Hair",
                "name": "Hair",
                "inputs": [],
                "outputs": outputs,
                "particles": [
                    { "position": [0.0, 0.0], "mobility": 1.0, "delay": 1.0,
                      "acceleration": 1.0, "radius": 0.0 },
                ],
                "normalization": {
                    "position": { "min": -1.0, "default": 0.0, "max": 1.0 },
                    "angle": { "min": -1.0, "default": 0.0, "max": 1.0 },
                },
            }],
        })
    }

    /// 動的物理のない旧形式 (パラメータへの出力だけ) を読める。
    #[test]
    fn reads_parameter_only_data() {
        let physics: PhysicsSettings = serde_json::from_value(rig_json(json!([{
            "parameter": "HairFront",
            "particle": 1,
            "source": "angle",
            "scale": 1.5,
            "weight": 100.0,
        }])))
        .unwrap();
        let rig = &physics.rigs[0];
        assert_eq!(
            rig.outputs[0].target,
            PhysicsTarget::Parameter {
                id: "HairFront".into()
            }
        );
        assert_eq!(rig.outputs[0].parameter(), Some(&"HairFront".into()));
        assert!(physics.colliders.is_empty());
        assert!(rig.colliders.is_empty());
        assert_eq!(rig.frame, None);
        assert_eq!(rig.particles[0].collision_radius, 0.0);
        assert_eq!(rig.outputs[0].blend, PhysicsBlend::Overwrite);

        // 書き出しは新しい形式になり、読み直すと同じ値になる。
        let value = serde_json::to_value(&physics).unwrap();
        let output = &value["rigs"][0]["outputs"][0];
        assert_eq!(
            output["target"],
            json!({ "type": "parameter", "id": "HairFront" })
        );
        assert!(output.get("parameter").is_none());
        assert!(output.get("blend").is_none());
        let back: PhysicsSettings = serde_json::from_value(value).unwrap();
        assert_eq!(back, physics);
    }

    #[test]
    fn output_without_target_is_an_error() {
        let result: Result<PhysicsSettings, _> = serde_json::from_value(rig_json(json!([{
            "particle": 1,
            "source": "angle",
            "scale": 1.0,
            "weight": 100.0,
        }])));
        assert!(result.unwrap_err().to_string().contains("target"));
    }

    fn dynamic() -> PhysicsSettings {
        let output = |target| PhysicsOutput {
            target,
            particle: 1,
            source: PhysicsSource::Angle,
            scale: 1.0,
            weight: 100.0,
            reflect: false,
            blend: PhysicsBlend::Overwrite,
        };
        let range = NormalizationRange {
            min: -1.0,
            default: 0.0,
            max: 1.0,
        };
        PhysicsSettings {
            gravity: Vec2::new(0.0, 1.0),
            wind: Vec2::ZERO,
            fps: 60.0,
            rigs: vec![PhysicsRig {
                id: "Hair".into(),
                name: "Hair".into(),
                inputs: vec![],
                outputs: vec![
                    output(PhysicsTarget::DeformerAngle {
                        deformer: "Hair1".into(),
                    }),
                    output(PhysicsTarget::DeformerOffset {
                        deformer: "Hair1".into(),
                    }),
                    output(PhysicsTarget::Vertices {
                        owner: VertexOwner::Mesh("Skirt".into()),
                        weights: vec![0.0, 0.5, 1.0],
                    }),
                    output(PhysicsTarget::Vertices {
                        owner: VertexOwner::Warp("SkirtWarp".into()),
                        weights: vec![1.0; 4],
                    }),
                ],
                particles: vec![PhysicsParticle {
                    position: Vec2::ZERO,
                    mobility: 1.0,
                    delay: 1.0,
                    acceleration: 1.0,
                    radius: 0.0,
                    collision_radius: 4.0,
                }],
                normalization: PhysicsNormalization {
                    position: range,
                    angle: range,
                },
                colliders: vec!["Shoulder".into(), "Head".into()],
                frame: Some("Head".into()),
            }],
            colliders: vec![
                PhysicsCollider {
                    id: "Shoulder".into(),
                    name: "Shoulder".into(),
                    attach: Some("Body".into()),
                    shape: ColliderShape::Capsule {
                        a: Vec2::new(-50.0, 0.0),
                        b: Vec2::new(50.0, 0.0),
                        radius: 10.0,
                    },
                },
                PhysicsCollider {
                    id: "Head".into(),
                    name: "Head".into(),
                    attach: None,
                    shape: ColliderShape::Circle {
                        center: Vec2::new(0.0, -100.0),
                        radius: 80.0,
                    },
                },
            ],
        }
    }

    #[test]
    fn dynamic_physics_round_trips() {
        let physics = dynamic();
        let value = serde_json::to_value(&physics).unwrap();
        assert_eq!(
            value["rigs"][0]["outputs"][0]["target"],
            json!({ "type": "deformer_angle", "deformer": "Hair1" })
        );
        assert_eq!(
            value["rigs"][0]["outputs"][2]["target"],
            json!({ "type": "vertices", "owner": { "mesh": "Skirt" }, "weights": [0.0, 0.5, 1.0] })
        );
        assert_eq!(
            value["colliders"][1]["shape"],
            json!({ "type": "circle", "center": [0.0, -100.0], "radius": 80.0 })
        );
        assert_eq!(value["colliders"][1].get("attach"), None);
        let back: PhysicsSettings = serde_json::from_value(value).unwrap();
        assert_eq!(back, physics);
    }

    #[test]
    fn unknown_targets_and_shapes_are_read_as_unknown() {
        let mut value = serde_json::to_value(dynamic()).unwrap();
        value["rigs"][0]["outputs"][0]["target"] = json!({ "type": "spring", "k": 2.0 });
        value["colliders"][0]["shape"] = json!({ "type": "polygon", "points": [[0, 0]] });
        let physics: PhysicsSettings = serde_json::from_value(value).unwrap();
        assert_eq!(physics.rigs[0].outputs[0].target, PhysicsTarget::Unknown);
        assert_eq!(physics.rigs[0].outputs[0].parameter(), None);
        assert_eq!(physics.colliders[0].shape, ColliderShape::Unknown);
        assert!(serde_json::to_string(&physics).is_err());
    }

    /// 新しい版で増えた頂点への出力の対象 (`{"bone": ..}` など) と、中身の無い未知の種類。
    fn future_owners() -> serde_json::Value {
        let mut value = serde_json::to_value(dynamic()).unwrap();
        let outputs = &mut value["rigs"][0]["outputs"];
        outputs[2]["target"]["owner"] = json!({ "bone": { "id": "Spine", "segment": 2 } });
        outputs[3]["target"]["owner"] = json!("all");
        value
    }

    fn assert_future_owners(physics: &PhysicsSettings) {
        let owner = |index: usize| match &physics.rigs[0].outputs[index].target {
            PhysicsTarget::Vertices { owner, .. } => owner.clone(),
            target => panic!("not vertices: {target:?}"),
        };
        assert_eq!(owner(2), VertexOwner::Unknown);
        assert_eq!(owner(3), VertexOwner::Unknown);
        // ほかの出力はそのまま読める。
        assert_eq!(physics.rigs[0].outputs[..2], dynamic().rigs[0].outputs[..2]);
        assert_eq!(
            crate::unknown::in_physics(physics),
            [
                "rigs[Hair].outputs[2].target.owner",
                "rigs[Hair].outputs[3].target.owner",
            ]
        );
        assert!(serde_json::to_string(physics).is_err());
    }

    #[test]
    fn unknown_vertex_owners_are_read_from_json() {
        let physics: PhysicsSettings = serde_json::from_value(future_owners()).unwrap();
        assert_future_owners(&physics);
    }

    #[test]
    fn unknown_vertex_owners_are_read_from_msgpack() {
        // `.ugoe` のバンドルと同じく、フィールド名付きの MessagePack で書く。
        let bytes = rmp_serde::to_vec_named(&future_owners()).unwrap();
        let physics: PhysicsSettings = rmp_serde::from_slice(&bytes).unwrap();
        assert_future_owners(&physics);

        // 知っている対象は MessagePack でも往復する。
        let bytes = rmp_serde::to_vec_named(&dynamic()).unwrap();
        let back: PhysicsSettings = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(back, dynamic());
    }

    #[test]
    fn malformed_vertex_owners_are_errors() {
        for owner in [
            json!({ "mesh": 1 }),
            json!({ "warp": null }),
            json!("mesh"),
            json!({}),
            json!({ "mesh": "A", "warp": "B" }),
            json!({ "bone": "A", "mesh": "B" }),
            json!(["mesh", "A"]),
        ] {
            assert!(
                serde_json::from_value::<VertexOwner>(owner.clone()).is_err(),
                "{owner}"
            );
        }
        assert_eq!(
            serde_json::from_value::<VertexOwner>(json!({ "warp": "W" })).unwrap(),
            VertexOwner::Warp("W".into())
        );
    }

    /// 合成の方法: `add` は読み書きでき、未知の値は `Unknown` (`overwrite` として扱い、書き出せない)。
    #[test]
    fn blend_round_trips_and_unknown_falls_back() {
        let mut physics = dynamic();
        physics.rigs[0].outputs[0].blend = PhysicsBlend::Add;
        let mut value = serde_json::to_value(&physics).unwrap();
        assert_eq!(value["rigs"][0]["outputs"][0]["blend"], json!("add"));
        assert!(value["rigs"][0]["outputs"][1].get("blend").is_none());
        let back: PhysicsSettings = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(back, physics);

        value["rigs"][0]["outputs"][0]["blend"] = json!("multiply");
        let physics: PhysicsSettings = serde_json::from_value(value).unwrap();
        let blend = physics.rigs[0].outputs[0].blend;
        assert_eq!(blend, PhysicsBlend::Unknown);
        assert_eq!(blend.effective(), PhysicsBlend::Overwrite);
        assert!(serde_json::to_string(&physics).is_err());
    }
}
