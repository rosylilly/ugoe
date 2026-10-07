//! 外から来るデータ (ファイル・ホストの引数) のどんな値でも、SDK (`Puppet`) とその下の層がパニックしないこと
//! (T0184、`docs/portability.md#パニックしない`、ADR 0031)。
//!
//! - 切り詰めた・ビットを反転したバンドル (`assets/compat` と `ugoe_model::fixtures` から書き出したもの) とモジュールを読み、
//!   読めたものは `Puppet` にして動かす。
//! - 巨大な数・NaN・無限大のパラメータ・`dt`・トラッキング・リップシンク・受け手の入力・当たり判定の点を与える
//!   (扱いは `docs/portability.md#ホストから来る数`)。
//! - 範囲外の番号 (受け手のパーツの番号・動的物理の加算量の番号・評価の問い合わせ) と、知らない ID を与える。
//!
//! 読み込みと画像のデコードだけの検査は `crates/ugoe-format/tests/broken_input.rs`。

#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use ugoe::core::Vec2;
use ugoe::format::module_file::read_module;
use ugoe::format::{
    Bundle, TextureData, TextureFormat, WriteOptions, read_bundle, write_bundle_with,
};
use ugoe::model::effects::{Breath, BreathParameter, Effects, EyeBlink, LipSync};
use ugoe::model::expression::{Expression, ExpressionOp, ExpressionParameter};
use ugoe::model::motion::{Curve, CurveTarget, Motion, MotionEvent, Point, Segment};
use ugoe::model::physics::{ColliderShape, PhysicsSettings};
use ugoe::model::{Model, fixtures, standard};
use ugoe::motion::{BodyInput, Priority, Rotation, TrackingInput};
use ugoe::runtime::{ModelInstance, ParameterValues};
use ugoe::{LipSyncInput, PartOpacityInput, Puppet};

const DT: f32 = 1.0 / 60.0;

/// ホストが渡しうる極端な数 (`1e18` は 2 乗しても f32 で有限な大きな数)。
const EXTREMES: [f32; 12] = [
    f32::NAN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::MAX,
    f32::MIN,
    f32::MIN_POSITIVE,
    1.0e-45, // 非正規化数
    -0.0,
    1.0e30,
    -1.0e30,
    1.0e18,
    -1.0e18,
];

/// 極端な `dt` (1 年分・負・NaN・無限大を含む)。
const DTS: [f32; 9] = [
    DT,
    0.0,
    -1.0,
    f32::NAN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::MAX,
    365.0 * 24.0 * 3600.0,
    1.0e-30,
];

/// 1×1 の白い PNG (テクスチャを持つフィクスチャのバンドルに入れる)。
const WHITE_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0b, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0x63, 0xf8, 0x0f, 0x04, 0x00,
    0x09, 0xfb, 0x03, 0xfd, 0x68, 0xfa, 0x1c, 0xcc, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44,
    0xae, 0x42, 0x60, 0x82,
];

fn compat_dir(version: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/compat")
        .join(version)
}

/// `assets/compat` の拡張子が `extension` のファイル (名前と中身)。
fn compat_files(extension: &str) -> Vec<(String, Vec<u8>)> {
    let mut files = Vec::new();
    for version in ["dev", "future"] {
        for entry in std::fs::read_dir(compat_dir(version)).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == extension) {
                let name = format!("{version}/{}", path.file_name().unwrap().to_string_lossy());
                files.push((name, std::fs::read(&path).unwrap()));
            }
        }
    }
    files.sort();
    assert!(!files.is_empty(), "no .{extension} in assets/compat");
    files
}

/// 物理の設定を持つ `ugoe_model::fixtures` のモデル。
fn fixture_cases() -> Vec<(Model, Option<PhysicsSettings>)> {
    vec![
        (fixtures::single_quad(), None),
        (fixtures::pose_and_effects(), None),
        (fixtures::glued_quads(), None),
        (fixtures::nested_masks(), None),
        (fixtures::blend_shapes(), None),
        (fixtures::smooth_arc(), None),
        (fixtures::avatar_profile(), None),
        (
            fixtures::physics_hair(),
            Some(fixtures::physics_hair_settings()),
        ),
        (
            fixtures::physics_relay(),
            Some(fixtures::physics_relay_settings()),
        ),
        (
            fixtures::physics_additive(),
            Some(fixtures::physics_additive_settings()),
        ),
        (
            fixtures::dynamic_hair(),
            Some(fixtures::dynamic_hair_settings()),
        ),
    ]
}

