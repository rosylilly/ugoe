//! 物理のテンプレート (`physics_template.rs`) のテスト。
//!
//! 動きの検査は、ランタイム SDK の `Puppet` にトラッキング (頭の回転。体の回転はないので、
//! `BodyAngle*` は頭の回転 × 0.3) を与えて行う (`docs/spec/avatar-profile.md#頭と体`)。

use ugoe::Puppet;
use ugoe::motion::tracking::{BODY_FROM_HEAD, Rotation, TrackingInput};
use ugoe_model::Model;
use ugoe_model::fixtures;
use ugoe_model::physics::{PhysicsBlend, PhysicsSettings, PhysicsSource, PhysicsTarget};
use ugoe_model::standard::{
    ANGLE_X, ANGLE_Y, ANGLE_Z, BODY_ANGLE_X, BODY_ANGLE_Y, BODY_ANGLE_Z, range,
};

use crate::physics_template::{BOUNCE_RELAY, commands};
use crate::{Command, Document, EditError, PhysicsTemplate, PhysicsTemplateOptions};

const FPS: f32 = 60.0;
/// 頭を振る先の `AngleX`。
const YAW: f32 = 20.0;

fn template(template: PhysicsTemplate, options: PhysicsTemplateOptions) -> Command {
    Command::ApplyPhysicsTemplate { template, options }
}

fn create_missing() -> PhysicsTemplateOptions {
    PhysicsTemplateOptions {
        create_missing: true,
        ..PhysicsTemplateOptions::default()
    }
}

fn rig_ids(doc: &Document) -> Vec<&str> {
    doc.physics()
        .map(|p| p.rigs.iter().map(|r| r.id.as_str()).collect())
        .unwrap_or_default()
}

fn param_ids(model: &Model) -> Vec<&str> {
    model.parameters.iter().map(|p| p.id.as_str()).collect()
}

#[test]
fn command_json_uses_snake_case() {
    let json = serde_json::to_value(template(
        PhysicsTemplate::BodyFollow,
        PhysicsTemplateOptions::default(),
    ))
    .unwrap();
    assert_eq!(json["op"], "apply_physics_template");
    assert_eq!(json["template"], "body_follow");
    assert_eq!(json["options"]["blend"], "add");
    // `options` は省略できる (既定: 作らない・加算)。
    let parsed: Command = serde_json::from_value(serde_json::json!({
        "op": "apply_physics_template",
        "template": "neck_follow",
    }))
    .unwrap();
    assert_eq!(
        parsed,
        template(
            PhysicsTemplate::NeckFollow,
            PhysicsTemplateOptions::default()
        )
    );
}

/// ないパラメータを作らなければ、それを使う軸は飛ばす。適用は 1 回の Undo で戻る。
#[test]
fn body_follow_skips_missing_axes_and_undoes_in_one_step() {
    let mut doc = Document::new(fixtures::physics_additive());
    let before = doc.model().clone();
    doc.apply(template(
        PhysicsTemplate::BodyFollow,
        PhysicsTemplateOptions::default(),
    ))
    .unwrap();
    assert_eq!(rig_ids(&doc), ["BodyFollowX"]);
    let rig = &doc.physics().unwrap().rigs[0];
    assert_eq!(rig.inputs[0].parameter.as_str(), ANGLE_X);
    assert_eq!(rig.inputs[0].source, PhysicsSource::Angle);
    // 角度の正規化は `AngleX` の値域そのもの (値 = 根元の角度 (度))。
    assert_eq!(
        (rig.normalization.angle.min, rig.normalization.angle.max),
        (-30.0, 30.0)
    );
    let out = &rig.outputs[0];
    assert_eq!(
        out.target,
        PhysicsTarget::Parameter {
            id: BODY_ANGLE_X.into()
        }
    );
    assert_eq!((out.blend, out.scale), (PhysicsBlend::Add, BODY_FROM_HEAD));
    assert_eq!(param_ids(doc.model()), param_ids(&before));

    assert!(doc.undo());
    assert!(doc.physics().is_none());
    assert_eq!(doc.model(), &before);
    assert!(!doc.undo());
}

