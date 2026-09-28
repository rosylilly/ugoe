//! M3 の受け入れテストの通しの手順 (T0310)。
//!
//! 髪の房 (回転デフォーマの鎖 `Hair1`〜`Hair3`) と体 (`Body`) を持つモデル (`fixtures::dynamic_hair`) に、
//! エディタと同じ `Command` だけで物理 (動的物理のリグ・肩の衝突体) とモーション・表情を作る。
//! 規則は `docs/editor.md#物理の編集` と `docs/editor.md#モーションと表情の編集`。
//! `ugoe-cli` の GPU テスト (`tests/m3_render_gpu.rs`) も `#[path]` でこのファイルを読み込むので、
//! 両方のクレートが依存するクレートだけを使う (`Vec2` は `ugoe_editor_core::space` から)。
#![allow(dead_code)]

use std::sync::Arc;

use ugoe_editor_core::space::Vec2;
use ugoe_editor_core::texture::solid_png;
use ugoe_editor_core::{Command, Document, SegmentShape, Textures};
use ugoe_model::expression::{Expression, ExpressionOp, ExpressionParameter};
use ugoe_model::motion::{CurveTarget, Motion, MotionEvent, Point};
use ugoe_model::physics::{ColliderShape, PhysicsCollider, PhysicsSettings};
use ugoe_model::validate::{validate, validate_physics};
use ugoe_model::{Model, fixtures, standard};

/// 髪のリグ。
pub const HAIR: &str = "Hair";
/// 肩の衝突体 (`Body` に取り付けたカプセル)。
pub const SHOULDER: &str = "Shoulder";
/// 頭を左右に振るモーション。
pub const SHAKE: &str = "Shake";
/// 頭を傾ける表情。
pub const TILT: &str = "Tilt";
pub const SHAKE_DURATION: f32 = 2.0;
/// `Shake` の `AngleX` のキー (時刻, 値)。0.5 → 1.5 秒の区間はベジェ。
pub const SHAKE_KEYS: [(f32, f32); 4] = [(0.0, 0.0), (0.5, 30.0), (1.5, -30.0), (2.0, 0.0)];

/// 肩のカプセル: `Body` の局所座標で `(-10,-60)`–`(40,-40)`、ワールドでは `(118,140)`–`(168,160)`。
pub const SHOULDER_A: Vec2 = Vec2::new(118.0, 140.0);
pub const SHOULDER_B: Vec2 = Vec2::new(168.0, 160.0);
pub const SHOULDER_RADIUS: f32 = 8.0;
/// 髪の粒子の衝突半径。
pub const COLLISION_RADIUS: f32 = 2.0;

/// 通しの手順を進めるドキュメントと、コマンドごとの状態の履歴 (Undo / Redo の検査用)。
#[derive(Debug)]
pub struct Workflow {
    pub doc: Document,
    /// `history[0]` は最初の状態、`history[i]` は `i` 個目のコマンドを適用した後の状態。
    pub history: Vec<Snapshot>,
}

/// Undo / Redo で比べる状態。
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub model: Model,
    pub physics: Option<PhysicsSettings>,
    pub motions: Vec<Motion>,
    pub expressions: Vec<Expression>,
}

impl Snapshot {
    pub fn of(doc: &Document) -> Self {
        Self {
            model: doc.model().clone(),
            physics: doc.physics().cloned(),
            motions: doc.motions().to_vec(),
            expressions: doc.expressions().to_vec(),
        }
    }
}

impl Workflow {
    fn new() -> Self {
        let textures: Textures = [("white".to_owned(), Arc::from(solid_png(1, 1, [255; 4])))]
            .into_iter()
            .collect();
        let doc = Document::with_textures(fixtures::dynamic_hair(), textures);
        let history = vec![Snapshot::of(&doc)];
        Self { doc, history }
    }

    /// コマンドを適用し、警告がなく、モデルと物理が検証を通ることを確かめて履歴に積む。
    #[track_caller]
    pub fn apply(&mut self, command: Command) {
        let label = format!("{command:?}");
        let report = self
            .doc
            .apply_with_report(command)
            .unwrap_or_else(|e| panic!("command failed: {e}\n{label}"));
        assert_eq!(report.warnings, vec![], "unexpected warnings: {label}");
        assert_eq!(validate(self.doc.model()), vec![], "invalid after {label}");
        if let Some(physics) = self.doc.physics() {
            assert_eq!(
                validate_physics(physics, self.doc.model()),
                vec![],
                "invalid physics after {label}"
            );
        }
        self.history.push(Snapshot::of(&self.doc));
    }

    pub fn physics(&self) -> &PhysicsSettings {
        self.doc.physics().expect("physics settings")
    }
}

