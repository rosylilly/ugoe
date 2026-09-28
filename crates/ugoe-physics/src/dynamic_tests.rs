//! 動的物理のテスト (`docs/spec/physics.md#動的物理-揺れ用パラメータのいらない物理`)。
//! 入力は `ugoe_model::fixtures::dynamic_hair` とその物理設定。

use ugoe_core::Mat2;
use ugoe_model::fixtures::{
    dynamic_hair, dynamic_hair_settings, physics_hair, physics_hair_settings,
};
use ugoe_model::physics::{PhysicsOutput, PhysicsSettings};
use ugoe_model::standard::{ANGLE_X, ANGLE_Z};
use ugoe_runtime::{CompiledModel, ModelInstance};

use super::*;

const STEP: f32 = 1.0 / 60.0;

fn setup(settings: PhysicsSettings) -> (PhysicsSystem, ModelInstance) {
    let instance = ModelInstance::new(CompiledModel::compile(dynamic_hair()).unwrap());
    (PhysicsSystem::new(settings), instance)
}

/// 衝突体のない設定 (髪がまっすぐ垂れる)。
fn without_colliders() -> PhysicsSettings {
    let mut settings = dynamic_hair_settings();
    for rig in &mut settings.rigs {
        rig.colliders.clear();
    }
    settings
}

/// 1 フレーム進める (`Puppet::update` の物理と評価の部分)。
fn frame(physics: &mut PhysicsSystem, instance: &mut ModelInstance) {
    physics.update_instance(instance, STEP);
    instance.update();
}

fn set(instance: &mut ModelInstance, id: &str, value: f32) {
    let index = instance.parameters().index_of(id).unwrap();
    instance.parameters_mut().set(index, value);
}

fn deformer(instance: &ModelInstance, id: &str) -> usize {
    instance.compiled().deformer_index(id).unwrap()
}

/// デフォーマ `id` に加えた角度。
fn angle_offset(instance: &ModelInstance, id: &str) -> f32 {
    instance
        .dynamic_offsets()
        .unwrap()
        .angle(deformer(instance, id))
}

/// 回転デフォーマ `id` の局所座標の下向き (0,1) のワールドでの向き。
fn world_down(instance: &ModelInstance, id: &str) -> Vec2 {
    let frame = instance
        .rotation_world_frame(deformer(instance, id))
        .unwrap();
    (frame.matrix2 * Vec2::Y).normalize()
}

fn rotated_down(degrees: f32) -> Vec2 {
    Mat2::from_angle(degrees.to_radians()) * Vec2::Y
}

#[test]
fn settings_with_dynamic_features_are_dynamic() {
    assert!(PhysicsSystem::new(dynamic_hair_settings()).is_dynamic());
    assert!(!PhysicsSystem::new(physics_hair_settings()).is_dynamic());
    // 衝突体だけ・重力の基準だけでも動的物理。
    let mut settings = physics_hair_settings();
    settings.rigs[0].frame = Some("Head".into());
    assert!(PhysicsSystem::new(settings).is_dynamic());
}

/// 揺れ用のパラメータなしで、回転デフォーマの鎖が揺れる。
#[test]
fn rotation_chain_swings_without_swing_parameters() {
    let (mut physics, mut instance) = setup(without_colliders());
    for _ in 0..30 {
        frame(&mut physics, &mut instance);
    }
    for id in ["Hair1", "Hair2", "Hair3"] {
        assert!(angle_offset(&instance, id).abs() < 1e-3, "{id}");
    }
    let strand = instance.compiled().mesh_index("Strand").unwrap();
    let rest = instance.drawables()[strand].positions.clone();

    // 頭を 1/6 秒で右へ 30 px 動かすと、髪は遅れて左に振れる (時計回りが正)。
    let mut peak = 0.0f32;
    for k in 1..=10 {
        set(&mut instance, ANGLE_X, 3.0 * k as f32);
        frame(&mut physics, &mut instance);
        peak = peak.max(angle_offset(&instance, "Hair1"));
    }
    assert!(peak > 5.0, "peak = {peak}");
    // 毛先は、頭と一緒に平行移動しただけの位置からずれている。
    let tip = instance.drawables()[strand].positions[3];
    assert!((tip - (rest[3] + Vec2::new(30.0, 0.0))).length() > 5.0);

    // 止まると揺れが収まり、また真下に垂れる。
    for _ in 0..600 {
        frame(&mut physics, &mut instance);
    }
    for id in ["Hair1", "Hair2", "Hair3"] {
        assert!(angle_offset(&instance, id).abs() < 0.1, "{id}");
    }
    let tip = instance.drawables()[strand].positions[3];
    assert!(
        tip.abs_diff_eq(rest[3] + Vec2::new(30.0, 0.0), 0.05),
        "{tip}"
    );
}

