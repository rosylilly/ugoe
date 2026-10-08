//! `Puppet` の 1 フレームの評価にかかる時間を計測する (T0190、docs/testing.md#性能の計測)。
//!
//! `cargo run --release -p ugoe --example puppet_timing`
//!
//! 性能の目標 (docs/roadmap.md の M7「1 体を 1ms 以内」) と比べるため、ホストが毎フレーム呼ぶ処理の全体を測る。
//!
//! - `update`: `Puppet::update(dt)` の 1 回 (モーション → トラッキング相当の入力 → 表情 → 目パチ → 呼吸 → 物理 →
//!   リップシンク → ポーズ → 評価)。入力 (頭と体の角度・目の開き・リップシンクの音量) は毎フレーム動かし、
//!   ループするモーションを再生し続ける。描画・GPU への転送・ファイルの読み込みは含めない。
//! - `follow`: 受け手の `Puppet::follow`。指揮役の `update` が配る値の列を先に記録しておき、それを順に渡す。
//! - 段の内訳: 物理の段を止めた `update` と、評価 (`ModelInstance::update`) だけ。
//!
//! モデルは 2 つ。
//!
//! - `generated`: 中程度のモデルを模した生成のモデル ([`medium_model`])。頂点は 4,500 ほど、メッシュ 144 枚。
//! - `avatar.ugoe`: 見本のアバター (`assets/samples/avatar.ugoe`。実際のリグの重さの目安)。
//!
//! 表示のフレームレートを 60fps とする。ほかの処理の揺れを除くため、`BATCH` フレームずつの平均を取り、
//! そのうち最も速いものと中央値を示す (`physics_timing` と同じ測り方)。
//!
//! `--check` を付けると、`update` と `follow` の中央値を上限 ([`LIMITS`]) と比べ、超えたものがあれば終了コード 1 で終わる
//! (T0191。`just perf` と週に 1 度の CI の `perf.yml` が流す。上限の決め方は docs/testing.md#遅くなったら気づく検査)。

use std::hint::black_box;
use std::time::{Duration, Instant};

use ugoe::core::{Rgb, Vec2, math};
use ugoe::model::deformer::{RotationDeformer, RotationKeyform, WarpDeformer, WarpKeyform};
use ugoe::model::mesh::MeshKeyform;
use ugoe::model::motion::{Curve, CurveTarget, Motion, Point, Segment};
use ugoe::model::parameter::ParameterKind;
use ugoe::model::part::PartKeyform;
use ugoe::model::physics::{
    ColliderShape, NormalizationRange, PhysicsBlend, PhysicsCollider, PhysicsInput,
    PhysicsNormalization, PhysicsOutput, PhysicsParticle, PhysicsRig, PhysicsSettings,
    PhysicsSource, PhysicsTarget, VertexOwner,
};
use ugoe::model::pose::{DEFAULT_FADE_DURATION, PoseEntry, PoseGroup};
use ugoe::model::{
    BlendMode, Deformer, DeformerKind, Effects, KeyformBinding, KeyformGrid, MaskMode, MaskRef,
    Mesh, Model, Parameter, Part, Pose, fixtures, standard,
};
use ugoe::motion::Priority;
use ugoe::runtime::ParameterValues;
use ugoe::{LipSyncInput, PartOpacityInput, Puppet, Toggles};

const FRAMES: usize = 6_000;
const BATCH: usize = 200;
const DT: f32 = 1.0 / 60.0;

/// `--check` の上限: (モデル, 計測, 1 フレームの中央値の上限 [µs])。決め方は docs/testing.md#遅くなったら気づく検査。
///
/// 手元 (Ryzen 9 7900) の値 (生成のモデル 55µs ほど、`avatar.ugoe` 18µs ほど) に、CI のランナーの遅さと揺れを見込み
/// (2 倍 × 1.3 倍ほど)、さらに 2 倍ほどの余裕を持たせた。生成のモデルの上限は、中程度の PC がランナーの 3 倍遅くても
/// 目標の 1ms に収まる値でもある。
const LIMITS: [(&str, &str, u64); 4] = [
    ("generated", "update", 300),
    ("generated", "follow", 300),
    ("avatar.ugoe", "update", 100),
    ("avatar.ugoe", "follow", 100),
];

/// `--check` で上限を超えたときに測り直す回数。ほかの処理と重なった一時の揺れで落ちないように、最も速い中央値で判定する。
const RETRIES: usize = 2;

