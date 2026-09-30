//! M3 の受け入れテスト (T0310) のうち、ランタイム SDK (`Puppet`) で確かめるもの。
//!
//! - 物理・モーション・表情・目パチ・呼吸・リップシンク・ポーズを 1 つの `Puppet` で同時に動かし、
//!   フレーム内の適用順 (`docs/spec/motion.md#フレーム内の適用順`) の効果を確かめる。
//! - 揺れ用パラメータのない髪 (回転デフォーマの鎖) が揺れ、体の衝突体にぶつかる
//!   (`docs/spec/physics.md#動的物理-揺れ用パラメータのいらない物理`)。
//! - アバタープロファイルに準拠した見本のモデル (`assets/samples/avatar.ugoe`、T0702) を、
//!   ARKit のブレンドシェイプの列で動かす (`docs/spec/avatar-profile.md#フェイストラッキングとの対応`)。
//!
//! エディタで作ったモデルを書き出して再生するテストは `ugoe-editor-core/tests/m3_workflow.rs`。

#![allow(clippy::unwrap_used)]

use std::f32::consts::TAU;
use std::path::PathBuf;

use ugoe::core::Vec2;
use ugoe::model::avatar::check::check_profile;
use ugoe::model::effects::{Effects, EyeBlink};
use ugoe::model::expression::{Expression, ExpressionOp, ExpressionParameter};
use ugoe::model::motion::{Curve, CurveTarget, Motion, Point, Segment};
use ugoe::model::physics::{
    PhysicsBlend, PhysicsInput, PhysicsOutput, PhysicsSettings, PhysicsSource, PhysicsTarget,
};
use ugoe::model::{Model, fixtures, standard};
use ugoe::motion::tracking::standard_value;
use ugoe::motion::{BodyInput, Priority, Rotation, TrackingInput};
use ugoe::{Puppet, Toggles};

const DT: f32 = 1.0 / 60.0;

fn param(puppet: &Puppet, id: &str) -> f32 {
    let p = puppet.instance().parameters();
    p.get(p.index_of(id).unwrap_or_else(|| panic!("parameter {id}")))
}

#[track_caller]
fn assert_close(actual: f32, expected: f32, tolerance: f32, what: &str) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{what}: {actual} != {expected} (±{tolerance})"
    );
}

fn deformer(puppet: &Puppet, id: &str) -> usize {
    puppet
        .instance()
        .compiled()
        .deformer_index(id)
        .unwrap_or_else(|| panic!("deformer {id}"))
}

/// 動的物理がデフォーマ `id` の角度に加えた量 (度)。
fn angle_offset(puppet: &Puppet, id: &str) -> f32 {
    puppet
        .instance()
        .dynamic_offsets()
        .expect("dynamic physics")
        .angle(deformer(puppet, id))
}

/// 回転デフォーマ `id` の原点のワールド位置。
fn origin(puppet: &Puppet, id: &str) -> Vec2 {
    puppet
        .instance()
        .rotation_world_frame(deformer(puppet, id))
        .expect("rotation deformer")
        .translation
}

/// `fixtures::dynamic_hair_settings` の衝突体 `Shoulder` (ワールドで `(163,150)`–`(198,170)`、
/// 半径 8) の線分までの距離。粒子の衝突半径 2 と合わせ、10 未満なら食い込んでいる。
fn shoulder_distance(p: Vec2) -> f32 {
    let (a, b) = (Vec2::new(163.0, 150.0), Vec2::new(198.0, 170.0));
    let t = ((p - a).dot(b - a) / (b - a).length_squared()).clamp(0.0, 1.0);
    (p - (a + (b - a) * t)).length()
}
const SHOULDER_CLEARANCE: f32 = 10.0;

fn constant_curve(target: CurveTarget, v: f32) -> Curve {
    Curve {
        target,
        segments: vec![Segment::Start {
            p: Point { t: 0.0, v },
        }],
        fade_in: None,
        fade_out: None,
    }
}

fn motion(name: &str, duration: f32, curves: Vec<Curve>) -> Motion {
    Motion {
        name: name.into(),
        duration,
        looping: true,
        fade_in: 0.0,
        fade_out: 0.0,
        curves,
        events: vec![],
    }
}

// ---- 1. すべてを同じ Puppet で ----

