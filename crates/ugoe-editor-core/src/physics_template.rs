//! 物理のテンプレート (`Command::ApplyPhysicsTemplate`)。規則は `docs/editor.md#物理のテンプレート`、
//! 物理の意味は `docs/spec/physics.md` (リグの順序と中継・合成の方法)。
//!
//! テンプレートは、標準パラメータの ID を使った定番のリグの組を、物理のコマンドの列にする。
//! [`commands`] が列を作り、`Document::apply` がそれを `Batch` と同じく 1 つの Undo 単位で適用する。
//! 列を作るだけなので、AI や CLI は [`commands`] の結果を確かめてから適用することもできる。

use std::collections::HashSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use ugoe_core::Vec2;
use ugoe_model::deformer::DeformerKind;
use ugoe_model::physics::{
    NormalizationRange, PhysicsBlend, PhysicsInput, PhysicsNormalization, PhysicsOutput,
    PhysicsParticle, PhysicsRig, PhysicsRigId, PhysicsSettings, PhysicsSource, PhysicsTarget,
};
use ugoe_model::standard::{
    ANGLE_X, ANGLE_Y, ANGLE_Z, BODY_ANGLE_X, BODY_ANGLE_Y, BODY_ANGLE_Z, range,
};
use ugoe_model::{DeformerId, Model, ParamId, Parameter};
use ugoe_motion::tracking::BODY_FROM_HEAD;

use crate::command::Command;
use crate::document::EditError;

/// 物理のテンプレートの種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PhysicsTemplate {
    /// 体の追従: 頭の回転 (`AngleX/Y/Z`) に、体の回転 (`BodyAngleX/Y/Z`) が遅れて付いてきて、少し行き過ぎて戻る。
    BodyFollow,
    /// 弾み: 頭の左右の動きを中継パラメータ `BodyBounce` に受け、もう 1 段の振り子で `BodyAngleX` に弾みを足す。
    Bounce,
    /// 首の追従: 頭の傾き (`AngleZ`) と左右 (`AngleX`) で、首の回転デフォーマを遅れて傾ける (動的物理)。
    NeckFollow,
}

/// [`Command::ApplyPhysicsTemplate`] の選択。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, JsonSchema)]
pub struct PhysicsTemplateOptions {
    /// モデルにない標準パラメータを、標準の値域で作るか。`false` なら、それを使うリグを作らない。
    /// 中継パラメータ (`BodyBounce`) はこの選択によらず作る。
    #[serde(default)]
    pub create_missing: bool,
    /// 体の回転に書き込む合成の方法。`add` (既定) はトラッキング・モーションの値に遅れと行き過ぎを足し、
    /// `overwrite` は物理の値で置き換える (体の回転を書かないホスト向け)。
    #[serde(default = "default_blend")]
    pub blend: PhysicsBlend,
    /// 首の追従で傾ける回転デフォーマ。省略すると、ID か名前に `neck` (大文字小文字を問わない) か
    /// `首` を含む最初の回転デフォーマ。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub neck: Option<DeformerId>,
}

fn default_blend() -> PhysicsBlend {
    PhysicsBlend::Add
}

impl Default for PhysicsTemplateOptions {
    fn default() -> Self {
        Self {
            create_missing: false,
            blend: default_blend(),
            neck: None,
        }
    }
}

/// 弾みのテンプレートが作る中継パラメータ。
pub const BOUNCE_RELAY: &str = "BodyBounce";
/// 中継パラメータの値域 (位置の正規化の範囲と同じ)。
const RELAY_RANGE: (f32, f32, f32) = (-10.0, 0.0, 10.0);

/// 位置の正規化の範囲。根元はこの範囲で動く (振り子の長さ [`LENGTH`] と同じ桁)。
const POSITION: NormalizationRange = NormalizationRange {
    min: -10.0,
    default: 0.0,
    max: 10.0,
};
/// 振り子の長さ。
const LENGTH: f32 = 10.0;

/// 振り子の係数 (`docs/spec/physics.md#係数の意味`)。
#[derive(Clone, Copy)]
struct Swing {
    mobility: f32,
    delay: f32,
    acceleration: f32,
}

/// 体の追従: 重く、頭を急に振ると 0.1 秒ほど遅れて付いてきて、1 割ほど行き過ぎ、1 秒ほどで収まる。
const BODY: Swing = Swing {
    mobility: 0.65,
    delay: 0.5,
    acceleration: 1.5,
};
/// 弾みの 1 段目: よく揺れる (少し大げさなくらい)。
const BOUNCE: Swing = Swing {
    mobility: 0.95,
    delay: 0.8,
    acceleration: 1.0,
};
/// 弾みの 2 段目・首: 1 段目の揺れを受けて、少し遅れて揺れる。
const FOLLOW: Swing = Swing {
    mobility: 0.9,
    delay: 0.7,
    acceleration: 1.5,
};