fn main() {
    let generated = medium_model();
    let errors = ugoe::model::validate::validate(&generated);
    assert!(errors.is_empty(), "{errors:?}");
    let generated = Subject {
        name: "generated",
        model: generated,
        physics: Some(medium_physics()),
        motion: Some(idle_motion()),
    };
    check_physics_moves(&generated);

    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/samples/avatar.ugoe"
    );
    let bytes = std::fs::read(path).expect("read avatar.ugoe");
    let bundle = ugoe::format::read_bundle(&bytes).expect("avatar.ugoe");
    let motion = bundle
        .motions
        .iter()
        .find(|m| m.looping)
        .or(bundle.motions.first())
        .cloned();
    let avatar = Subject {
        name: "avatar.ugoe",
        model: bundle.model,
        physics: bundle.physics,
        motion,
    };

    println!(
        "{:12} {:>6} {:>9} {:>9} {:>6} {:>6} {:>8} {:>9}  motion",
        "model", "meshes", "vertices", "triangles", "warps", "rots", "params", "physics"
    );
    for subject in [&generated, &avatar] {
        subject.describe();
    }
    println!();
    println!("per frame (us), batches of {BATCH} frames out of {FRAMES}: best / median");
    // `--check` で上限と比べる中央値: (モデル, 計測, 中央値)
    let mut medians = Vec::new();
    for subject in [&generated, &avatar] {
        let update = measure_update(subject, Toggles::default());
        report(subject.name, "update", update);
        medians.push((subject.name, "update", update.1));
        let follow = measure_follow(subject);
        report(subject.name, "follow", follow);
        medians.push((subject.name, "follow", follow.1));
        let no_physics = measure_update(
            subject,
            Toggles {
                physics: false,
                ..Toggles::default()
            },
        );
        report(subject.name, "update without physics", no_physics);
        report(subject.name, "evaluation only", measure_evaluation(subject));
    }

    if std::env::args().skip(1).any(|arg| arg == "--check") {
        let over = check(&[&generated, &avatar], &medians);
        if over > 0 {
            eprintln!("{over} measurement(s) over the limit (docs/testing.md)");
            std::process::exit(1);
        }
    }
}

/// 測った中央値を [`LIMITS`] と比べ、上限を超えた計測の数を返す。超えたものは [`RETRIES`] 回まで測り直す。
fn check(subjects: &[&Subject], medians: &[(&str, &str, Duration)]) -> usize {
    println!();
    println!("check (us): median / limit");
    let mut over = 0;
    for (model, name, limit) in LIMITS {
        let subject = subjects
            .iter()
            .find(|s| s.name == model)
            .expect("subject in LIMITS");
        let measure = || match name {
            "update" => measure_update(subject, Toggles::default()).1,
            "follow" => measure_follow(subject).1,
            _ => unreachable!("unknown measurement in LIMITS: {name}"),
        };
        let limit = Duration::from_micros(limit);
        let mut median = medians
            .iter()
            .find(|&&(m, n, _)| m == model && n == name)
            .map(|&(_, _, median)| median)
            .expect("measured in main");
        for _ in 0..RETRIES {
            if median <= limit {
                break;
            }
            median = median.min(measure());
        }
        let ok = median <= limit;
        println!(
            "  {model:12} {name:24} {:>9.1} / {:>9.1}  {}",
            median.as_secs_f64() * 1e6,
            limit.as_secs_f64() * 1e6,
            if ok { "ok" } else { "OVER" },
        );
        if !ok {
            over += 1;
        }
    }
    over
}

/// 生成のモデルの物理のリグが形を動かしていること (計測が空回りしていないこと) を確かめる。
fn check_physics_moves(subject: &Subject) {
    let positions = |physics: bool| {
        let mut puppet = subject.puppet();
        puppet.set_toggles(Toggles {
            physics,
            ..Toggles::default()
        });
        for k in 0..120 {
            drive(&mut puppet, k);
            puppet.update(DT);
        }
        let compiled = puppet.instance().compiled().clone();
        ["HairFront0", "Strand0_2Mesh0", "Skirt0"].map(|id| {
            let index = compiled.mesh_index(id).expect("mesh");
            let positions = puppet.instance().drawables()[index].positions.clone();
            assert!(positions.iter().all(|p| p.x.is_finite() && p.y.is_finite()));
            positions
        })
    };
    let (with, without) = (positions(true), positions(false));
    for (a, b) in with.iter().zip(&without) {
        assert_ne!(a, b, "physics should move the mesh");
    }
}

/// 計測するモデルと、それに付ける物理・モーション。
struct Subject {
    name: &'static str,
    model: Model,
    physics: Option<PhysicsSettings>,
    motion: Option<Motion>,
}

impl Subject {
    fn puppet(&self) -> Puppet {
        let mut puppet = Puppet::new(self.model.clone()).expect("compile");
        if let Some(physics) = &self.physics {
            puppet
                .set_physics(Some(physics.clone()))
                .expect("valid physics");
        }
        puppet
    }

    fn describe(&self) {
        let model = &self.model;
        let vertices: usize = model.meshes.iter().map(|m| m.uvs.len()).sum();
        let triangles: usize = model.meshes.iter().map(|m| m.indices.len() / 3).sum();
        let warps = model
            .deformers
            .iter()
            .filter(|d| matches!(d.kind, DeformerKind::Warp(_)))
            .count();
        let rotations = model
            .deformers
            .iter()
            .filter(|d| matches!(d.kind, DeformerKind::Rotation(_)))
            .count();
        let rigs = self.physics.as_ref().map_or(0, |p| p.rigs.len());
        println!(
            "{:12} {:>6} {:>9} {:>9} {:>6} {:>6} {:>8} {:>4} rigs  {}",
            self.name,
            model.meshes.len(),
            vertices,
            triangles,
            warps,
            rotations,
            model.parameters.len(),
            rigs,
            self.motion.as_ref().map_or("-", |m| m.name.as_str()),
        );
    }
}