/// 見本のアバター (`fixtures::avatar_profile`: 全標準・拡張パラメータ、ポーズ `ArmA`/`ArmB`、
/// 標準のエフェクト) に、`fixtures::dynamic_hair` の体・頭・髪の鎖・スカートを足したモデル。
fn full_model() -> Model {
    let mut model = fixtures::avatar_profile();
    let hair = fixtures::dynamic_hair();
    // `dynamic_hair` の `AngleX` / `AngleZ` は標準パラメータと同じ値域。
    for p in &hair.parameters {
        assert!(model.parameters.iter().any(|q| q == p), "{}", p.id.as_str());
    }
    model.deformers.extend(hair.deformers);
    model.meshes.extend(hair.meshes);
    model.effects = Effects {
        // 数秒のうちに何度かまばたきするように間隔を短くする。
        eye_blink: EyeBlink {
            interval: 1.0,
            ..EyeBlink::standard()
        },
        ..Effects::standard()
    };
    model
}

/// 動的物理 (`dynamic_hair_settings` の髪とスカート)、パラメータへの物理 (`physics_hair_settings` の
/// 前髪・後ろ髪)、口の開きを入力にするリグ `Jaw` (出力は `HairSide`) をまとめた設定。
fn full_physics() -> PhysicsSettings {
    let mut settings = fixtures::dynamic_hair_settings();
    settings.rigs.extend(fixtures::physics_hair_settings().rigs);
    let mut jaw = fixtures::physics_hair_settings().rigs.remove(0);
    jaw.id = "Jaw".into();
    jaw.name = "Jaw".into();
    jaw.inputs = vec![PhysicsInput {
        parameter: standard::MOUTH_OPEN_Y.into(),
        source: PhysicsSource::X,
        weight: 100.0,
        reflect: false,
    }];
    jaw.outputs = vec![PhysicsOutput {
        target: PhysicsTarget::Parameter {
            id: standard::HAIR_SIDE.into(),
        },
        particle: 1,
        source: PhysicsSource::Angle,
        scale: 0.1,
        weight: 100.0,
        reflect: false,
        blend: PhysicsBlend::Overwrite,
    }];
    settings.rigs.push(jaw);
    settings
}

fn full_puppet() -> Puppet {
    let model = full_model();
    assert_eq!(ugoe::model::validate::validate(&model), vec![]);
    let mut puppet = Puppet::new(model).expect("compile");
    puppet
        .set_physics(Some(full_physics()))
        .expect("valid physics");
    assert!(puppet.physics().unwrap().is_dynamic());
    puppet
}

/// トラッキングの入力: 頭を 10 度左へ、左目を半分閉じ、口を 0.3 開け、微笑む。
fn tracking(jaw_open: f32) -> TrackingInput {
    let mut input = TrackingInput {
        head: Rotation {
            yaw: 10.0,
            pitch: 0.0,
            roll: 0.0,
        },
        ..TrackingInput::default()
    };
    input.set_blend_shapes([
        ("eyeBlinkLeft", 0.5),
        ("jawOpen", jaw_open),
        ("mouthSmileLeft", 0.4),
        ("mouthSmileRight", 0.4),
    ]);
    input
}

/// `AngleX` の呼吸 (`Breath::standard()`: 重み 0.5、振幅 15、周期 6.5345 秒)。
fn breath_angle_x(t: f32) -> f32 {
    0.5 * 15.0 * (TAU * t / 6.5345).sin()
}