/// 基準のデフォーマがなければ重力はキャンバスの下向きなので、頭を傾けても髪は真下に垂れる。
#[test]
fn hair_hangs_down_when_head_tilts_without_frame() {
    let (mut physics, mut instance) = setup(without_colliders());
    set(&mut instance, ANGLE_Z, 30.0);
    for _ in 0..3 {
        frame(&mut physics, &mut instance);
    }
    assert!((angle_offset(&instance, "Hair1") + 30.0).abs() < 1e-3);
    assert!(angle_offset(&instance, "Hair2").abs() < 1e-3);
    for id in ["Hair1", "Hair2", "Hair3"] {
        assert!(world_down(&instance, id).abs_diff_eq(Vec2::Y, 1e-4), "{id}");
    }
}

/// 基準のデフォーマを傾けると重力の向きが変わる。
#[test]
fn tilting_the_frame_deformer_turns_gravity() {
    let mut settings = without_colliders();
    settings.rigs[0].frame = Some("Head".into());
    let (mut physics, mut instance) = setup(settings);
    for _ in 0..3 {
        frame(&mut physics, &mut instance);
    }
    let chain_down = |physics: &PhysicsSystem| {
        let p = physics.chain(0).unwrap().particles();
        (p[3].position - p[0].position).normalize()
    };
    assert!(chain_down(&physics).abs_diff_eq(Vec2::Y, 1e-4));

    // 頭を 30 度傾けると、粒子の鎖が垂れる向きも 30 度回る。髪は頭と一緒に傾いたまま。
    set(&mut instance, ANGLE_Z, 30.0);
    for _ in 0..600 {
        frame(&mut physics, &mut instance);
    }
    assert!(
        chain_down(&physics).abs_diff_eq(rotated_down(30.0), 1e-3),
        "{}",
        chain_down(&physics)
    );
    assert!(angle_offset(&instance, "Hair1").abs() < 0.05);
    assert!(world_down(&instance, "Hair3").abs_diff_eq(rotated_down(30.0), 1e-3));
}

/// カプセル `(118,140)`–`(168,160)` までの距離。
fn shoulder_distance(p: Vec2) -> f32 {
    let (a, b) = (Vec2::new(118.0, 140.0), Vec2::new(168.0, 160.0));
    let t = ((p - a).dot(b - a) / (b - a).length_squared()).clamp(0.0, 1.0);
    (p - (a + (b - a) * t)).length()
}

/// 衝突体の外に粒子が押し出される。
#[test]
fn particles_are_pushed_out_of_colliders() {
    // 衝突体がなければ、粒子 2 は肩のカプセルの内側 (半径 8 + 衝突半径 2) に垂れる。
    let (mut physics, mut instance) = setup(without_colliders());
    frame(&mut physics, &mut instance);
    let p = physics.chain(0).unwrap().particles()[2].position;
    assert!(shoulder_distance(p) < 10.0 - 1.0, "{p}");

    let (mut physics, mut instance) = setup(dynamic_hair_settings());
    for k in 0..240 {
        // 頭を左右に振っても、粒子は衝突体に入らない。
        set(&mut instance, ANGLE_X, 30.0 * (k as f32 * 0.05).sin());
        frame(&mut physics, &mut instance);
        for (i, particle) in physics
            .chain(0)
            .unwrap()
            .particles()
            .iter()
            .enumerate()
            .skip(1)
        {
            let d = shoulder_distance(particle.position);
            assert!(d >= 10.0 - 1e-3, "frame {k} particle {i}: {d}");
        }
    }
    // 衝突体を避けた姿勢が、デフォーマの角度に出る。
    assert!(
        angle_offset(&instance, "Hair1").abs() > 1.0
            || angle_offset(&instance, "Hair2").abs() > 1.0
    );
}