/// 作ると選べば、標準の値域でパラメータを足してから 3 軸すべてのリグを作る。Undo でパラメータも消える。
#[test]
fn body_follow_creates_missing_standard_parameters() {
    let mut doc = Document::new(fixtures::physics_additive());
    doc.apply(template(PhysicsTemplate::BodyFollow, create_missing()))
        .unwrap();
    assert_eq!(rig_ids(&doc), ["BodyFollowX", "BodyFollowY", "BodyFollowZ"]);
    for id in [ANGLE_Y, ANGLE_Z, BODY_ANGLE_Y, BODY_ANGLE_Z] {
        let p = doc.model().parameters.iter().find(|p| p.id == id).unwrap();
        assert_eq!(Some((p.min, p.default, p.max)), range(id), "{id}");
        assert!(!p.internal);
    }
    assert!(doc.undo());
    assert_eq!(param_ids(doc.model()), [ANGLE_X, BODY_ANGLE_X]);
}

/// 作れるリグが 1 つもなければ拒否し、何も変えない。
#[test]
fn template_without_any_parameter_is_rejected() {
    let mut doc = Document::new(fixtures::single_quad());
    let err = doc
        .apply(template(
            PhysicsTemplate::Bounce,
            PhysicsTemplateOptions::default(),
        ))
        .unwrap_err();
    assert!(
        matches!(&err, EditError::InvalidPhysics { reason } if reason.contains(ANGLE_X)),
        "{err}"
    );
    assert!(doc.physics().is_none());
    assert!(!doc.undo());
}

/// 弾みは、中継パラメータ (内部用) を作り、それを書くリグ → 読むリグの順に並べる。
#[test]
fn bounce_creates_an_internal_relay_in_order() {
    let mut doc = Document::new(fixtures::physics_additive());
    doc.apply(template(
        PhysicsTemplate::Bounce,
        PhysicsTemplateOptions::default(),
    ))
    .unwrap();
    assert_eq!(rig_ids(&doc), [BOUNCE_RELAY, "BodyBounceAngle"]);
    let relay = doc
        .model()
        .parameters
        .iter()
        .find(|p| p.id == BOUNCE_RELAY)
        .unwrap();
    assert!(relay.internal);
    let mut puppet = Puppet::new(doc.model().clone()).unwrap();
    puppet.set_physics(doc.physics().cloned()).unwrap();
    assert!(puppet.physics().unwrap().has_relay());
    assert!(!puppet.physics().unwrap().is_dynamic());
}

/// もう一度適用すると、同じ ID のリグを並びの位置を保って置き換える (増えない)。
#[test]
fn reapplying_replaces_rigs_in_place() {
    let mut doc = Document::new(fixtures::physics_additive());
    doc.apply(template(
        PhysicsTemplate::Bounce,
        PhysicsTemplateOptions::default(),
    ))
    .unwrap();
    doc.apply(template(
        PhysicsTemplate::BodyFollow,
        PhysicsTemplateOptions::default(),
    ))
    .unwrap();
    // 手で変えたリグを、テンプレートで初期値に戻す。
    let mut particles = doc.physics().unwrap().rigs[0].particles.clone();
    particles[1].mobility = 0.1;
    doc.apply(Command::SetPhysicsParticles {
        rig: BOUNCE_RELAY.into(),
        particles,
    })
    .unwrap();
    let applied = doc.physics().cloned();
    doc.apply(template(
        PhysicsTemplate::Bounce,
        PhysicsTemplateOptions::default(),
    ))
    .unwrap();
    assert_eq!(
        rig_ids(&doc),
        [BOUNCE_RELAY, "BodyBounceAngle", "BodyFollowX"]
    );
    assert_eq!(doc.physics().unwrap().rigs[0].particles[1].mobility, 0.95);
    assert!(doc.undo());
    assert_eq!(doc.physics().cloned(), applied);
}

/// 新しいリグが書くパラメータを既存のリグが読むなら、その直前に置く (同じステップで伝わる)。
#[test]
fn new_rigs_go_before_existing_readers() {
    // `physics_relay_settings` の先頭のリグ `BodyFollow` は `BodyAngleX` を読む。
    let mut doc = Document::new(fixtures::physics_relay())
        .with_physics(Some(fixtures::physics_relay_settings()));
    doc.apply(template(PhysicsTemplate::BodyFollow, create_missing()))
        .unwrap();
    assert_eq!(
        rig_ids(&doc),
        [
            "BodyFollowX",
            "BodyFollow",
            "BodyBounce",
            "BodyAngle",
            "BodyFollowY",
            "BodyFollowZ"
        ]
    );
    let list = commands(
        doc.model(),
        doc.physics(),
        PhysicsTemplate::BodyFollow,
        &create_missing(),
    )
    .unwrap();
    // 適用済みなら、置き換えだけでパラメータもリグも増やさない。
    assert!(list.iter().all(|c| !matches!(
        c,
        Command::AddParameter { .. } | Command::AddPhysicsRig { .. }
    )));
}