/// 手順をすべて行う。
pub fn run() -> Workflow {
    let mut wf = Workflow::new();
    assert!(wf.doc.physics().is_none());
    physics(&mut wf);
    motion(&mut wf);
    expression(&mut wf);
    wf
}

/// 段階 1: 物理。世界の設定、髪の鎖からのリグ、肩の衝突体、衝突の設定。
fn physics(wf: &mut Workflow) {
    wf.apply(Command::SetPhysicsWorld {
        gravity: Vec2::new(0.0, 1.0),
        wind: Vec2::ZERO,
        fps: 60.0,
    });
    wf.apply(Command::CreatePhysicsRig {
        id: Some(HAIR.into()),
        name: "Hair".into(),
        deformers: vec!["Hair1".into(), "Hair2".into(), "Hair3".into()],
    });
    let rig = &wf.physics().rigs[0];
    assert_eq!(rig.id, HAIR);
    // 根元と各デフォーマの原点、先端 (Strand の下端) の 4 粒子。
    assert_eq!(rig.particles.len(), 4);
    assert_eq!(rig.outputs.len(), 3);

    wf.apply(Command::AddPhysicsCollider {
        collider: PhysicsCollider {
            id: SHOULDER.into(),
            name: "Shoulder".into(),
            attach: Some("Body".into()),
            shape: ColliderShape::Capsule {
                a: Vec2::new(-10.0, -60.0),
                b: Vec2::new(40.0, -40.0),
                radius: SHOULDER_RADIUS,
            },
        },
    });
    wf.apply(Command::SetPhysicsRigColliders {
        rig: HAIR.into(),
        colliders: vec![SHOULDER.into()],
    });
    let mut particles = wf.physics().rigs[0].particles.clone();
    for p in particles.iter_mut().skip(1) {
        p.collision_radius = COLLISION_RADIUS;
    }
    wf.apply(Command::SetPhysicsParticles {
        rig: HAIR.into(),
        particles,
    });
    let physics = wf.physics();
    assert_eq!(physics.colliders.len(), 1);
    assert_eq!(physics.rigs[0].colliders, vec![SHOULDER.to_owned()]);
}

pub fn angle_x() -> CurveTarget {
    CurveTarget::Parameter(standard::ANGLE_X.into())
}

/// 段階 2: モーション。空のモーションを作り、`AngleX` にキーを打ち、区間をベジェにし、イベントを置く。
fn motion(wf: &mut Workflow) {
    wf.apply(Command::AddMotion {
        motion: Motion {
            name: SHAKE.into(),
            duration: 1.0,
            looping: false,
            fade_in: 0.0,
            fade_out: 0.0,
            curves: Vec::new(),
            events: Vec::new(),
        },
    });
    wf.apply(Command::SetMotionProperties {
        name: SHAKE.into(),
        duration: Some(SHAKE_DURATION),
        looping: Some(true),
        fade_in: None,
        fade_out: None,
    });
    for (t, v) in SHAKE_KEYS {
        wf.apply(Command::SetMotionKey {
            motion: SHAKE.into(),
            target: angle_x(),
            point: Point { t, v },
        });
    }
    wf.apply(Command::SetMotionSegment {
        motion: SHAKE.into(),
        target: angle_x(),
        t: 1.5,
        segment: SegmentShape::Bezier { c1: None, c2: None },
    });
    wf.apply(Command::SetMotionEvents {
        motion: SHAKE.into(),
        events: vec![MotionEvent {
            t: 0.5,
            value: "right".into(),
        }],
    });
    let motion = &wf.doc.motions()[0];
    assert_eq!((motion.duration, motion.looping), (SHAKE_DURATION, true));
    assert_eq!(motion.curves.len(), 1);
    assert_eq!(motion.curves[0].segments.len(), SHAKE_KEYS.len());
}

/// 段階 3: 表情。頭を 10 度傾ける (`AngleZ` に加える)。
fn expression(wf: &mut Workflow) {
    wf.apply(Command::AddExpression {
        expression: Expression {
            name: TILT.into(),
            fade_in: 0.0,
            fade_out: 0.0,
            parameters: Vec::new(),
        },
    });
    wf.apply(Command::SetExpressionParameters {
        name: TILT.into(),
        parameters: vec![ExpressionParameter {
            parameter: standard::ANGLE_Z.into(),
            value: 10.0,
            op: ExpressionOp::Add,
        }],
    });
}

/// 肩のカプセルの芯 (線分) までの距離。半径 + 衝突半径 未満なら食い込んでいる。
pub fn shoulder_distance(p: Vec2) -> f32 {
    let (a, b) = (SHOULDER_A, SHOULDER_B);
    let t = ((p - a).dot(b - a) / (b - a).length_squared()).clamp(0.0, 1.0);
    (p - (a + (b - a) * t)).length()
}