/// 弾みを体の回転に足す倍率。
const BOUNCE_SCALE: f32 = 0.4;
/// 首の追従の、頭の傾きに対する首の傾きの倍率。
const NECK_SCALE: f32 = 0.3;
/// 首の追従の、頭の左右の入力の重み (%)。
const NECK_YAW_WEIGHT: f32 = 50.0;

fn particles(swing: Swing) -> Vec<PhysicsParticle> {
    vec![
        PhysicsParticle {
            position: Vec2::ZERO,
            mobility: 1.0,
            delay: 1.0,
            acceleration: 1.0,
            radius: 0.0,
            collision_radius: 0.0,
        },
        PhysicsParticle {
            position: Vec2::new(0.0, LENGTH),
            mobility: swing.mobility,
            delay: swing.delay,
            acceleration: swing.acceleration,
            radius: LENGTH,
            collision_radius: 0.0,
        },
    ]
}

fn input(parameter: &str, source: PhysicsSource, weight: f32, reflect: bool) -> PhysicsInput {
    PhysicsInput {
        parameter: ParamId::new(parameter),
        source,
        weight,
        reflect,
    }
}

fn output(
    target: PhysicsTarget,
    source: PhysicsSource,
    scale: f32,
    blend: PhysicsBlend,
) -> PhysicsOutput {
    PhysicsOutput {
        target,
        particle: 1,
        source,
        scale,
        weight: 100.0,
        reflect: false,
        blend,
    }
}

fn to_parameter(id: &str) -> PhysicsTarget {
    PhysicsTarget::Parameter {
        id: ParamId::new(id),
    }
}

/// パラメータの値域をそのまま角度 (度) にする正規化の範囲。値 `v` の入力が根元を `v` 度回す。
fn angle_of(model: &Model, id: &str) -> NormalizationRange {
    let (min, default, max) = model
        .parameters
        .iter()
        .find(|p| p.id.as_str() == id)
        .map(|p| (p.min, p.default, p.max))
        .or_else(|| range(id))
        .unwrap_or((POSITION.min, POSITION.default, POSITION.max));
    NormalizationRange { min, default, max }
}

fn rig(
    id: &str,
    name: &str,
    inputs: Vec<PhysicsInput>,
    outputs: Vec<PhysicsOutput>,
    swing: Swing,
    angle: NormalizationRange,
) -> PhysicsRig {
    PhysicsRig {
        id: id.into(),
        name: name.to_owned(),
        inputs,
        outputs,
        particles: particles(swing),
        normalization: PhysicsNormalization {
            position: POSITION,
            angle,
        },
        colliders: Vec::new(),
        frame: None,
    }
}

/// いっしょに作るか、いっしょに作らないリグの組 (例: 弾みの 2 つのリグ)。
struct Unit {
    rigs: Vec<PhysicsRig>,
    /// 使う標準パラメータ (なければ作るか、組ごと作らない)。
    standard: Vec<&'static str>,
}