/// 各段が決まった順に重なることを、パラメータの値で確かめる。
///
/// - モーション (`AngleX` = 20、`Cheek` = 0.6、`ArmB` の表示) → トラッキング (`AngleX` を 10 で上書き)
///   → 基準値の保存 → 表情 (`Cheek` + 0.2、目 × 0.8、口 + 0.5) → 目パチ (目に掛ける) → 呼吸 (`AngleX` に足す)
///   → 物理 (呼吸を含む `AngleX` を読む。リップシンクは読まない) → リップシンク (口を上書き) → ポーズ。
#[test]
fn every_stage_runs_in_one_puppet_in_frame_order() {
    let mut puppet = full_puppet();
    assert!(puppet.start_motion(
        motion(
            "Idle",
            2.0,
            vec![
                constant_curve(CurveTarget::Parameter(standard::ANGLE_X.into()), 20.0),
                constant_curve(CurveTarget::Parameter(standard::CHEEK.into()), 0.6),
                constant_curve(CurveTarget::PartOpacity("ArmB".into()), 1.0),
            ],
        ),
        Priority::Normal,
    ));
    let op = |id: &str, value: f32, op: ExpressionOp| ExpressionParameter {
        parameter: id.into(),
        value,
        op,
    };
    puppet.set_expression(Expression {
        name: "Smile".into(),
        fade_in: 0.0,
        fade_out: 0.0,
        parameters: vec![
            op(standard::CHEEK, 0.2, ExpressionOp::Add),
            op(standard::EYE_L_OPEN, 0.8, ExpressionOp::Multiply),
            op(standard::EYE_R_OPEN, 0.8, ExpressionOp::Multiply),
            op(standard::MOUTH_OPEN_Y, 0.5, ExpressionOp::Add),
        ],
    });
    puppet.set_tracking(Some(tracking(0.3)));

    let part = |id: &str| {
        let compiled = puppet.instance().compiled();
        compiled
            .part_index(id)
            .unwrap_or_else(|| panic!("part {id}"))
    };
    let (arm_a, sleeve_a, arm_b) = (part("ArmA"), part("SleeveA"), part("ArmB"));
    let mut blinked = false;
    let mut eyes_open = false;
    let mut hair_swing = 0.0f32;
    let mut hair_front = 0.0f32;
    for frame in 1..=360 {
        let t = frame as f32 * DT;
        // リップシンクの音量を 0.25 秒ごとに入れたり切ったりする。
        let lip_sync = (frame / 15) % 2 == 0;
        puppet.set_lip_sync_level(lip_sync.then_some(1.0));
        puppet.update(DT);
        let what = |id: &str| format!("{id} at frame {frame}");

        // モーションの AngleX はトラッキングが上書きし、呼吸はその上に足される。
        assert_close(
            param(&puppet, standard::ANGLE_X),
            10.0 + breath_angle_x(t),
            1e-2,
            &what("AngleX"),
        );
        // トラッキングしないパラメータにはモーションの値が残り、表情が重なる (累積しない)。
        assert_close(param(&puppet, standard::CHEEK), 0.8, 1e-5, &what("Cheek"));
        assert_close(
            param(&puppet, standard::MOUTH_FORM),
            0.4,
            1e-5,
            &what("MouthForm"),
        );
        // 目: トラッキング (0.5 / 1.0) × 表情 (0.8) × 目パチ (両目に同じ開き具合)。
        let (left, right) = (
            param(&puppet, standard::EYE_L_OPEN),
            param(&puppet, standard::EYE_R_OPEN),
        );
        assert!(right <= 0.8 + 1e-5, "{}: {right}", what("EyeROpen"));
        assert_close(left, 0.5 * right, 1e-5, &what("EyeLOpen"));
        blinked |= right == 0.0;
        eyes_open |= (right - 0.8).abs() < 1e-5;
        // 口: リップシンクは表情の後に上書きする。切ると基準値 (トラッキング 0.3 + 表情 0.5) に戻る。
        let mouth = if lip_sync { 1.0 } else { 0.8 };
        assert_close(
            param(&puppet, standard::MOUTH_OPEN_Y),
            mouth,
            1e-5,
            &what("MouthOpenY"),
        );
        // 物理はリップシンクより前に読むので、口のリグの入力は 0.8 のまま変わらず、揺れない。
        assert_close(
            param(&puppet, standard::HAIR_SIDE),
            0.0,
            1e-4,
            &what("HairSide"),
        );
        // 呼吸で頭が動くので、パラメータへの物理と動的物理がどちらも揺れる。
        hair_front = hair_front.max(param(&puppet, standard::HAIR_FRONT).abs());
        hair_swing = hair_swing.max(angle_offset(&puppet, "Hair1").abs());
        // 髪は揺れても肩の衝突体に食い込まない。
        let chain = puppet.physics().unwrap().chain(0).unwrap();
        for (i, p) in chain.particles().iter().enumerate().skip(1) {
            assert!(
                shoulder_distance(p.position) >= SHOULDER_CLEARANCE - 1e-3,
                "{}: particle {i} at {}",
                what("Hair"),
                p.position
            );
        }
        // ポーズ: モーションが `ArmB` を求めるので、`fade_duration` (0.5 秒) かけて切り替わる。
        let opacity = |p: usize| puppet.instance().part_opacity_override(p);
        let shown = (t / 0.5).min(1.0);
        assert_close(opacity(arm_b).0, shown, 1e-3, &what("ArmB"));
        assert!(opacity(arm_a).0 <= 1.0 - shown + 1e-3, "{}", what("ArmA"));
        assert_eq!(opacity(arm_a), opacity(sleeve_a), "{}", what("SleeveA"));
    }
    assert!(blinked && eyes_open, "the eyes must blink and reopen");
    assert!(hair_front > 1e-3, "HairFront never moved: {hair_front}");
    assert!(hair_swing > 0.1, "the hair never swung: {hair_swing}");
    assert_eq!(puppet.instance().part_opacity_override(arm_b), (1.0, 1.0));
    assert_eq!(puppet.instance().part_opacity_override(arm_a), (0.0, 1.0));
}