/// フレーム `k` の、トラッキング相当の入力とリップシンクの音量を書く。
fn drive(puppet: &mut Puppet, k: usize) {
    let t = k as f32 * DT;
    let params = puppet.instance_mut().parameters_mut();
    set(params, standard::ANGLE_X, 25.0 * math::sin(t * 1.3));
    set(params, standard::ANGLE_Y, 15.0 * math::sin(t * 0.9 + 1.0));
    set(params, standard::ANGLE_Z, 10.0 * math::cos(t * 0.7));
    set(params, standard::BODY_ANGLE_X, 6.0 * math::sin(t * 0.5));
    set(params, standard::BODY_ANGLE_Y, 4.0 * math::cos(t * 0.4));
    let eye = 0.85 + 0.15 * math::sin(t * 2.1);
    set(params, standard::EYE_L_OPEN, eye);
    set(params, standard::EYE_R_OPEN, eye);
    set(
        params,
        standard::EYE_L_SMILE,
        0.5 + 0.5 * math::sin(t * 0.3),
    );
    set(
        params,
        standard::EYE_R_SMILE,
        0.5 + 0.5 * math::sin(t * 0.3),
    );
    puppet.set_lip_sync_level(Some(0.5 + 0.5 * math::sin(t * 9.0)));
}

fn set(params: &mut ParameterValues, id: &str, value: f32) {
    if let Some(index) = params.index_of(id) {
        params.set(index, value);
    }
}

/// モーションが終わっていたら (ループしないモーションの場合) 始め直す。
fn keep_playing(puppet: &mut Puppet, motion: Option<&Motion>) {
    if let Some(motion) = motion
        && !puppet.motions().is_playing()
    {
        puppet.start_motion(motion.clone(), Priority::Force);
    }
}

/// `BATCH` フレームずつの 1 フレームあたりの時間。
#[derive(Default)]
struct Batches {
    total: Duration,
    frames: usize,
    averages: Vec<Duration>,
}

impl Batches {
    fn add(&mut self, elapsed: Duration) {
        self.total += elapsed;
        self.frames += 1;
        if self.frames == BATCH {
            self.averages.push(self.total / BATCH as u32);
            self.total = Duration::ZERO;
            self.frames = 0;
        }
    }

    /// (最も速いもの, 中央値)。
    fn summary(mut self) -> (Duration, Duration) {
        self.averages.sort();
        (self.averages[0], self.averages[self.averages.len() / 2])
    }
}

fn report(model: &str, name: &str, (best, median): (Duration, Duration)) {
    println!(
        "  {model:12} {name:24} {:>9.1} / {:>9.1}",
        best.as_secs_f64() * 1e6,
        median.as_secs_f64() * 1e6,
    );
}

fn measure_update(subject: &Subject, toggles: Toggles) -> (Duration, Duration) {
    let mut puppet = subject.puppet();
    puppet.set_toggles(toggles);
    let mut batches = Batches::default();
    for k in 0..FRAMES {
        keep_playing(&mut puppet, subject.motion.as_ref());
        drive(&mut puppet, k);
        let start = Instant::now();
        puppet.update(DT);
        batches.add(start.elapsed());
        black_box(puppet.instance().drawables());
    }
    batches.summary()
}

/// 指揮役の `update` が配る 1 フレーム分の値。
struct Sent {
    values: Vec<f32>,
    lip_sync: Option<LipSyncInput>,
    part_opacities: Vec<PartOpacityInput>,
}

fn measure_follow(subject: &Subject) -> (Duration, Duration) {
    let mut director = subject.puppet();
    let sent: Vec<Sent> = (0..FRAMES)
        .map(|k| {
            keep_playing(&mut director, subject.motion.as_ref());
            drive(&mut director, k);
            director.update(DT);
            Sent {
                values: director.values_before_physics().to_vec(),
                lip_sync: director.lip_sync_applied(),
                part_opacities: director.part_opacities_applied().to_vec(),
            }
        })
        .collect();
    let mut receiver = subject.puppet();
    let mut batches = Batches::default();
    for frame in &sent {
        let start = Instant::now();
        receiver
            .follow(&frame.values, frame.lip_sync, &frame.part_opacities, DT)
            .expect("follow");
        batches.add(start.elapsed());
        black_box(receiver.instance().drawables());
    }
    batches.summary()
}

/// 評価 (`ModelInstance::update`) だけ。入力は `update` と同じに動かす (モーション・効果・物理は通さない)。
fn measure_evaluation(subject: &Subject) -> (Duration, Duration) {
    let mut puppet = subject.puppet();
    let mut batches = Batches::default();
    for k in 0..FRAMES {
        drive(&mut puppet, k);
        let instance = puppet.instance_mut();
        let start = Instant::now();
        instance.update();
        batches.add(start.elapsed());
        black_box(instance.drawables());
    }
    batches.summary()
}

// ---- 中程度のモデルを模した生成のモデル ----