/// 壊す元のバンドル: `assets/compat` のファイルと、`ugoe_model::fixtures` を書き出したもの (無圧縮と zstd)。
fn bundle_corpus() -> Vec<(String, Vec<u8>)> {
    let mut corpus = compat_files("ugoe");
    for (model, physics) in fixture_cases() {
        for compress in [false, true] {
            let name = format!("fixtures/{} (compress: {compress})", model.meta.name);
            let mut bundle = Bundle::new(model.clone());
            bundle.physics.clone_from(&physics);
            let png = || TextureData {
                format: TextureFormat::Png,
                bytes: WHITE_PNG.to_vec(),
            };
            bundle.textures = model.textures.iter().map(|_| png()).collect();
            bundle.normal_maps = (model.textures.iter())
                .filter(|t| t.normal_map.is_some())
                .map(|_| png())
                .collect();
            let bytes = write_bundle_with(&bundle, WriteOptions { compress }).unwrap();
            corpus.push((name, bytes));
        }
    }
    corpus
}

/// `bytes` を読めたら `Puppet` にして、モーションと表情を当てて動かす。読めないのはかまわない (パニックしなければよい)。
fn load_and_drive(bytes: &[u8]) {
    let Ok(bundle) = read_bundle(bytes) else {
        return;
    };
    let Ok(mut puppet) = Puppet::new(bundle.model) else {
        return;
    };
    if let Some(physics) = bundle.physics {
        puppet = puppet.with_physics(physics);
    }
    drive(&mut puppet, &bundle.motions, &bundle.expressions);
}

/// モーションと表情を当てて、短い `dt` と極端な `dt` で進め、当たり判定を問い合わせる。
fn drive(puppet: &mut Puppet, motions: &[Motion], expressions: &[Expression]) {
    for motion in motions {
        puppet.start_motion(motion.clone(), Priority::Force);
    }
    if let Some(expression) = expressions.first() {
        puppet.set_expression(expression.clone());
    }
    for dt in [DT, DT, 0.5, f32::MAX, DT] {
        puppet.update(dt);
    }
    let _ = puppet.hit_areas_at(Vec2::new(0.0, 0.0));
}

/// ビットを反転した列: どのバイトも、位置で決めた 1 ビットを反転する (8 バイトごとにすべてのビットの位置を通る)。
fn bit_flips(bytes: &[u8]) -> impl Iterator<Item = Vec<u8>> + '_ {
    (0..bytes.len()).map(|i| {
        let mut flipped = bytes.to_vec();
        flipped[i] ^= 1 << (i % 8);
        flipped
    })
}

/// `f` がパニックしなければ `true` (パニックしたら、どの入力かを assert の文で示すため)。
fn catch(f: impl FnOnce()) -> bool {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).is_ok()
}

#[test]
fn truncated_bundles_do_not_panic() {
    for (name, bytes) in bundle_corpus() {
        for len in 0..bytes.len() {
            let result = catch(|| load_and_drive(&bytes[..len]));
            assert!(result, "{name}: truncated to {len} bytes");
        }
    }
}

#[test]
fn bit_flipped_bundles_do_not_panic() {
    for (name, bytes) in bundle_corpus() {
        for (i, flipped) in bit_flips(&bytes).enumerate() {
            let result = catch(|| load_and_drive(&flipped));
            assert!(result, "{name}: bit {} of byte {i} flipped", i % 8);
        }
    }
}

/// 切り詰めた・ビットを反転したモジュールを、着せ替えの本体に取り付けて動かす。
#[test]
fn broken_modules_do_not_panic() {
    let base =
        read_bundle(&std::fs::read(compat_dir("dev").join("dress-up-base.ugoe")).unwrap()).unwrap();
    let base = Puppet::new(base.model).unwrap();
    for (name, bytes) in compat_files("ugoemod") {
        let truncated = (0..bytes.len()).map(|len| bytes[..len].to_vec());
        for (k, broken) in truncated.chain(bit_flips(&bytes)).enumerate() {
            let result = catch(|| {
                let Ok(file) = read_module(&broken) else {
                    return;
                };
                let mut puppet = Puppet::new(base.base_model().clone()).unwrap();
                if puppet.attach_module(Arc::new(file.module)).is_ok() {
                    drive(&mut puppet, &[], &[]);
                }
            });
            assert!(result, "{name}: case {k}");
        }
    }
}

