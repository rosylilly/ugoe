//! 物理演算 (振り子チェーン) の設定データ。アルゴリズムは `ugoe-physics`、仕様は `docs/spec/physics.md`。

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};
use ugoe_core::Vec2;

use crate::{DeformerId, MeshId, ParamId};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PhysicsSettings {
    #[cfg_attr(feature = "ts", ts(as = "crate::Vec2Repr"))]
    #[cfg_attr(feature = "schema", schemars(with = "crate::Vec2Repr"))]
    pub gravity: Vec2,
    #[cfg_attr(feature = "ts", ts(as = "crate::Vec2Repr"))]
    #[cfg_attr(feature = "schema", schemars(with = "crate::Vec2Repr"))]
    pub wind: Vec2,
    /// 固定ステップ実行の周波数 (Hz)。
    pub fps: f32,
    pub rigs: Vec<PhysicsRig>,
    /// 衝突体。どのリグと衝突するかは `PhysicsRig::colliders` で選ぶ
    /// (`docs/spec/physics.md#衝突体`)。省略時はなし。
    #[serde(default)]
    pub colliders: Vec<PhysicsCollider>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PhysicsRig {
    pub id: String,
    pub name: String,
    pub inputs: Vec<PhysicsInput>,
    pub outputs: Vec<PhysicsOutput>,
    pub particles: Vec<PhysicsParticle>,
    pub normalization: PhysicsNormalization,
    /// このリグの粒子と衝突する衝突体 (`PhysicsCollider::id`)。省略時はなし。
    #[serde(default)]
    pub colliders: Vec<String>,
    /// 重力と風を解釈する座標系の基準のデフォーマ。`None` ならキャンバスの座標系
    /// (`docs/spec/physics.md#重力と風の基準`)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<DeformerId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum PhysicsSource {
    X,
    Y,
    Angle,
    /// 未知の値 (新しい版のファイル)。その入力・出力を無視する。
    /// 書き出せない (`docs/spec/file-formats.md#未知の値の読み込み`)。
    #[serde(other, skip_serializing)]
    #[cfg_attr(feature = "ts", ts(skip))]
    #[cfg_attr(feature = "schema", schemars(skip))]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PhysicsInput {
    pub parameter: ParamId,
    pub source: PhysicsSource,
    /// 0..=100 (%)。
    pub weight: f32,
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
    pub source: PhysicsSource,
    pub scale: f32,
    pub weight: f32,
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
pub enum PhysicsTarget {
    /// パラメータに書き込む (従来の出力)。
    Parameter { id: ParamId },
    /// 回転デフォーマの角度に足す。
    DeformerAngle { deformer: DeformerId },
    /// 回転デフォーマの原点をずらす。
    DeformerOffset { deformer: DeformerId },
    /// ワープデフォーマの制御点、またはメッシュの頂点を、重み付きで粒子の変位に追従させる。
    Vertices {
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum VertexOwner {
    /// メッシュの頂点。
    Mesh(MeshId),
    /// ワープデフォーマの制御点。
    Warp(DeformerId),
}

/// 衝突体 (`docs/spec/physics.md#衝突体`)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PhysicsCollider {
    pub id: String,
    pub name: String,
    /// 取り付け先のデフォーマ。形状の位置はこのデフォーマの局所座標系で表し、
    /// そのワールド変換で動く。`None` ならキャンバスに固定する。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attach: Option<DeformerId>,
    pub shape: ColliderShape,
}

/// 衝突体の形状。半径はキャンバスのピクセル単位。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ColliderShape {
    Circle {
        #[cfg_attr(feature = "ts", ts(as = "crate::Vec2Repr"))]
        #[cfg_attr(feature = "schema", schemars(with = "crate::Vec2Repr"))]
        center: Vec2,
        radius: f32,
    },
    /// 線分 `a`–`b` から `radius` 以内の領域。
    Capsule {
        #[cfg_attr(feature = "ts", ts(as = "crate::Vec2Repr"))]
        #[cfg_attr(feature = "schema", schemars(with = "crate::Vec2Repr"))]
        a: Vec2,
        #[cfg_attr(feature = "ts", ts(as = "crate::Vec2Repr"))]
        #[cfg_attr(feature = "schema", schemars(with = "crate::Vec2Repr"))]
        b: Vec2,
        radius: f32,
    },
    /// 未知の値 (新しい版のファイル)。その衝突体を無視する。
    /// 書き出せない (`docs/spec/file-formats.md#未知の値の読み込み`)。
    #[serde(other, skip_serializing)]
    #[cfg_attr(feature = "ts", ts(skip))]
    #[cfg_attr(feature = "schema", schemars(skip))]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PhysicsParticle {
    /// 初期位置 (根元からの相対)。
    #[cfg_attr(feature = "ts", ts(as = "crate::Vec2Repr"))]
    #[cfg_attr(feature = "schema", schemars(with = "crate::Vec2Repr"))]
    pub position: Vec2,
    pub mobility: f32,
    pub delay: f32,
    pub acceleration: f32,
    /// 1 つ前の粒子との距離 (区間の長さ)。衝突には使わない。
    pub radius: f32,
    /// 衝突用の半径。衝突体からこの半径を含めて押し出す。省略時は 0。
    #[serde(default)]
    pub collision_radius: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PhysicsNormalization {
    pub position: NormalizationRange,
    pub angle: NormalizationRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct NormalizationRange {
    pub min: f32,
    pub default: f32,
    pub max: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

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