/// 中程度のモデルを模した生成のモデル。キャンバスは 1024×1024。
///
/// - パラメータ: 標準のパラメータ (`standard::ALL`。角度・目・眉・口・体・呼吸・髪の揺れ)。
/// - デフォーマ: 体のワープ (体の角度 X・Y × 呼吸の 3 軸) → 体の回転 → 頭の回転 → 顔のワープ (角度 X × Y) →
///   目・目玉・眉・口・前髪・横髪のワープ。頭の回転の下に後ろ髪のワープと、物理で揺らす髪の房 (回転 3 段 × 6 本)。
///   体の回転の下に腕 (回転 2 段 × 左右 × ポーズの 2 種)、体のワープの下にスカートのワープ。
/// - メッシュ 144 枚、頂点 4,532。顔・目・口・体のメッシュは自身のキーフォーム (2 軸の格子) も持つ。
///   目玉は白目で、口の中は口の輪郭で、顔の影は顔の地でマスクする。
/// - ポーズ: 腕の 2 種 (`ArmA`・`ArmB`)。エフェクト: 標準 (目パチ・呼吸・リップシンク)。
fn medium_model() -> Model {
    let mut model = fixtures::single_quad();
    model.meta.name = "puppet_timing".into();
    model.meta.canvas.width = 1024.0;
    model.meta.canvas.height = 1024.0;
    model.meta.canvas.origin = Vec2::new(512.0, 512.0);
    model.parameters = standard::ALL
        .iter()
        .map(|&(id, min, default, max)| Parameter {
            id: id.into(),
            name: id.into(),
            min,
            max,
            default,
            kind: ParameterKind::Normal,
            mirror: None,
            group: None,
            internal: false,
        })
        .collect();
    model.parts = [
        ("Root", None, 1.0),
        ("Body", Some("Root"), 1.0),
        ("Head", Some("Root"), 1.0),
        ("Hair", Some("Head"), 1.0),
        ("Face", Some("Head"), 1.0),
        ("ArmA", Some("Body"), 1.0),
        ("ArmB", Some("Body"), 0.0),
    ]
    .into_iter()
    .map(|(id, parent, opacity)| Part {
        id: id.into(),
        name: id.into(),
        parent: parent.map(Into::into),
        keyforms: KeyformGrid::constant(PartKeyform { opacity }),
    })
    .collect();
    model.deformers = deformers();
    model.meshes = meshes();
    model.pose = Pose {
        fade_duration: DEFAULT_FADE_DURATION,
        groups: vec![PoseGroup {
            entries: vec![
                PoseEntry {
                    part: "ArmA".into(),
                    links: vec![],
                },
                PoseEntry {
                    part: "ArmB".into(),
                    links: vec![],
                },
            ],
        }],
    };
    model.effects = Effects::standard();
    model
}

/// 物理で揺らす髪の房の数。
const STRANDS: usize = 6;
/// 髪の房の回転デフォーマの段数。
const STRAND_SEGMENTS: usize = 3;
/// 髪の房の 1 段の長さ (px)。
const STRAND_LENGTH: f32 = 50.0;
/// スカートのワープの格子 (列, 行)。
const SKIRT_GRID: (u16, u16) = (4, 2);

/// パラメータ `id` のキー。最小・既定・最大 (既定が端なら 2 つ)。
fn keys(id: &str) -> Vec<f32> {
    let (min, default, max) = standard::range(id).expect("standard parameter");
    let mut keys = vec![min, default, max];
    keys.dedup();
    keys
}

/// `params` を軸にした格子。セルは各軸の値を -1〜1 (既定が 0) に直したもので作る。先頭の軸が最も速く変わる。
fn grid<T>(params: &[&str], cell: impl Fn(&[f32]) -> T) -> KeyformGrid<T> {
    let axes: Vec<Vec<f32>> = params
        .iter()
        .map(|id| {
            let (min, default, max) = standard::range(id).expect("standard parameter");
            keys(id)
                .iter()
                .map(|&k| {
                    if k > default {
                        (k - default) / (max - default)
                    } else if k < default {
                        (k - default) / (default - min)
                    } else {
                        0.0
                    }
                })
                .collect()
        })
        .collect();
    let count: usize = axes.iter().map(Vec::len).product();
    let cells = (0..count)
        .map(|mut i| {
            let values: Vec<f32> = axes
                .iter()
                .map(|n| {
                    let v = n[i % n.len()];
                    i /= n.len();
                    v
                })
                .collect();
            cell(&values)
        })
        .collect();
    let bindings = params
        .iter()
        .map(|id| KeyformBinding {
            parameter: (*id).into(),
            keys: keys(id),
        })
        .collect();
    KeyformGrid::new(bindings, cells)
}