/// 上のテストの対照: 口のリグは入力が変われば揺れる (リップシンクの代わりにトラッキングで口を動かす)。
/// 呼吸を止めると頭が動かないので、パラメータへの物理は揺れない。
#[test]
fn physics_reads_breath_and_tracking_but_not_lip_sync() {
    let mut puppet = full_puppet();
    puppet.set_toggles(Toggles {
        breath: false,
        eye_blink: false,
        ..Toggles::default()
    });
    let mut side = 0.0f32;
    for frame in 1..=120 {
        let jaw = if (frame / 15) % 2 == 0 { 0.3 } else { 0.5 };
        puppet.set_tracking(Some(tracking(jaw)));
        puppet.update(DT);
        side = side.max(param(&puppet, standard::HAIR_SIDE).abs());
        assert_close(
            param(&puppet, standard::HAIR_FRONT),
            0.0,
            1e-4,
            &format!("HairFront at frame {frame}"),
        );
        assert_close(
            angle_offset(&puppet, "Hair1"),
            0.0,
            1e-3,
            &format!("Hair1 at frame {frame}"),
        );
    }
    assert!(side > 1e-3, "HairSide never moved: {side}");
}

// ---- 2. 揺れ用パラメータのない髪が揺れて体にぶつかる ----

/// 頭を左右に振るモーション (`AngleX`: 0 → 30 → -30 → 0、2 秒でループ)。
fn head_shake() -> Motion {
    let key = |t: f32, v: f32| Point { t, v };
    motion(
        "Shake",
        2.0,
        vec![Curve {
            target: CurveTarget::Parameter(standard::ANGLE_X.into()),
            segments: vec![
                Segment::Start { p: key(0.0, 0.0) },
                Segment::Linear { to: key(0.5, 30.0) },
                Segment::Linear {
                    to: key(1.5, -30.0),
                },
                Segment::Linear { to: key(2.0, 0.0) },
            ],
            fade_in: None,
            fade_out: None,
        }],
    )
}

/// 肩の衝突体 (`Body` に取り付けた円)。中心は `Body` の局所座標で `(40,-45)`、ワールドでは `(168,155)`。
/// 静止姿勢の髪 (x = 148) からは離れていて、頭を右へ振ると髪が当たる。
const SHOULDER_CENTER: Vec2 = Vec2::new(168.0, 155.0);
const SHOULDER_CIRCLE_RADIUS: f32 = 6.0;

/// `dynamic_hair_settings` の肩のカプセルを、上の円に置き換えた設定。髪の粒子の衝突半径は 2。
fn shoulder_circle_settings() -> PhysicsSettings {
    let mut settings = fixtures::dynamic_hair_settings();
    settings.colliders[0].shape = ugoe::model::physics::ColliderShape::Circle {
        center: Vec2::new(40.0, -45.0),
        radius: SHOULDER_CIRCLE_RADIUS,
    };
    settings
}