/// パラメータ・リップシンク・トラッキング・基準値・当たり判定の点・`dt` に極端な数を与えても、パニックせず、
/// パラメータと頂点に有限でない値が残らない。
#[test]
fn extreme_host_numbers_do_not_panic() {
    for (model, physics) in fixture_cases() {
        let mut puppet = Puppet::new(model.clone()).unwrap();
        if let Some(physics) = physics.clone() {
            puppet = puppet.with_physics(physics);
        }
        puppet.set_effects(Effects {
            lip_sync: LipSync {
                parameters: vec![standard::MOUTH_OPEN_Y.into(), standard::ANGLE_X.into()],
            },
            ..Effects::default()
        });
        for v in EXTREMES {
            let what = format!("{} {v}", model.meta.name);
            let count = puppet.instance().parameters().len();
            let params = puppet.instance_mut().parameters_mut();
            for index in params.ids().to_vec() {
                let index = params.index_of(index.as_str()).unwrap();
                params.set(index, v);
                params.blend(index, v, v);
            }
            puppet.set_lip_sync_level(Some(v));
            let mut tracking = TrackingInput {
                head: Rotation {
                    yaw: v,
                    pitch: v,
                    roll: v,
                },
                body: BodyInput {
                    yaw: Some(v),
                    pitch: Some(v),
                    roll: Some(v),
                    shoulder_left: Some(v),
                    shoulder_right: Some(v),
                },
                ..TrackingInput::default()
            };
            tracking.blend_shapes.fill(v);
            puppet.set_tracking(Some(tracking));
            for dt in DTS {
                puppet.update(dt);
            }
            let _ = puppet.set_base_values(&vec![v; count]);
            let _ = puppet.follow(
                &vec![v; count],
                Some(LipSyncInput {
                    level: v,
                    strength: v,
                }),
                &[PartOpacityInput {
                    part: 0,
                    value: v,
                    weight: v,
                }],
                v,
            );
            let _ = puppet.hit_areas_at(Vec2::new(v, v));
            let _ = puppet.hit_test_standard("Head", Vec2::new(v, -v));
            let instance = puppet.instance();
            assert!(
                instance
                    .parameters()
                    .as_slice()
                    .iter()
                    .all(|p| p.is_finite()),
                "{what}: {:?}",
                instance.parameters().as_slice()
            );
            for d in instance.drawables() {
                assert!(
                    d.positions.iter().all(|p| p.is_finite()) && d.opacity.is_finite(),
                    "{what}: {d:?}"
                );
            }
            puppet.set_tracking(None);
        }
    }
}

/// 範囲外の番号と知らない ID は、無視するか失敗を返す。
#[test]
fn out_of_range_numbers_and_unknown_ids_do_not_panic() {
    for (model, physics) in fixture_cases() {
        let mut puppet = Puppet::new(model.clone()).unwrap();
        if let Some(physics) = physics.clone() {
            puppet = puppet.with_physics(physics);
        }
        let count = puppet.instance().parameters().len();
        for len in [0, count.saturating_sub(1), count + 1, 10_000] {
            assert!(puppet.set_base_values(&vec![0.0; len]).is_err() || len == count);
        }
        for part in [u32::MAX, u32::MAX - 1, model.parts.len() as u32] {
            let input = PartOpacityInput {
                part,
                value: 0.5,
                weight: 1.0,
            };
            puppet
                .follow(&vec![0.0; count], None, &[input], DT)
                .unwrap();
        }
        assert!(!puppet.hit_test("no such hit area", Vec2::ZERO));
        assert!(puppet.detach_module("no such module").is_err());
        puppet.set_tracking_kept(["no such parameter"]);
        let instance = puppet.instance_mut();
        for i in [
            usize::MAX,
            usize::MAX - 1,
            model.deformers.len(),
            model.meshes.len(),
        ] {
            let offsets = instance.dynamic_offsets_mut();
            offsets.add_angle(i, 1.0);
            offsets.add_origin(i, Vec2::ONE);
            assert!(offsets.warp_points_mut(i).is_none() || i < model.deformers.len());
            assert!(offsets.mesh_positions_mut(i).is_none() || i < model.meshes.len());
            assert!(instance.warp_world_points(i).is_none() || i < model.deformers.len());
            assert!(instance.rotation_world_frame(i).is_none() || i < model.deformers.len());
            assert!(instance.rotation_local_keyform(i).is_none() || i < model.deformers.len());
            if i >= model.parts.len() {
                instance.set_part_opacity_override(i, 0.5, 1.0);
            }
        }
        puppet.update(DT);
    }
}