/// 制御点 `(cols + 1) × (rows + 1)` (行優先、x が速い) が `rect` に並ぶワープ。各軸の値 `v` で、
/// 先頭の軸で上の行ほど横へ、2 番目の軸で中央の列ほど縦へずらし、3 番目の軸で横に膨らませる。
fn warp(
    id: &str,
    parent: Option<&str>,
    (cols, rows): (u16, u16),
    rect: [f32; 4],
    params: &[&str],
) -> Deformer {
    let [x0, y0, x1, y1] = rect;
    let (w, h) = (x1 - x0, y1 - y0);
    let keyforms = grid(params, |v| {
        let dx = v.first().copied().unwrap_or(0.0) * 0.08 * w;
        let dy = v.get(1).copied().unwrap_or(0.0) * 0.05 * h;
        let swell = 1.0 + v.get(2).copied().unwrap_or(0.0) * 0.02;
        let points = (0..=rows)
            .flat_map(|y| (0..=cols).map(move |x| (x, y)))
            .map(|(x, y)| {
                let (u, t) = (
                    f32::from(x) / f32::from(cols),
                    f32::from(y) / f32::from(rows),
                );
                let lift = 1.0 - t;
                Vec2::new(
                    x0 + w * (0.5 + (u - 0.5) * swell) + dx * lift,
                    y0 + h * t + dy * lift * math::sin(u * std::f32::consts::PI),
                )
            })
            .collect();
        WarpKeyform {
            points,
            opacity: 1.0,
        }
    });
    Deformer {
        id: id.into(),
        name: id.into(),
        part: "Root".into(),
        parent: parent.map(Into::into),
        kind: DeformerKind::Warp(WarpDeformer {
            rows,
            cols,
            keyforms,
            depth: None,
        }),
        mirror: None,
        mirror_axis: None,
    }
}

/// 原点 `origin` の回転デフォーマ。軸があれば、先頭の軸の値 × `degrees` 度だけ回す。
fn rotation(id: &str, parent: &str, origin: Vec2, params: &[&str], degrees: f32) -> Deformer {
    let keyforms = grid(params, |v| RotationKeyform {
        origin,
        angle: v.first().copied().unwrap_or(0.0) * degrees,
        scale: 1.0,
        reflect_x: false,
        reflect_y: false,
        opacity: 1.0,
    });
    Deformer {
        id: id.into(),
        name: id.into(),
        part: "Root".into(),
        parent: Some(parent.into()),
        kind: DeformerKind::Rotation(RotationDeformer { keyforms }),
        mirror: None,
        mirror_axis: None,
    }
}

fn strand(k: usize, segment: usize) -> String {
    format!("Strand{k}_{segment}")
}

fn deformers() -> Vec<Deformer> {
    use standard::*;
    // 顔のワープの中の部位 (顔の UV 空間)。
    let unit = [0.0, 0.0, 1.0, 1.0];
    let mut deformers = vec![
        warp(
            "Body",
            None,
            (5, 5),
            [312.0, 420.0, 712.0, 1000.0],
            &[BODY_ANGLE_X, BODY_ANGLE_Y, BREATH],
        ),
        rotation(
            "BodyRot",
            "Body",
            Vec2::new(0.5, 0.15),
            &[BODY_ANGLE_Z],
            10.0,
        ),
        rotation(
            "HeadRot",
            "BodyRot",
            Vec2::new(0.0, -20.0),
            &[ANGLE_Z],
            15.0,
        ),
        warp(
            "Face",
            Some("HeadRot"),
            (5, 5),
            [-160.0, -340.0, 160.0, 0.0],
            &[ANGLE_X, ANGLE_Y],
        ),
        warp(
            "HairBack",
            Some("HeadRot"),
            (4, 4),
            [-200.0, -360.0, 200.0, 60.0],
            &[HAIR_BACK, ANGLE_X],
        ),
        warp(
            "EyeL",
            Some("Face"),
            (3, 3),
            [0.2, 0.45, 0.45, 0.6],
            &[EYE_L_OPEN, EYE_L_SMILE],
        ),
        warp(
            "EyeR",
            Some("Face"),
            (3, 3),
            [0.55, 0.45, 0.8, 0.6],
            &[EYE_R_OPEN, EYE_R_SMILE],
        ),
        warp(
            "EyeBallL",
            Some("EyeL"),
            (2, 2),
            unit,
            &[EYE_BALL_X, EYE_BALL_Y],
        ),
        warp(
            "EyeBallR",
            Some("EyeR"),
            (2, 2),
            unit,
            &[EYE_BALL_X, EYE_BALL_Y],
        ),
        warp(
            "BrowL",
            Some("Face"),
            (3, 3),
            [0.2, 0.35, 0.45, 0.42],
            &[BROW_L_Y, ANGLE_X],
        ),
        warp(
            "BrowR",
            Some("Face"),
            (3, 3),
            [0.55, 0.35, 0.8, 0.42],
            &[BROW_R_Y, ANGLE_X],
        ),
        warp(
            "Mouth",
            Some("Face"),
            (3, 3),
            [0.35, 0.72, 0.65, 0.85],
            &[MOUTH_OPEN_Y, MOUTH_FORM],
        ),
        warp(
            "HairFront",
            Some("Face"),
            (4, 4),
            [0.0, 0.0, 1.0, 0.45],
            &[HAIR_FRONT, ANGLE_X],
        ),
        warp(
            "HairSideL",
            Some("Face"),
            (3, 3),
            [-0.1, 0.2, 0.15, 0.9],
            &[HAIR_SIDE, ANGLE_X],
        ),
        warp(
            "HairSideR",
            Some("Face"),
            (3, 3),
            [0.85, 0.2, 1.1, 0.9],
            &[HAIR_SIDE, ANGLE_X],
        ),
        warp(
            "Skirt",
            Some("Body"),
            SKIRT_GRID,
            [0.1, 0.7, 0.9, 1.0],
            &[BODY_ANGLE_X],
        ),
    ];
    for k in 0..STRANDS {
        let x = -150.0 + 60.0 * k as f32;
        deformers.push(rotation(
            &strand(k, 0),
            "HeadRot",
            Vec2::new(x, -80.0),
            &[],
            0.0,
        ));
        for s in 1..STRAND_SEGMENTS {
            deformers.push(rotation(
                &strand(k, s),
                &strand(k, s - 1),
                Vec2::new(0.0, STRAND_LENGTH),
                &[],
                0.0,
            ));
        }
    }
    for arm in ["ArmA", "ArmB"] {
        for (side, x) in [("L", -120.0), ("R", 120.0)] {
            let upper = format!("{arm}{side}0");
            deformers.push(rotation(
                &upper,
                "BodyRot",
                Vec2::new(x, 40.0),
                &[BODY_ANGLE_X],
                20.0,
            ));
            deformers.push(rotation(
                &format!("{arm}{side}1"),
                &upper,
                Vec2::new(0.0, 140.0),
                &[BODY_ANGLE_Y],
                15.0,
            ));
        }
    }
    deformers
}