/// 頭を振ったときの、髪の粒子と肩の中心の距離の最小値、描かれる房の関節 (`Hair2`・`Hair3` の原点) と
/// 肩の中心の距離の最小値、髪の角度の最大の振れ。
fn shake_hair(settings: PhysicsSettings) -> (f32, f32, f32) {
    let mut puppet = Puppet::new(fixtures::dynamic_hair())
        .unwrap()
        .with_physics(settings);
    assert!(puppet.start_motion(head_shake(), Priority::Normal));
    let (mut particles, mut joints, mut swing) = (f32::INFINITY, f32::INFINITY, 0.0f32);
    for _ in 0..240 {
        puppet.update(DT);
        let chain = puppet.physics().unwrap().chain(0).unwrap();
        for p in chain.particles().iter().skip(1) {
            particles = particles.min(p.position.distance(SHOULDER_CENTER));
        }
        for id in ["Hair2", "Hair3"] {
            joints = joints.min(origin(&puppet, id).distance(SHOULDER_CENTER));
        }
        for id in ["Hair1", "Hair2", "Hair3"] {
            swing = swing.max(angle_offset(&puppet, id).abs());
        }
    }
    (particles, joints, swing)
}

#[test]
fn hair_swings_without_swing_parameters_and_hits_the_body() {
    // モデルには頭の向きのパラメータしかない (髪を揺らすパラメータはない)。
    let model = fixtures::dynamic_hair();
    let ids: Vec<&str> = model.parameters.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, [standard::ANGLE_X, standard::ANGLE_Z]);
    let reach = SHOULDER_CIRCLE_RADIUS + 2.0;

    // 衝突体がなければ、髪は肩を通り抜けて揺れる。
    let mut free = shoulder_circle_settings();
    for rig in &mut free.rigs {
        rig.colliders.clear();
    }
    let (_, joints, swing) = shake_hair(free);
    assert!(swing > 30.0, "swing without colliders: {swing}");
    assert!(
        joints < SHOULDER_CIRCLE_RADIUS - 2.0,
        "the free hair should pass through the shoulder: {joints}"
    );

    // 衝突体があると、粒子は肩の外 (半径 + 衝突半径) で止まる。区間の長さは保たれるので、描かれる房の関節も
    // 粒子と同じ位置にあり、出力の補間 (最大 1 ステップの遅れ、`docs/spec/physics.md#時間の進め方`) の分
    // (1px 未満) を除いて肩の外にある (T0311)。
    let (particles, joints, swing) = shake_hair(shoulder_circle_settings());
    assert!(swing > 30.0, "swing with colliders: {swing}");
    assert!(
        particles >= reach - 1e-3,
        "a particle went into the shoulder: {particles}"
    );
    assert!(
        joints >= reach - 1.0,
        "the drawn hair went into the shoulder: {joints}"
    );
}

/// 髪の毛先 (メッシュ `Strand` の下の頂点) は、頭と一緒に動くだけでなく遅れて振れる。
#[test]
fn hair_tip_lags_behind_the_head() {
    let mut puppet = Puppet::new(fixtures::dynamic_hair())
        .unwrap()
        .with_physics(fixtures::dynamic_hair_settings());
    let strand = puppet.instance().compiled().mesh_index("Strand").unwrap();
    puppet.update(DT);
    let rest = puppet.instance().drawables()[strand].positions.clone();
    let head_rest = origin(&puppet, "Head");
    puppet.start_motion(head_shake(), Priority::Normal);
    let mut lag = 0.0f32;
    for _ in 0..60 {
        puppet.update(DT);
        let moved = origin(&puppet, "Head") - head_rest;
        let tip = puppet.instance().drawables()[strand].positions[3];
        lag = lag.max((tip - (rest[3] + moved)).length());
    }
    assert!(lag > 5.0, "the tip only lagged {lag}px");
}

// ---- 3. アバタープロファイルの見本を ARKit の値で動かす ----

fn sample_bundle() -> ugoe::format::Bundle {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/samples/avatar.ugoe");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    ugoe::format::read_bundle(&bytes).expect("read the sample bundle")
}