/// ホストが評価の状態 (パラメータの並びやインスタンスそのもの) を別のモデルのものに差し替えても、パニックしない。
#[test]
fn replacing_the_evaluation_state_with_another_model_does_not_panic() {
    let cases = fixture_cases();
    for (model, physics) in &cases {
        for (other, _) in &cases {
            let mut puppet = Puppet::new(model.clone()).unwrap();
            if let Some(physics) = physics.clone() {
                puppet = puppet.with_physics(physics);
            }
            puppet.update(DT);
            *puppet.instance_mut().parameters_mut() = ParameterValues::from_model(other);
            puppet.update(DT);
            let compiled = Puppet::new(other.clone())
                .unwrap()
                .instance()
                .compiled()
                .clone();
            *puppet.instance_mut() = ModelInstance::new(compiled);
            puppet.update(DT);
        }
    }
}

/// 物理の設定の数をすべて `v` にしたもの (項目ごとに 1 つ)。`Puppet::with_physics` は設定を検査しない。
fn physics_with_extremes(settings: &PhysicsSettings, v: f32) -> Vec<PhysicsSettings> {
    let mut variants = Vec::new();
    let mut each = |f: &mut dyn FnMut(&mut PhysicsSettings)| {
        let mut s = settings.clone();
        f(&mut s);
        variants.push(s);
    };
    each(&mut |s| {
        s.gravity = Vec2::splat(v);
        s.wind = Vec2::splat(-v);
    });
    each(&mut |s| s.fps = v);
    each(&mut |s| {
        for p in s.rigs.iter_mut().flat_map(|r| &mut r.particles) {
            p.position = Vec2::splat(v);
            p.mobility = v;
            p.delay = v;
            p.acceleration = v;
            p.radius = v;
            p.collision_radius = v;
        }
    });
    each(&mut |s| {
        for rig in &mut s.rigs {
            for input in &mut rig.inputs {
                input.weight = v;
            }
            for output in &mut rig.outputs {
                output.scale = v;
                output.weight = v;
            }
            for range in [
                &mut rig.normalization.position,
                &mut rig.normalization.angle,
            ] {
                range.min = v;
                range.default = v;
                range.max = -v;
            }
        }
    });
    each(&mut |s| {
        for c in &mut s.colliders {
            match &mut c.shape {
                ColliderShape::Circle { center, radius } => {
                    *center = Vec2::splat(v);
                    *radius = v;
                }
                ColliderShape::Capsule { a, b, radius } => {
                    *a = Vec2::splat(v);
                    *b = Vec2::splat(-v);
                    *radius = v;
                }
                ColliderShape::Unknown => {}
            }
        }
    });
    variants
}

/// 物理の設定の参照と番号を壊したもの: 範囲外の粒子・粒子の無いリグ・知らない ID。
fn physics_with_broken_references(settings: &PhysicsSettings) -> Vec<PhysicsSettings> {
    let mut variants = Vec::new();
    for particle in [0, 1, u32::MAX, u32::MAX - 1] {
        let mut s = settings.clone();
        for output in s.rigs.iter_mut().flat_map(|r| &mut r.outputs) {
            output.particle = particle;
        }
        variants.push(s);
    }
    for keep in [0, 1] {
        let mut s = settings.clone();
        for rig in &mut s.rigs {
            rig.particles.truncate(keep);
        }
        variants.push(s);
    }
    let mut s = settings.clone();
    for rig in &mut s.rigs {
        for input in &mut rig.inputs {
            input.parameter = "NoSuchParameter".into();
        }
        rig.colliders.push("NoSuchCollider".into());
        rig.frame = Some("NoSuchDeformer".into());
    }
    for c in &mut s.colliders {
        c.attach = Some("NoSuchDeformer".into());
    }
    variants.push(s);
    // 同じ ID のリグを重ねる (中継の入力の解決が前のリグの出力を読む)。
    let mut s = settings.clone();
    s.rigs.extend(settings.rigs.iter().cloned());
    variants.push(s);
    variants
}