/// テンプレートの組 (モデルに合わせる前)。
fn units(
    model: &Model,
    template: PhysicsTemplate,
    options: &PhysicsTemplateOptions,
) -> Result<Vec<Unit>, EditError> {
    Ok(match template {
        PhysicsTemplate::BodyFollow => [
            ("X", ANGLE_X, BODY_ANGLE_X),
            ("Y", ANGLE_Y, BODY_ANGLE_Y),
            ("Z", ANGLE_Z, BODY_ANGLE_Z),
        ]
        .into_iter()
        .map(|(axis, head, body)| Unit {
            // 根元を頭の回転の値と同じ角度 (度) だけ回す。静止時の粒子 1 の角度は頭の回転と同じなので、
            // `add` で足すのは「振り子の角度 − 頭の回転」(遅れと行き過ぎ) の 0.3 倍。トラッキングが
            // 体の回転を頭の回転 × 0.3 にしていれば、結果は振り子の角度 × 0.3 (`overwrite` と同じ) になる。
            rigs: vec![rig(
                &format!("BodyFollow{axis}"),
                &format!("体の追従 {axis}"),
                vec![input(head, PhysicsSource::Angle, 100.0, false)],
                vec![output(
                    to_parameter(body),
                    PhysicsSource::Angle,
                    BODY_FROM_HEAD,
                    options.blend,
                )],
                BODY,
                angle_of(model, head),
            )],
            standard: vec![head, body],
        })
        .collect(),
        PhysicsTemplate::Bounce => vec![Unit {
            rigs: vec![
                // 1 段目: 頭の左右で根元を横に動かし、遅れて揺れる振り子の横の位置を中継パラメータに書く
                // (静止時は 0。弾みそのもの)。
                rig(
                    BOUNCE_RELAY,
                    "体の弾み",
                    vec![input(ANGLE_X, PhysicsSource::X, 100.0, false)],
                    vec![output(
                        to_parameter(BOUNCE_RELAY),
                        PhysicsSource::X,
                        1.0,
                        PhysicsBlend::Overwrite,
                    )],
                    BOUNCE,
                    POSITION,
                ),
                // 2 段目: 中継パラメータで根元をもう一度横に動かし、揺れの揺れを体の回転に足す。
                rig(
                    "BodyBounceAngle",
                    "体の弾み → 角度",
                    vec![input(BOUNCE_RELAY, PhysicsSource::X, 100.0, true)],
                    vec![output(
                        to_parameter(BODY_ANGLE_X),
                        PhysicsSource::X,
                        BOUNCE_SCALE,
                        options.blend,
                    )],
                    FOLLOW,
                    POSITION,
                ),
            ],
            standard: vec![ANGLE_X, BODY_ANGLE_X],
        }],
        PhysicsTemplate::NeckFollow => {
            let neck = neck_deformer(model, options.neck.as_ref())?;
            vec![Unit {
                rigs: vec![rig(
                    "NeckFollow",
                    "首の追従",
                    vec![
                        input(ANGLE_Z, PhysicsSource::Angle, 100.0, false),
                        input(ANGLE_X, PhysicsSource::X, NECK_YAW_WEIGHT, false),
                    ],
                    // デフォーマの角度への出力は静止時の値を引かないので、首は頭の傾きの 0.3 倍だけ傾き、
                    // 頭を動かすと遅れて付いてくる。根元は首のデフォーマの原点に取り付く。
                    vec![output(
                        PhysicsTarget::DeformerAngle { deformer: neck },
                        PhysicsSource::Angle,
                        NECK_SCALE,
                        PhysicsBlend::Overwrite,
                    )],
                    FOLLOW,
                    angle_of(model, ANGLE_Z),
                )],
                standard: vec![ANGLE_Z, ANGLE_X],
            }]
        }
    })
}

/// 首の追従で傾ける回転デフォーマを決める。
fn neck_deformer(model: &Model, neck: Option<&DeformerId>) -> Result<DeformerId, EditError> {
    let rotation = |d: &&ugoe_model::Deformer| matches!(d.kind, DeformerKind::Rotation(_));
    if let Some(id) = neck {
        let deformer = model
            .deformers
            .iter()
            .find(|d| &d.id == id)
            .ok_or_else(|| EditError::NotFound {
                kind: "deformer",
                id: id.to_string(),
            })?;
        if !rotation(&deformer) {
            return Err(EditError::DeformerKindMismatch {
                id: id.to_string(),
                expected: "rotation",
            });
        }
        return Ok(id.clone());
    }
    model
        .deformers
        .iter()
        .filter(rotation)
        .find(|d| is_neck_name(d.id.as_str()) || is_neck_name(&d.name))
        .map(|d| d.id.clone())
        .ok_or_else(|| EditError::InvalidPhysics {
            reason: "no neck rotation deformer: select one (its ID or name has no `neck` or `首`)"
                .to_owned(),
        })
}

fn is_neck_name(name: &str) -> bool {
    name.to_lowercase().contains("neck") || name.contains('首')
}

/// 首の追従が既定で選ぶ回転デフォーマ (ID か名前に `neck` か `首` を含む最初のもの)。
pub fn default_neck(model: &Model) -> Option<DeformerId> {
    neck_deformer(model, None).ok()
}

fn standard_parameter(id: &str, internal: bool) -> Parameter {
    let (min, default, max) = if internal {
        RELAY_RANGE
    } else {
        range(id).unwrap_or(RELAY_RANGE)
    };
    Parameter {
        id: ParamId::new(id),
        name: id.to_owned(),
        min,
        max,
        default,
        kind: Default::default(),
        mirror: None,
        group: None,
        internal,
    }
}

fn written(rig: &PhysicsRig) -> impl Iterator<Item = &str> {
    rig.outputs.iter().filter_map(|o| match &o.target {
        PhysicsTarget::Parameter { id } => Some(id.as_str()),
        _ => None,
    })
}