/// トラッキングで頭を振り、`frames` フレーム分の `(BodyAngleX, BodyBounce)` を返す。
/// 10 フレーム正面を向いてから `AngleX` を [`YAW`] にする。
fn head_turn(model: &Model, physics: Option<&PhysicsSettings>, frames: usize) -> Vec<(f32, f32)> {
    let mut puppet = Puppet::new(model.clone()).unwrap();
    puppet.set_physics(physics.cloned()).unwrap();
    let params = puppet.instance().parameters();
    let body = params.index_of(BODY_ANGLE_X).unwrap();
    let relay = params.index_of(BOUNCE_RELAY);
    (0..10 + frames)
        .map(|frame| {
            let yaw = if frame < 10 { 0.0 } else { YAW };
            puppet.set_tracking(Some(TrackingInput {
                head: Rotation {
                    yaw,
                    ..Rotation::default()
                },
                ..TrackingInput::default()
            }));
            puppet.update(1.0 / FPS);
            let params = puppet.instance().parameters();
            (
                params.get(body).unwrap(),
                relay.and_then(|r| params.get(r)).unwrap_or_default(),
            )
        })
        .skip(10)
        .collect()
}

fn applied(options: PhysicsTemplateOptions, templates: &[PhysicsTemplate]) -> Document {
    let mut doc = Document::new(fixtures::physics_additive());
    for &t in templates {
        doc.apply(template(t, options.clone())).unwrap();
    }
    doc
}

/// 体の追従: 頭を振ると、体は遅れて付いてきて、少し行き過ぎてから、トラッキングの値 (頭 × 0.3) に落ち着く。
#[test]
fn body_follows_the_head_late_and_settles() {
    let doc = applied(
        PhysicsTemplateOptions::default(),
        &[PhysicsTemplate::BodyFollow],
    );
    let target = YAW * BODY_FROM_HEAD;
    let trace: Vec<f32> = head_turn(doc.model(), doc.physics(), 180)
        .into_iter()
        .map(|(body, _)| body)
        .collect();
    let without: Vec<f32> = head_turn(doc.model(), None, 180)
        .into_iter()
        .map(|(body, _)| body)
        .collect();
    // 物理がなければ、体はすぐにトラッキングの値になる。
    assert!((without[0] - target).abs() < 1e-4, "{}", without[0]);
    // 遅れ: 頭が動いた直後は、体はまだ半分も動いていない。ただし最初のフレームは、物理の出力が
    // 1 ステップ遅れる (`docs/spec/physics.md#時間の進め方`) ので、足す量がまだ 0 でトラッキングの値のまま。
    assert!((trace[0] - target).abs() < 1e-4, "first frame {}", trace[0]);
    assert!(trace[1] < target * 0.5, "second frame {}", trace[1]);
    let late = trace
        .iter()
        .skip(1)
        .position(|v| *v > target * 0.9)
        .unwrap();
    assert!((4..30).contains(&late), "reached 90% at frame {late}");
    // 少し (値域に届かない程度に) 行き過ぎてから戻る。
    let max = trace.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    assert!(max > target + 0.3 && max < target * 1.3, "overshoot {max}");
    // 2 秒ほどで収まる。
    for (frame, v) in trace.iter().enumerate().skip(120) {
        assert!((v - target).abs() < 0.1, "frame {frame}: {v}");
    }

    // トラッキングが体の回転を頭 × 0.3 にしているなら、加算と上書きは同じ動き (振り子の角度 × 0.3)。
    let overwrite = applied(
        PhysicsTemplateOptions {
            blend: PhysicsBlend::Overwrite,
            ..PhysicsTemplateOptions::default()
        },
        &[PhysicsTemplate::BodyFollow],
    );
    let over = head_turn(overwrite.model(), overwrite.physics(), 180);
    // 最初のフレームだけは、上書きは 1 ステップ前の物理の値 (0) を、加算は基準値をそのまま書く。
    for (frame, (a, (o, _))) in trace.iter().zip(&over).enumerate().skip(1) {
        assert!((a - o).abs() < 0.05, "frame {frame}: add {a} overwrite {o}");
    }
}