/// 検査していない物理の設定 (極端な数・範囲外の番号・知らない ID) でも、パニックせず、パラメータと頂点が有限のまま。
#[test]
fn unchecked_physics_settings_do_not_panic() {
    for (model, physics) in fixture_cases() {
        let Some(settings) = physics else {
            continue;
        };
        let variants = EXTREMES
            .iter()
            .flat_map(|&v| physics_with_extremes(&settings, v))
            .chain(physics_with_broken_references(&settings));
        for (k, variant) in variants.enumerate() {
            let what = format!("{} variant {k}", model.meta.name);
            let ok = catch(|| {
                let mut puppet = Puppet::new(model.clone()).unwrap().with_physics(variant);
                for dt in DTS {
                    puppet.update(dt);
                }
                let instance = puppet.instance();
                assert!(
                    instance
                        .parameters()
                        .as_slice()
                        .iter()
                        .all(|p| p.is_finite()),
                    "{what}: {:?}",
                    instance.parameters().as_slice()
                );
                for d in instance.drawables() {
                    assert!(d.positions.iter().all(|p| p.is_finite()), "{what}: {d:?}");
                }
            });
            assert!(ok, "{what}");
        }
    }
}

/// 時刻と値がすべて `v` の点。
fn point(t: f32, v: f32) -> Point {
    Point { t, v }
}

/// 検査していないモーション: 時刻・値・制御点・長さ・フェード・イベントの時刻が `v` で、区間の順も壊れている。
fn motion_with_extremes(v: f32) -> Motion {
    let segments = vec![
        Segment::Start { p: point(v, v) },
        Segment::Bezier {
            c1: point(v, -v),
            c2: point(-v, v),
            to: point(0.5, v),
        },
        Segment::Linear { to: point(0.0, v) },
        Segment::Stepped { to: point(v, 1.0) },
        Segment::InverseStepped { to: point(-v, 0.0) },
        Segment::Start { p: point(1.0, 1.0) },
    ];
    let curve = |target| Curve {
        target,
        segments: segments.clone(),
        fade_in: Some(v),
        fade_out: Some(-v),
    };
    Motion {
        name: "extremes".into(),
        duration: v,
        looping: v.is_sign_positive(),
        fade_in: v,
        fade_out: v,
        curves: vec![
            curve(CurveTarget::Parameter(standard::ANGLE_X.into())),
            curve(CurveTarget::Parameter("NoSuchParameter".into())),
            curve(CurveTarget::PartOpacity("NoSuchPart".into())),
            curve(CurveTarget::PartOpacity("Root".into())),
            curve(CurveTarget::EffectGroup("LipSync".into())),
            Curve {
                target: CurveTarget::Parameter(standard::ANGLE_Y.into()),
                segments: vec![],
                fade_in: None,
                fade_out: None,
            },
        ],
        events: vec![
            MotionEvent {
                t: v,
                value: "a".into(),
            },
            MotionEvent {
                t: -v,
                value: "b".into(),
            },
        ],
    }
}

/// 検査していないモーション・表情・エフェクトの設定に極端な数を入れても、パニックしない。
#[test]
fn unchecked_motions_expressions_and_effects_do_not_panic() {
    for (model, _) in fixture_cases() {
        for v in EXTREMES {
            let what = format!("{} {v}", model.meta.name);
            let ok = catch(|| {
                let mut puppet = Puppet::new(model.clone()).unwrap();
                puppet.set_effects(Effects {
                    eye_blink: EyeBlink {
                        parameters: vec![standard::EYE_L_OPEN.into(), "NoSuchParameter".into()],
                        interval: v,
                        closing: v,
                        closed: -v,
                        opening: v,
                        seed: u64::MAX,
                    },
                    breath: Breath {
                        parameters: vec![BreathParameter {
                            parameter: standard::BREATH.into(),
                            offset: v,
                            peak: v,
                            cycle: v,
                            weight: v,
                        }],
                    },
                    lip_sync: LipSync {
                        parameters: vec![standard::MOUTH_OPEN_Y.into()],
                    },
                });
                for priority in [Priority::Idle, Priority::Normal, Priority::Force] {
                    puppet.start_motion(motion_with_extremes(v), priority);
                }
                puppet.set_expression(Expression {
                    name: "extremes".into(),
                    fade_in: v,
                    fade_out: -v,
                    parameters: [
                        ExpressionOp::Add,
                        ExpressionOp::Multiply,
                        ExpressionOp::Overwrite,
                    ]
                    .into_iter()
                    .map(|op| ExpressionParameter {
                        parameter: standard::ANGLE_Z.into(),
                        value: v,
                        op,
                    })
                    .collect(),
                });
                for dt in DTS {
                    puppet.update(dt);
                }
            });
            assert!(ok, "{what}");
        }
    }
}