/// 頂点 `nx × ny` の格子のメッシュ。位置は親の空間の `rect`。軸があれば自身のキーフォームの格子を持つ。
#[allow(clippy::too_many_arguments)]
fn mesh(
    id: &str,
    part: &str,
    parent: &str,
    (nx, ny): (u16, u16),
    rect: [f32; 4],
    params: &[&str],
    order: f32,
    masks: Vec<MaskRef>,
) -> Mesh {
    let uvs: Vec<Vec2> = (0..ny)
        .flat_map(|y| (0..nx).map(move |x| (x, y)))
        .map(|(x, y)| {
            Vec2::new(
                f32::from(x) / f32::from(nx - 1),
                f32::from(y) / f32::from(ny - 1),
            )
        })
        .collect();
    let mut indices = Vec::new();
    for y in 0..ny - 1 {
        for x in 0..nx - 1 {
            let i = y * nx + x;
            indices.extend([i, i + 1, i + nx, i + 1, i + nx + 1, i + nx]);
        }
    }
    let [x0, y0, x1, y1] = rect;
    let (w, h) = (x1 - x0, y1 - y0);
    let keyforms = grid(params, |v| {
        let a = v.first().copied().unwrap_or(0.0);
        let b = v.get(1).copied().unwrap_or(0.0);
        MeshKeyform {
            positions: uvs
                .iter()
                .map(|uv| {
                    Vec2::new(
                        x0 + w * uv.x + 0.05 * w * a * uv.y,
                        y0 + h * uv.y * (1.0 + 0.1 * b),
                    )
                })
                .collect(),
            opacity: 1.0,
            draw_order: order,
            multiply_color: Rgb::WHITE,
            screen_color: Rgb::BLACK,
            mask_strength: 1.0,
        }
    });
    Mesh {
        id: id.into(),
        name: id.into(),
        part: part.into(),
        parent: Some(parent.into()),
        texture: 0,
        uvs,
        indices,
        blend: BlendMode::Normal,
        culling: false,
        masks,
        mask_mode: MaskMode::Union,
        mask_only: false,
        keyforms,
        mirror: None,
        mirror_axis: None,
        depth: None,
        lit: true,
    }
}

/// メッシュの組: 親・パーツ・枚数・頂点の格子・自身のキーフォームの軸・描画順の基準・マスク
/// (組の何枚目から、どのメッシュでマスクするか)。
struct Group<'a> {
    prefix: String,
    parent: String,
    part: &'a str,
    count: usize,
    size: (u16, u16),
    rect: [f32; 4],
    params: &'a [&'a str],
    order: f32,
    mask: Option<(usize, String)>,
}