/// テンプレートを適用するコマンドの列を作る。モデルと物理は変えない。
///
/// - ないパラメータは、`options.create_missing` なら `add_parameter` (標準の値域) を先に置き、
///   そうでなければそのパラメータを使う組 (体の追従は軸ごと、弾みは 2 つのリグ) を作らない。
///   作る組が 1 つもなければ [`EditError::InvalidPhysics`] で拒否する。
/// - 中継パラメータ `BodyBounce` がなければ内部用 (`internal`) で作る。
/// - 同じ ID のリグがあれば、並びの位置を保って中身を置き換える (もう一度適用すると初期値に戻る)。
/// - 新しいリグは末尾に足す。ただし、そのリグが書くパラメータを読む既存のリグがあれば、その最初のものの直前に置く
///   (中継が同じステップで伝わるように。`docs/spec/physics.md#リグの順序と中継`)。
pub fn commands(
    model: &Model,
    physics: Option<&PhysicsSettings>,
    template: PhysicsTemplate,
    options: &PhysicsTemplateOptions,
) -> Result<Vec<Command>, EditError> {
    let has = |id: &str| model.parameters.iter().any(|p| p.id.as_str() == id);
    let mut out = Vec::new();
    let mut created: HashSet<&str> = HashSet::new();
    let mut missing: Vec<&str> = Vec::new();
    let mut rigs = Vec::new();
    for unit in units(model, template, options)? {
        let absent: Vec<&str> = unit
            .standard
            .iter()
            .copied()
            .filter(|id| !has(id) && !created.contains(id))
            .collect();
        if !absent.is_empty() && !options.create_missing {
            missing.extend(absent);
            continue;
        }
        for id in absent {
            created.insert(id);
            out.push(Command::AddParameter {
                parameter: standard_parameter(id, false),
            });
        }
        rigs.extend(unit.rigs);
    }
    if rigs.is_empty() {
        missing.dedup();
        return Err(EditError::InvalidPhysics {
            reason: format!(
                "the template needs the parameters {}; allow creating missing parameters",
                missing.join(", ")
            ),
        });
    }
    let relay = rigs.iter().any(|r| written(r).any(|id| id == BOUNCE_RELAY));
    if relay && !has(BOUNCE_RELAY) {
        out.push(Command::AddParameter {
            parameter: standard_parameter(BOUNCE_RELAY, true),
        });
    }

    // 並びを追いながら、置き換え・追加・並べ替えを積む。
    let mut order: Vec<PhysicsRig> = physics.map(|p| p.rigs.clone()).unwrap_or_default();
    let template_ids: HashSet<PhysicsRigId> = rigs.iter().map(|r| r.id.clone()).collect();
    for rig in rigs {
        if let Some(existing) = order.iter_mut().find(|r| r.id == rig.id) {
            out.extend(replace(existing, &rig));
            *existing = rig;
            continue;
        }
        let writes: HashSet<&str> = written(&rig).collect();
        let reader = order.iter().position(|r| {
            !template_ids.contains(&r.id)
                && r.inputs
                    .iter()
                    .any(|i| writes.contains(i.parameter.as_str()))
        });
        let id = rig.id.clone();
        out.push(Command::AddPhysicsRig { rig: rig.clone() });
        match reader {
            Some(index) => {
                out.push(Command::ReorderPhysicsRig {
                    id,
                    before: Some(order[index].id.clone()),
                });
                order.insert(index, rig);
            }
            None => order.push(rig),
        }
    }
    Ok(out)
}

/// 既存のリグ `existing` の中身を `rig` に置き換えるコマンド (ID と並びの位置は保つ)。
fn replace(existing: &PhysicsRig, rig: &PhysicsRig) -> Vec<Command> {
    let id = &rig.id;
    let mut out = Vec::new();
    if existing.name != rig.name {
        out.push(Command::RenamePhysicsRig {
            id: id.clone(),
            name: rig.name.clone(),
        });
    }
    out.push(Command::SetPhysicsInputs {
        rig: id.clone(),
        inputs: rig.inputs.clone(),
    });
    out.push(Command::SetPhysicsOutputs {
        rig: id.clone(),
        outputs: rig.outputs.clone(),
    });
    out.push(Command::SetPhysicsParticles {
        rig: id.clone(),
        particles: rig.particles.clone(),
    });
    out.push(Command::SetPhysicsNormalization {
        rig: id.clone(),
        normalization: rig.normalization,
    });
    if existing.frame.is_some() {
        out.push(Command::SetPhysicsFrame {
            rig: id.clone(),
            frame: None,
        });
    }
    if !existing.colliders.is_empty() {
        out.push(Command::SetPhysicsRigColliders {
            rig: id.clone(),
            colliders: Vec::new(),
        });
    }
    out
}