/// ARKit のフレームの列 (名前と値、頭の回転)。
fn arkit_frames() -> Vec<TrackingInput> {
    let frame = |yaw: f32, pitch: f32, roll: f32, shapes: &[(&str, f32)]| {
        let mut input = TrackingInput {
            head: Rotation { yaw, pitch, roll },
            ..TrackingInput::default()
        };
        assert_eq!(input.set_blend_shapes(shapes.iter().copied()), shapes.len());
        input
    };
    vec![
        frame(0.0, 0.0, 0.0, &[]),
        // 左目を閉じ、口を開ける。
        frame(0.0, 0.0, 0.0, &[("eyeBlinkLeft", 1.0), ("jawOpen", 0.7)]),
        // 自分の左を見て、微笑み、眉を上げる。
        frame(
            0.0,
            0.0,
            0.0,
            &[
                ("eyeLookOutLeft", 0.8),
                ("eyeLookInRight", 0.8),
                ("mouthSmileLeft", 0.6),
                ("mouthSmileRight", 0.6),
                ("browInnerUp", 0.5),
                ("browOuterUpLeft", 0.9),
            ],
        ),
        // 頭を回す (値域の外はクランプされる)。
        frame(20.0, -10.0, 15.0, &[("mouthPucker", 0.4)]),
        frame(
            50.0,
            45.0,
            -40.0,
            &[("eyeWideLeft", 1.0), ("eyeSquintRight", 0.7)],
        ),
        frame(0.0, 0.0, 0.0, &[]),
    ]
}

/// 見本のモデルの、ARKit のフレームの列に対する見た目 (描画物の頂点と不透明度)。`body` を各フレームの体の入力にする。
fn sample_looks(body: BodyInput) -> Vec<Vec<(Vec<Vec2>, f32)>> {
    let mut puppet = Puppet::new(sample_bundle().model).expect("compile the sample");
    puppet.set_toggles(Toggles {
        eye_blink: false,
        breath: false,
        ..Toggles::default()
    });
    arkit_frames()
        .into_iter()
        .map(|input| {
            puppet.set_tracking(Some(TrackingInput { body, ..input }));
            puppet.update(DT);
            puppet
                .instance()
                .drawables()
                .iter()
                .map(|d| (d.positions.clone(), d.opacity))
                .collect()
        })
        .collect()
}

#[test]
fn avatar_sample_is_unchanged_without_body_input() {
    // 体の入力が無いとき (T0645 より前と同じ入力) の見た目を基準にする。
    let base = sample_looks(BodyInput::default());
    // 見本には肩のパラメータが無いので、肩の入力だけがあっても見た目は同じ。
    let shoulders = sample_looks(BodyInput {
        shoulder_left: Some(0.8),
        shoulder_right: Some(-0.5),
        ..BodyInput::default()
    });
    assert_eq!(base, shoulders);
    // 体の回転を与えると変わる (比べ方が見た目の違いを捉えていることの確認)。
    let turned = sample_looks(BodyInput {
        yaw: Some(10.0),
        ..BodyInput::default()
    });
    assert_ne!(base, turned);
}

