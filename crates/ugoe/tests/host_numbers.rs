//! ホストから来る数 (NaN・無限大) が `Puppet` の合成を通っても、パラメータ・パーツの不透明度・頂点に
//! 有限でない値が残らず、指揮役の出力を受け手が `follow` で受け付けること (T0010、`docs/portability.md#ホストから来る数`)。
//!
//! `Puppet::start_motion`・`Puppet::set_expression` は渡されたモーション・表情を検証しない
//! (ファイルから読むものは `ugoe-format` が検証する)。ここではわざと有限でない数を入れたモーションと表情を渡す。

#![allow(clippy::unwrap_used)]

use ugoe::model::effects::{Effects, LIP_SYNC_GROUP, LipSync};
use ugoe::model::expression::{Expression, ExpressionOp, ExpressionParameter};
use ugoe::model::motion::{Curve, CurveTarget, Motion, Point, Segment};
use ugoe::model::physics::PhysicsSettings;
use ugoe::model::{Model, fixtures, standard};
use ugoe::motion::Priority;
use ugoe::{Puppet, Toggles};

const DT: f32 = 1.0 / 60.0;

fn constant(target: CurveTarget, v: f32) -> Curve {
    Curve {
        target,
        segments: vec![Segment::Start {
            p: Point { t: 0.0, v },
        }],
        fade_in: None,
        fade_out: None,
    }
}

/// パラメータ・パーツの不透明度 (ポーズの要素とそうでないもの)・エフェクトグループのカーブの値がすべて `bad` のモーション。
fn bad_curves(bad: f32) -> Motion {
    Motion {
        name: "bad_curves".into(),
        duration: 1.0,
        looping: true,
        fade_in: 0.0,
        fade_out: 0.0,
        curves: vec![
            constant(CurveTarget::Parameter(standard::ANGLE_X.into()), bad),
            constant(CurveTarget::PartOpacity("Root".into()), bad),
            constant(CurveTarget::PartOpacity("ArmB".into()), bad),
            constant(CurveTarget::EffectGroup(LIP_SYNC_GROUP.into()), bad),
            constant(CurveTarget::EffectGroup("EyeBlink".into()), bad),
        ],
        events: vec![],
    }
}

/// 長さとフェードが `bad` のモーション (カーブの値は有限)。
fn bad_timing(bad: f32) -> Motion {
    Motion {
        name: "bad_timing".into(),
        duration: bad,
        looping: true,
        fade_in: bad,
        fade_out: bad,
        curves: vec![
            constant(CurveTarget::Parameter(standard::ANGLE_Y.into()), 10.0),
            constant(CurveTarget::PartOpacity("ArmA".into()), 0.5),
        ],
        events: vec![],
    }
}

fn bad_expression(bad: f32) -> Expression {
    let item = |id: &str, op| ExpressionParameter {
        parameter: id.into(),
        value: bad,
        op,
    };
    Expression {
        name: "bad".into(),
        fade_in: bad,
        fade_out: 0.0,
        parameters: vec![
            item(standard::ANGLE_Z, ExpressionOp::Add),
            item(standard::EYE_L_OPEN, ExpressionOp::Multiply),
            item(standard::BODY_ANGLE_X, ExpressionOp::Overwrite),
        ],
    }
}

/// パラメータの値・パーツの不透明度 (上書きと評価後)・頂点・描画の要素の不透明度。
fn state(puppet: &Puppet) -> Vec<f32> {
    let instance = puppet.instance();
    let parts = (0..instance.compiled().model().parts.len()).flat_map(|i| {
        let (value, weight) = instance.part_opacity_override(i).unwrap();
        [value, weight, instance.part_opacity(i).unwrap()]
    });
    let drawables = instance.drawables().iter().flat_map(|d| {
        d.positions
            .iter()
            .flat_map(|p| [p.x, p.y])
            .chain([d.opacity])
    });
    instance
        .parameters()
        .as_slice()
        .iter()
        .copied()
        .chain(parts)
        .chain(drawables)
        .collect()
}

fn cases() -> Vec<(Model, Option<PhysicsSettings>)> {
    vec![
        (fixtures::pose_and_effects(), None),
        (
            fixtures::physics_hair(),
            Some(fixtures::physics_hair_settings()),
        ),
    ]
}

fn make(model: &Model, physics: Option<&PhysicsSettings>) -> Puppet {
    let mut puppet = Puppet::new(model.clone()).unwrap();
    if let Some(settings) = physics {
        puppet = puppet.with_physics(settings.clone());
        // リップシンクを物理の入力に当てる。
        puppet.set_effects(Effects {
            lip_sync: LipSync {
                parameters: vec![standard::ANGLE_X.into()],
            },
            ..Effects::default()
        });
    }
    puppet
}

/// 有限でない数を持つモーションと表情を、1 つの `Puppet` と、指揮役 + 受け手 (`follow`) の両方で再生する。
/// どちらも有限のままで、受け手は指揮役の出力をすべて受け付け、1 つの `Puppet` と同じ形になる。
#[test]
fn non_finite_motion_and_expression_data_keep_the_puppet_finite() {
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        for (model, physics) in cases() {
            let mut single = make(&model, physics.as_ref());
            let mut conductor = make(&model, physics.as_ref());
            conductor.set_toggles(Toggles::default().with_physics(false));
            let mut follower = make(&model, physics.as_ref());
            for puppet in [&mut single, &mut conductor] {
                puppet.start_motion(bad_curves(bad), Priority::Normal);
                puppet.start_motion(bad_timing(bad), Priority::Idle);
                puppet.set_expression(bad_expression(bad));
            }
            for frame in 0..120 {
                let level = (frame % 5) as f32 / 4.0;
                for puppet in [&mut single, &mut conductor] {
                    puppet.set_lip_sync_level(Some(level));
                    puppet.update(DT);
                }
                let what = format!("{} {bad} frame {frame}", model.meta.name);
                let a = state(&single);
                assert!(a.iter().all(|v| v.is_finite()), "{what}: {a:?}");
                follower
                    .follow(
                        conductor.values_before_physics(),
                        conductor.lip_sync_applied(),
                        conductor.part_opacities_applied(),
                        DT,
                    )
                    .unwrap_or_else(|e| panic!("{what}: {e}"));
                let b = state(&follower);
                assert_eq!(a.len(), b.len());
                for (x, y) in a.iter().zip(&b) {
                    assert!((x - y).abs() <= 1e-3, "{what}: {x} != {y}");
                }
            }
        }
    }
}