/// 頂点への出力: 根元が動くと、ワープの下の行とメッシュの下の頂点が遅れて追従する。
#[test]
fn vertices_follow_particle_displacement() {
    let (mut physics, mut instance) = setup(dynamic_hair_settings());
    let cloth = deformer(&instance, "Cloth");
    let ribbon = instance.compiled().mesh_index("Ribbon").unwrap();
    for _ in 0..10 {
        frame(&mut physics, &mut instance);
    }
    let rest = instance.warp_world_points(cloth).unwrap().to_vec();
    let rest_ribbon = instance.drawables()[ribbon].positions.clone();

    set(&mut instance, ANGLE_X, 30.0);
    let mut lag = 0.0f32;
    for _ in 0..10 {
        frame(&mut physics, &mut instance);
        let points = instance.warp_world_points(cloth).unwrap();
        // 上の行 (重み 0) は動かない。
        assert_eq!(points[0], rest[0]);
        assert_eq!(points[1], rest[1]);
        let d = points[2] - rest[2];
        lag = lag.min(d.x);
        // メッシュの頂点にも同じ x の変位を加える (y は加えない)。
        let r = &instance.drawables()[ribbon].positions;
        assert!((r[2] - rest_ribbon[2]).abs_diff_eq(Vec2::new(d.x, 0.0), 1e-4));
        assert_eq!(r[0], rest_ribbon[0]);
    }
    // 根元が右へ動くので、粒子は遅れて左にずれる。
    assert!(lag < -1.0, "lag = {lag}");

    for _ in 0..600 {
        frame(&mut physics, &mut instance);
    }
    let points = instance.warp_world_points(cloth).unwrap();
    assert!(points[2].abs_diff_eq(rest[2], 0.05));
}

/// 原点への出力: 粒子の変位だけ回転デフォーマの原点をずらす。
#[test]
fn deformer_offset_moves_the_origin() {
    let mut settings = without_colliders();
    settings.rigs[0].outputs.push(PhysicsOutput {
        target: PhysicsTarget::DeformerOffset {
            deformer: "Body".into(),
        },
        particle: 1,
        source: PhysicsSource::X,
        scale: 2.0,
        weight: 50.0,
        reflect: false,
    });
    let (mut physics, mut instance) = setup(settings);
    let body = deformer(&instance, "Body");
    frame(&mut physics, &mut instance);
    assert_eq!(instance.dynamic_offsets().unwrap().origin(body), Vec2::ZERO);
    set(&mut instance, ANGLE_X, 30.0);
    frame(&mut physics, &mut instance);
    frame(&mut physics, &mut instance);
    let offset = instance.dynamic_offsets().unwrap().origin(body);
    assert!(offset.x < -0.1 && offset.y == 0.0, "{offset}");
    // 変位 × scale 2 × weight 50% = 変位。
    let p = physics.chain(0).unwrap().particles();
    let d = p[1].position - p[0].position - Vec2::new(0.0, 30.0);
    assert!((offset.x - d.x).abs() < 1.0);
    let frame_body = instance.rotation_world_frame(body).unwrap();
    assert_eq!(frame_body.translation, Vec2::new(128.0, 200.0) + offset);
}

/// `docs/spec/physics.md#決定性`: 同じ入力列から同じ出力 (ビット単位で一致)。
#[test]
fn dynamic_physics_is_deterministic() {
    let run = || {
        let (mut physics, mut instance) = setup(dynamic_hair_settings());
        let mut trace = Vec::new();
        for k in 0..240 {
            set(&mut instance, ANGLE_X, 30.0 * (k as f32 * 0.07).sin());
            set(&mut instance, ANGLE_Z, 20.0 * (k as f32 * 0.03).cos());
            // 表示のフレームレートが揺れても決定的。
            let dt = if k % 3 == 0 { STEP * 1.7 } else { STEP * 0.6 };
            physics.update_instance(&mut instance, dt);
            instance.update();
            trace.push(instance.drawables().to_vec());
        }
        trace
    };
    assert_eq!(run(), run());
}

/// 動的物理を使わない設定では、インスタンスに 2 段階の評価の状態を作らず、評価も増えない。
#[test]
fn non_dynamic_settings_leave_evaluation_unchanged() {
    let mut physics = PhysicsSystem::new(physics_hair_settings());
    let mut reference = PhysicsSystem::new(physics_hair_settings());
    let mut instance = ModelInstance::new(CompiledModel::compile(physics_hair()).unwrap());
    let mut params = instance.parameters().clone();
    for k in 0..30 {
        set(&mut instance, ANGLE_X, k as f32);
        let index = params.index_of(ANGLE_X).unwrap();
        params.set(index, k as f32);
        let count = instance.update_count();
        physics.update_instance(&mut instance, STEP);
        // 1 段目の評価をしない。
        assert_eq!(instance.update_count(), count);
        reference.update(&mut params, STEP);
        assert_eq!(instance.parameters().as_slice(), params.as_slice());
        instance.update();
    }
    assert!(instance.dynamic_offsets().is_none());
}