/// 弾み: 中継パラメータが揺れ、体は行き過ぎと戻りを何度か繰り返してから (弾んでから) 落ち着く。
#[test]
fn bounce_overshoots_and_settles() {
    let doc = applied(
        PhysicsTemplateOptions::default(),
        &[PhysicsTemplate::Bounce],
    );
    let target = YAW * BODY_FROM_HEAD;
    let trace = head_turn(doc.model(), doc.physics(), 300);
    let relay_peak = trace.iter().map(|(_, r)| r.abs()).fold(0.0, f32::max);
    assert!(relay_peak > 1.0, "relay {relay_peak}");
    // 体はトラッキングの値の上と下を行き来する。
    let body: Vec<f32> = trace.iter().map(|(b, _)| b - target).collect();
    let above = body[..120]
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, f32::max);
    let below = body[..120].iter().copied().fold(f32::INFINITY, f32::min);
    assert!(above > 0.5 && below < -0.5, "above {above} below {below}");
    let crossings = body[..180]
        .windows(2)
        .filter(|w| (w[0] > 0.05 && w[1] <= 0.05) || (w[0] < -0.05 && w[1] >= -0.05))
        .count();
    assert!(crossings >= 3, "crossings {crossings}");
    for (frame, (b, r)) in trace.iter().enumerate().skip(270) {
        assert!(
            (b - target).abs() < 0.1 && r.abs() < 0.1,
            "frame {frame}: {b} {r}"
        );
    }
}

/// 首の追従: 首の回転デフォーマを選ぶ (なければ拒否)。出力はデフォーマの角度なので動的物理になる。
#[test]
fn neck_follow_targets_a_rotation_deformer() {
    let mut doc = Document::new(fixtures::dynamic_hair());
    let before = doc.model().clone();
    // `dynamic_hair` には首の名前のデフォーマがない。
    let err = doc
        .apply(template(
            PhysicsTemplate::NeckFollow,
            PhysicsTemplateOptions::default(),
        ))
        .unwrap_err();
    assert!(matches!(err, EditError::InvalidPhysics { .. }), "{err}");
    // ワープは選べない。
    let err = doc
        .apply(template(
            PhysicsTemplate::NeckFollow,
            PhysicsTemplateOptions {
                neck: Some("Cloth".into()),
                ..PhysicsTemplateOptions::default()
            },
        ))
        .unwrap_err();
    assert!(
        matches!(err, EditError::DeformerKindMismatch { .. }),
        "{err}"
    );

    // 名前に「首」を含む回転デフォーマは自動で選ぶ。
    doc.apply(Command::RenameDeformer {
        id: "Body".into(),
        name: "首".into(),
    })
    .unwrap();
    doc.apply(template(
        PhysicsTemplate::NeckFollow,
        PhysicsTemplateOptions::default(),
    ))
    .unwrap();
    let rig = &doc.physics().unwrap().rigs[0];
    assert_eq!(rig.id.as_str(), "NeckFollow");
    assert_eq!(
        rig.outputs[0].target,
        PhysicsTarget::DeformerAngle {
            deformer: "Body".into()
        }
    );
    let inputs: Vec<&str> = rig.inputs.iter().map(|i| i.parameter.as_str()).collect();
    assert_eq!(inputs, [ANGLE_Z, ANGLE_X]);

    // 頭を傾けると、首のデフォーマの子 (`Cloth` のスカート) が遅れて傾き、傾いたまま落ち着く。
    let mut puppet = Puppet::new(doc.model().clone()).unwrap();
    puppet.set_physics(doc.physics().cloned()).unwrap();
    assert!(puppet.physics().unwrap().is_dynamic());
    let roll = puppet.instance().parameters().index_of(ANGLE_Z).unwrap();
    let skirt = |puppet: &Puppet| {
        let i = puppet
            .instance()
            .compiled()
            .model()
            .meshes
            .iter()
            .position(|m| m.id == "Skirt")
            .unwrap();
        puppet.instance().drawables()[i].positions.clone()
    };
    puppet.update(1.0 / FPS);
    let rest = skirt(&puppet);
    let mut moved = Vec::new();
    for _ in 0..180 {
        puppet.instance_mut().parameters_mut().set(roll, 30.0);
        puppet.update(1.0 / FPS);
        let now = skirt(&puppet);
        moved.push(
            now.iter()
                .zip(&rest)
                .map(|(a, b)| a.distance(*b))
                .fold(0.0, f32::max),
        );
    }
    let last = moved[179];
    assert!(last > 1.0, "the neck must tilt: {last}");
    assert!(
        moved[0] < last * 0.5,
        "the neck must lag: {} vs {last}",
        moved[0]
    );
    assert!((moved[178] - last).abs() < 0.05, "settled: {moved:?}");

    // 1 回の Undo で戻る。
    assert!(doc.undo());
    assert!(doc.undo());
    assert_eq!(doc.model(), &before);
    assert!(doc.physics().is_none());
}