fn meshes() -> Vec<Mesh> {
    use standard::*;
    let inner = [0.1, 0.1, 0.9, 0.9];
    let group = |prefix: &str,
                 parent: &str,
                 part: &'static str,
                 count: usize,
                 size: (u16, u16),
                 rect: [f32; 4],
                 params: &'static [&'static str],
                 order: f32| Group {
        prefix: prefix.into(),
        parent: parent.into(),
        part,
        count,
        size,
        rect,
        params,
        order,
        mask: None,
    };
    let masked = |g: Group<'static>, from: usize, by: &str| Group {
        mask: Some((from, by.into())),
        ..g
    };
    let mut groups = vec![
        group(
            "HairBack",
            "HairBack",
            "Hair",
            12,
            (7, 7),
            inner,
            &[],
            100.0,
        ),
        group(
            "Body",
            "Body",
            "Body",
            20,
            (6, 6),
            inner,
            &[BODY_ANGLE_X, BODY_ANGLE_Y],
            200.0,
        ),
        group("Skirt", "Skirt", "Body", 10, (6, 6), inner, &[], 300.0),
        masked(
            group(
                "Face",
                "Face",
                "Face",
                8,
                (7, 7),
                inner,
                &[ANGLE_X, ANGLE_Y],
                400.0,
            ),
            6,
            "Face0",
        ),
        group(
            "EyeL",
            "EyeL",
            "Face",
            5,
            (5, 5),
            inner,
            &[EYE_L_OPEN, EYE_L_SMILE],
            500.0,
        ),
        group(
            "EyeR",
            "EyeR",
            "Face",
            5,
            (5, 5),
            inner,
            &[EYE_R_OPEN, EYE_R_SMILE],
            500.0,
        ),
        masked(
            group(
                "EyeBallL",
                "EyeBallL",
                "Face",
                2,
                (5, 5),
                [0.3, 0.3, 0.7, 0.7],
                &[],
                510.0,
            ),
            0,
            "EyeL0",
        ),
        masked(
            group(
                "EyeBallR",
                "EyeBallR",
                "Face",
                2,
                (5, 5),
                [0.3, 0.3, 0.7, 0.7],
                &[],
                510.0,
            ),
            0,
            "EyeR0",
        ),
        group("BrowL", "BrowL", "Face", 2, (4, 4), inner, &[], 520.0),
        group("BrowR", "BrowR", "Face", 2, (4, 4), inner, &[], 520.0),
        masked(
            group(
                "Mouth",
                "Mouth",
                "Face",
                6,
                (5, 5),
                inner,
                &[MOUTH_OPEN_Y, MOUTH_FORM],
                530.0,
            ),
            2,
            "Mouth0",
        ),
        group(
            "HairFront",
            "HairFront",
            "Hair",
            16,
            (6, 6),
            inner,
            &[],
            600.0,
        ),
        group(
            "HairSideL",
            "HairSideL",
            "Hair",
            6,
            (6, 6),
            inner,
            &[],
            610.0,
        ),
        group(
            "HairSideR",
            "HairSideR",
            "Hair",
            6,
            (6, 6),
            inner,
            &[],
            610.0,
        ),
    ];
    for k in 0..STRANDS {
        for s in 0..STRAND_SEGMENTS {
            groups.push(group(
                &format!("{}Mesh", strand(k, s)),
                &strand(k, s),
                "Hair",
                1,
                (3, 6),
                [-10.0, 0.0, 10.0, STRAND_LENGTH],
                &[],
                150.0,
            ));
        }
    }
    for arm in ["ArmA", "ArmB"] {
        for side in ["L", "R"] {
            for segment in 0..2 {
                groups.push(group(
                    &format!("{arm}{side}{segment}Mesh"),
                    &format!("{arm}{side}{segment}"),
                    arm,
                    3,
                    (4, 6),
                    [-30.0, 0.0, 30.0, 150.0],
                    &[],
                    250.0,
                ));
            }
        }
    }
    groups
        .iter()
        .flat_map(|g| {
            (0..g.count).map(move |i| {
                let id = format!("{}{i}", g.prefix);
                // 同じ組の中で少しずつずらす。
                let d = 0.01 * i as f32 * (g.rect[2] - g.rect[0]);
                let [x0, y0, x1, y1] = g.rect;
                let masks = match &g.mask {
                    Some((from, by)) if i >= *from => vec![MaskRef::normal(by.as_str())],
                    _ => Vec::new(),
                };
                mesh(
                    &id,
                    g.part,
                    &g.parent,
                    g.size,
                    [x0 + d, y0, x1 + d, y1],
                    g.params,
                    g.order + i as f32,
                    masks,
                )
            })
        })
        .collect()
}