#[test]
fn avatar_sample_follows_arkit_tracking() {
    let bundle = sample_bundle();
    let report = check_profile(&bundle.model);
    assert_eq!(report.errors().count(), 0, "{:?}", report.issues);
    // 見本は T0704 で作り直すので、レベルは固定しない (主張したレベルを満たしていればよい)。
    assert_eq!(
        report.claimed,
        Some(report.satisfied),
        "{:?}",
        report.issues
    );
    let avatar = bundle.model.avatar.clone().expect("avatar profile");
    // 上書きのないパラメータは、標準の対応表の値 (値域にクランプ) になる。
    let overridden: Vec<&str> = avatar.tracking.iter().filter_map(|o| o.target()).collect();

    let model = bundle.model.clone();
    // トラッキングからパラメータへの対応付けだけを見るため、物理は読み込まない。見本の物理 (髪と耳飾りの揺れ、
    // T0704) は揺れが残って「中立に戻すと元の見た目に戻る」を 1 フレームでは満たさない。物理が読み込めて
    // 頭の動きで揺れることは `crates/ugoe-cli/src/sample.rs` の `physics_sways_earrings_and_hair_after_a_head_shake` が確かめる。
    let mut puppet = Puppet::new(bundle.model).expect("compile the sample");
    // トラッキングの値だけを見るため、目パチと呼吸を止める (ビューアもトラッキング中は目パチを止める)。
    puppet.set_toggles(Toggles {
        eye_blink: false,
        breath: false,
        ..Toggles::default()
    });
    let tracked: Vec<&str> = ugoe::model::avatar::TRACKED_PARAMETERS
        .iter()
        .copied()
        .filter(|id| model.parameters.iter().any(|p| p.id.as_str() == *id))
        .filter(|id| !overridden.contains(id))
        .collect();
    assert!(tracked.len() >= 15, "{tracked:?}");

    let mut looks = Vec::new();
    for (i, input) in arkit_frames().into_iter().enumerate() {
        puppet.set_tracking(Some(input));
        puppet.update(DT);
        for id in &tracked {
            let p = model
                .parameters
                .iter()
                .find(|p| p.id.as_str() == *id)
                .unwrap();
            let expected = standard_value(id, &input).unwrap().clamp(p.min, p.max);
            assert_close(
                param(&puppet, id),
                expected,
                1e-5,
                &format!("{id} at frame {i}"),
            );
        }
        looks.push(
            puppet
                .instance()
                .drawables()
                .iter()
                .map(|d| (d.positions.clone(), d.opacity))
                .collect::<Vec<_>>(),
        );
    }
    // 値を手で計算したものとも比べる。
    let frames = arkit_frames();
    let at = |i: usize, id: &str| standard_value(id, &frames[i]).unwrap();
    assert_eq!(at(1, standard::EYE_L_OPEN), 0.0);
    assert_close(at(1, standard::MOUTH_OPEN_Y), 0.7, 1e-6, "MouthOpenY");
    assert_close(at(2, standard::EYE_BALL_X), 0.8, 1e-6, "EyeBallX");
    assert_close(at(2, standard::MOUTH_FORM), 0.6, 1e-6, "MouthForm");
    assert_eq!(at(3, standard::ANGLE_X), 20.0);
    assert_close(at(3, standard::BODY_ANGLE_X), 6.0, 1e-5, "BodyAngleX");
    assert_eq!(param(&puppet, standard::ANGLE_X), 0.0);

    // トラッキングで見た目が変わり、中立に戻すと元の見た目に戻る。
    let moved = |a: &[(Vec<Vec2>, f32)], b: &[(Vec<Vec2>, f32)]| {
        a.iter()
            .zip(b)
            .map(|((pa, oa), (pb, ob))| {
                let d = pa
                    .iter()
                    .zip(pb)
                    .map(|(p, q)| p.distance(*q))
                    .fold(0.0, f32::max);
                d.max((oa - ob).abs() * 100.0)
            })
            .fold(0.0, f32::max)
    };
    for (i, look) in looks.iter().enumerate().take(5).skip(1) {
        assert!(
            moved(&looks[0], look) > 1.0,
            "frame {i} does not change the look"
        );
    }
    assert!(moved(&looks[0], &looks[5]) < 1e-3);
}

/// 物理の加算の出力 (T0313、`docs/spec/physics.md#合成の方法`)。トラッキングが `BodyAngleX` に書く
/// 頭の回転 × 0.3 に、頭の動きに遅れる揺れを足す。パラメータを複製しない。
/// フレームごとに基準値へ戻すので、揺れが収まるとトラッキングの値に戻る (累積しない)。
#[test]
fn physics_adds_sway_on_top_of_tracking() {
    let mut puppet = Puppet::new(fixtures::physics_additive()).expect("compile");
    puppet
        .set_physics(Some(fixtures::physics_additive_settings()))
        .expect("valid physics");
    let head = |yaw| TrackingInput {
        head: Rotation {
            yaw,
            pitch: 0.0,
            roll: 0.0,
        },
        ..TrackingInput::default()
    };
    puppet.set_tracking(Some(head(0.0)));
    for _ in 0..10 {
        puppet.update(DT);
    }
    assert_close(param(&puppet, standard::BODY_ANGLE_X), 0.0, 1e-5, "at rest");

    puppet.set_tracking(Some(head(20.0)));
    let trace: Vec<f32> = (0..300)
        .map(|_| {
            puppet.update(DT);
            param(&puppet, standard::BODY_ANGLE_X)
        })
        .collect();
    assert_close(param(&puppet, standard::ANGLE_X), 20.0, 1e-5, "AngleX");
    let target = 20.0 * ugoe::motion::tracking::BODY_FROM_HEAD;
    let min = trace[..30].iter().copied().fold(f32::INFINITY, f32::min);
    let max = trace[..120]
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, f32::max);
    assert!(min < target - 1.0, "the body lags behind the head: {min}");
    assert!(max > target + 0.1, "the body overshoots: {max}");
    for v in &trace[240..] {
        assert_close(*v, target, 0.05, "settled on the tracking value");
    }
}