/// [`medium_model`] 用の物理。
///
/// - 静的なリグ: 前髪・横髪・後ろ髪の揺れ (`HairFront`・`HairSide`・`HairBack` に書く)。
/// - 動的なリグ: 髪の房 6 本 (粒子 4 つ。回転デフォーマ 3 段の角度に足す。肩の衝突体と当たる) と、
///   スカート (スカートのワープの下の行の制御点を動かす)。
fn medium_physics() -> PhysicsSettings {
    let range = NormalizationRange {
        min: -10.0,
        default: 0.0,
        max: 10.0,
    };
    let normalization = PhysicsNormalization {
        position: range,
        angle: range,
    };
    let input = |parameter: &str, source: PhysicsSource, weight: f32| PhysicsInput {
        parameter: parameter.into(),
        source,
        weight,
        reflect: false,
    };
    let inputs = || {
        vec![
            input(standard::ANGLE_X, PhysicsSource::X, 60.0),
            input(standard::ANGLE_Z, PhysicsSource::Angle, 60.0),
            input(standard::BODY_ANGLE_X, PhysicsSource::X, 40.0),
        ]
    };
    let output =
        |target: PhysicsTarget, particle: u32, source: PhysicsSource, scale: f32| PhysicsOutput {
            target,
            particle,
            source,
            scale,
            weight: 100.0,
            reflect: false,
            blend: PhysicsBlend::Overwrite,
        };
    let particles = |count: usize, length: f32, collision_radius: f32| {
        (0..count)
            .map(|i| PhysicsParticle {
                position: Vec2::new(0.0, length * i as f32),
                mobility: if i == 0 { 1.0 } else { 0.9 },
                delay: if i == 0 { 1.0 } else { 0.85 },
                acceleration: 1.5,
                radius: if i == 0 { 0.0 } else { length },
                collision_radius: if i == 0 { 0.0 } else { collision_radius },
            })
            .collect()
    };
    let mut rigs: Vec<PhysicsRig> = [
        standard::HAIR_FRONT,
        standard::HAIR_SIDE,
        standard::HAIR_BACK,
    ]
    .iter()
    .map(|&id| PhysicsRig {
        id: id.into(),
        name: id.into(),
        inputs: inputs(),
        outputs: vec![output(
            PhysicsTarget::Parameter { id: id.into() },
            2,
            PhysicsSource::Angle,
            0.1,
        )],
        particles: particles(3, 10.0, 0.0),
        normalization,
        colliders: Vec::new(),
        frame: None,
    })
    .collect();
    for k in 0..STRANDS {
        rigs.push(PhysicsRig {
            id: strand(k, 0).as_str().into(),
            name: strand(k, 0),
            inputs: inputs(),
            outputs: (0..STRAND_SEGMENTS)
                .map(|s| {
                    output(
                        PhysicsTarget::DeformerAngle {
                            deformer: strand(k, s).as_str().into(),
                        },
                        s as u32 + 1,
                        PhysicsSource::Angle,
                        1.0,
                    )
                })
                .collect(),
            particles: particles(STRAND_SEGMENTS + 1, STRAND_LENGTH, 4.0),
            normalization,
            colliders: vec!["Shoulder".into()],
            frame: Some("HeadRot".into()),
        });
    }
    let (cols, rows) = SKIRT_GRID;
    let bottom_row: Vec<f32> = (0..=rows)
        .flat_map(|y| (0..=cols).map(move |_| if y == rows { 1.0 } else { 0.0 }))
        .collect();
    let skirt = || PhysicsTarget::Vertices {
        owner: VertexOwner::Warp("Skirt".into()),
        weights: bottom_row.clone(),
    };
    rigs.push(PhysicsRig {
        id: "Skirt".into(),
        name: "Skirt".into(),
        inputs: inputs(),
        outputs: vec![
            output(skirt(), 1, PhysicsSource::X, 0.002),
            output(skirt(), 1, PhysicsSource::Y, 0.002),
        ],
        particles: particles(2, 40.0, 0.0),
        normalization,
        colliders: Vec::new(),
        frame: Some("BodyRot".into()),
    });
    PhysicsSettings {
        gravity: Vec2::new(0.0, 1.0),
        wind: Vec2::ZERO,
        fps: 60.0,
        rigs,
        colliders: vec![PhysicsCollider {
            id: "Shoulder".into(),
            name: "Shoulder".into(),
            attach: Some("BodyRot".into()),
            shape: ColliderShape::Capsule {
                a: Vec2::new(-140.0, 30.0),
                b: Vec2::new(140.0, 30.0),
                radius: 20.0,
            },
        }],
    }
}

/// 4 秒でループする待機モーション。体の傾き・目玉・眉・口の形・頬をベジェで動かし、2 秒ごとに腕のポーズを
/// 切り替え (`ArmB` の不透明度)、リップシンクのエフェクトグループの重みを 1 に保つ。
fn idle_motion() -> Motion {
    const DURATION: f32 = 4.0;
    let wave = |target: CurveTarget, amplitude: f32| {
        let p = |t: f32, v: f32| Point { t, v };
        let quarter = DURATION / 4.0;
        let mut segments = vec![Segment::Start { p: p(0.0, 0.0) }];
        for (i, v) in [amplitude, 0.0, -amplitude, 0.0].into_iter().enumerate() {
            let t0 = quarter * i as f32;
            let t1 = t0 + quarter;
            let prev = if i == 0 {
                0.0
            } else {
                [amplitude, 0.0, -amplitude][i - 1]
            };
            segments.push(Segment::Bezier {
                c1: p(t0 + quarter / 3.0, prev),
                c2: p(t1 - quarter / 3.0, v),
                to: p(t1, v),
            });
        }
        Curve {
            target,
            segments,
            fade_in: None,
            fade_out: None,
        }
    };
    let parameter = |id: &str| CurveTarget::Parameter(id.into());
    let arm = Curve {
        target: CurveTarget::PartOpacity("ArmB".into()),
        segments: vec![
            Segment::Start {
                p: Point { t: 0.0, v: 0.0 },
            },
            Segment::Stepped {
                to: Point { t: 2.0, v: 1.0 },
            },
            Segment::Stepped {
                to: Point {
                    t: DURATION,
                    v: 1.0,
                },
            },
        ],
        fade_in: None,
        fade_out: None,
    };
    let lip_sync = Curve {
        target: CurveTarget::EffectGroup("LipSync".into()),
        segments: vec![
            Segment::Start {
                p: Point { t: 0.0, v: 1.0 },
            },
            Segment::Linear {
                to: Point {
                    t: DURATION,
                    v: 1.0,
                },
            },
        ],
        fade_in: None,
        fade_out: None,
    };
    Motion {
        name: "idle".into(),
        duration: DURATION,
        looping: true,
        fade_in: 0.5,
        fade_out: 0.5,
        curves: vec![
            wave(parameter(standard::BODY_ANGLE_Z), 5.0),
            wave(parameter(standard::EYE_BALL_X), 0.6),
            wave(parameter(standard::EYE_BALL_Y), 0.3),
            wave(parameter(standard::BROW_L_Y), 0.5),
            wave(parameter(standard::BROW_R_Y), 0.5),
            wave(parameter(standard::MOUTH_FORM), 0.8),
            wave(parameter(standard::CHEEK), 0.5),
            arm,
            lip_sync,
        ],
        events: Vec::new(),
    }
}
