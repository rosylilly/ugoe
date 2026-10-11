//! モーションと表情の編集コマンドのテスト (`motion_edit.rs`)。入力は `single_quad` (パラメータ `X`、パーツ `Root`)。

use ugoe_model::expression::{Expression, ExpressionOp, ExpressionParameter};
use ugoe_model::fixtures;
use ugoe_model::motion::{Curve, CurveTarget, Motion, MotionEvent, Point, Segment};
use ugoe_motion::curve::evaluate;

use std::sync::Arc;

use crate::texture::solid_png;
use crate::{
    Command, Document, EditError, Project, ProjectMeta, SegmentShape, Textures, export_bundle,
};

fn p(t: f32, v: f32) -> Point {
    Point { t, v }
}

fn x() -> CurveTarget {
    CurveTarget::Parameter("X".into())
}

fn motion(name: &str) -> Motion {
    Motion {
        name: name.into(),
        duration: 2.0,
        looping: false,
        fade_in: 0.0,
        fade_out: 0.0,
        curves: Vec::new(),
        events: Vec::new(),
    }
}

/// `Idle` (長さ 2 秒、カーブなし) を持つドキュメント。
fn doc() -> Document {
    let mut doc = Document::new(fixtures::single_quad());
    doc.apply(Command::AddMotion {
        motion: motion("Idle"),
    })
    .unwrap();
    doc
}

fn key(doc: &mut Document, t: f32, v: f32) -> Result<(), EditError> {
    doc.apply(Command::SetMotionKey {
        motion: "Idle".into(),
        target: x(),
        point: p(t, v),
    })
}

fn segments(doc: &Document) -> Vec<Segment> {
    doc.motions()[0].curves[0].segments.clone()
}

fn key_times(doc: &Document) -> Vec<f32> {
    segments(doc)
        .iter()
        .map(|s| match s {
            Segment::Start { p } => p.t,
            Segment::Linear { to }
            | Segment::Bezier { to, .. }
            | Segment::Stepped { to }
            | Segment::InverseStepped { to } => to.t,
            _ => f32::NAN,
        })
        .collect()
}

fn assert_close(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-4, "{a} != {b}");
}

// ---- モーション ----

#[test]
fn motions_are_undoable_and_mark_the_document_modified() {
    let mut doc = Document::new(fixtures::single_quad());
    assert!(!doc.is_modified());
    doc.apply(Command::AddMotion {
        motion: motion("Idle"),
    })
    .unwrap();
    assert!(doc.is_modified());
    assert_eq!(doc.motions().len(), 1);
    assert!(doc.undo());
    assert!(doc.motions().is_empty());
    assert!(!doc.is_modified());
    assert!(doc.redo());
    assert_eq!(doc.motions()[0].name, "Idle");
}

#[test]
fn motion_names_must_be_file_names_and_unique() {
    let mut doc = doc();
    for name in ["", ".hidden", "a/b", "笑顔"] {
        let err = doc
            .apply(Command::AddMotion {
                motion: motion(name),
            })
            .unwrap_err();
        assert!(
            matches!(err, EditError::InvalidName { .. }),
            "{name}: {err}"
        );
    }
    let err = doc
        .apply(Command::AddMotion {
            motion: motion("idle"),
        })
        .unwrap_err();
    assert!(matches!(err, EditError::DuplicateId { kind: "motion", .. }));

    doc.apply(Command::AddMotion {
        motion: motion("Walk"),
    })
    .unwrap();
    let rename = |name: &str, new_name: &str| Command::RenameMotion {
        name: name.into(),
        new_name: new_name.into(),
    };
    assert!(doc.apply(rename("Walk", "IDLE")).is_err());
    // 自分自身の大文字小文字だけを変える名前は使える。
    doc.apply(rename("Walk", "walk")).unwrap();
    doc.apply(rename("walk", "Run")).unwrap();
    assert_eq!(doc.motions()[1].name, "Run");
    assert!(matches!(
        doc.apply(rename("Missing", "X")),
        Err(EditError::NotFound { kind: "motion", .. })
    ));
    doc.apply(Command::RemoveMotion { name: "Run".into() })
        .unwrap();
    assert_eq!(doc.motions().len(), 1);
}

#[test]
fn motion_properties_are_validated() {
    let mut doc = doc();
    let set = |duration, fade_in| Command::SetMotionProperties {
        name: "Idle".into(),
        duration,
        looping: Some(true),
        fade_in,
        fade_out: None,
    };
    for (duration, fade_in) in [
        (Some(0.0), None),
        (Some(-1.0), None),
        (Some(f32::NAN), None),
        (None, Some(-0.5)),
    ] {
        let err = doc.apply(set(duration, fade_in)).unwrap_err();
        assert!(matches!(err, EditError::InvalidMotion { .. }), "{err}");
        // 拒否した編集は何も変えない (ループも)。
        assert!(!doc.motions()[0].looping);
    }
    doc.apply(set(Some(3.0), Some(0.25))).unwrap();
    let m = &doc.motions()[0];
    assert_eq!((m.duration, m.looping, m.fade_in), (3.0, true, 0.25));
}

#[test]
fn added_motions_must_refer_to_existing_targets_and_be_well_formed() {
    let mut doc = doc();
    let mut m = motion("Walk");
    m.curves.push(Curve {
        target: CurveTarget::Parameter("Missing".into()),
        segments: vec![Segment::Start { p: p(0.0, 0.0) }],
        fade_in: None,
        fade_out: None,
    });
    assert!(matches!(
        doc.apply(Command::AddMotion { motion: m.clone() }),
        Err(EditError::NotFound {
            kind: "parameter",
            ..
        })
    ));
    m.curves[0].target = x();
    m.curves[0].segments = vec![
        Segment::Start { p: p(1.0, 0.0) },
        Segment::Linear { to: p(0.5, 1.0) },
    ];
    assert!(matches!(
        doc.apply(Command::AddMotion { motion: m }),
        Err(EditError::InvalidMotion { .. })
    ));
    assert_eq!(doc.motions().len(), 1);
}

#[test]
fn events_are_sorted_and_validated() {
    let mut doc = doc();
    let event = |t: f32, value: &str| MotionEvent {
        t,
        value: value.into(),
    };
    doc.apply(Command::SetMotionEvents {
        motion: "Idle".into(),
        events: vec![event(1.5, "b"), event(0.5, "a")],
    })
    .unwrap();
    let values: Vec<_> = doc.motions()[0]
        .events
        .iter()
        .map(|e| e.value.as_str())
        .collect();
    assert_eq!(values, ["a", "b"]);
    assert!(
        doc.apply(Command::SetMotionEvents {
            motion: "Idle".into(),
            events: vec![event(-1.0, "c")],
        })
        .is_err()
    );
}

// ---- カーブ ----

#[test]
fn curves_are_added_removed_and_faded() {
    let mut doc = doc();
    let curve = Curve {
        target: CurveTarget::PartOpacity("Root".into()),
        segments: vec![Segment::Start { p: p(0.0, 1.0) }],
        fade_in: None,
        fade_out: None,
    };
    let add = |curve: Curve| Command::AddMotionCurve {
        motion: "Idle".into(),
        curve,
    };
    doc.apply(add(curve.clone())).unwrap();
    assert!(matches!(
        doc.apply(add(curve.clone())),
        Err(EditError::DuplicateId { kind: "curve", .. })
    ));
    let mut group = curve.clone();
    group.target = CurveTarget::EffectGroup("EyeBlink".into());
    doc.apply(add(group.clone())).unwrap();
    group.target = CurveTarget::EffectGroup("Sneeze".into());
    assert!(doc.apply(add(group)).is_err());

    doc.apply(Command::SetMotionCurveFade {
        motion: "Idle".into(),
        target: curve.target.clone(),
        fade_in: Some(0.2),
        fade_out: None,
    })
    .unwrap();
    assert_eq!(doc.motions()[0].curves[0].fade_in, Some(0.2));
    assert!(
        doc.apply(Command::SetMotionCurveFade {
            motion: "Idle".into(),
            target: curve.target.clone(),
            fade_in: None,
            fade_out: Some(-1.0),
        })
        .is_err()
    );

    doc.apply(Command::RemoveMotionCurve {
        motion: "Idle".into(),
        target: curve.target,
    })
    .unwrap();
    assert_eq!(doc.motions()[0].curves.len(), 1);
    assert!(
        doc.apply(Command::RemoveMotionCurve {
            motion: "Idle".into(),
            target: x(),
        })
        .is_err()
    );
}

// ---- キー ----

#[test]
fn setting_a_key_creates_the_curve_and_inserts_in_time_order() {
    let mut doc = doc();
    assert!(matches!(
        doc.apply(Command::SetMotionKey {
            motion: "Idle".into(),
            target: CurveTarget::Parameter("Missing".into()),
            point: p(0.0, 0.0),
        }),
        Err(EditError::NotFound { .. })
    ));
    key(&mut doc, 1.0, 0.5).unwrap();
    assert_eq!(segments(&doc), vec![Segment::Start { p: p(1.0, 0.5) }]);
    key(&mut doc, 2.0, 1.0).unwrap(); // 後ろ: 直線
    key(&mut doc, 0.0, 0.0).unwrap(); // 前: 最初の区間と同じ種類
    key(&mut doc, 1.5, 0.2).unwrap(); // 途中: 区間を分ける
    assert_eq!(
        segments(&doc),
        vec![
            Segment::Start { p: p(0.0, 0.0) },
            Segment::Linear { to: p(1.0, 0.5) },
            Segment::Linear { to: p(1.5, 0.2) },
            Segment::Linear { to: p(2.0, 1.0) },
        ]
    );
    // 同じ時刻なら値を変える。
    key(&mut doc, 1.5, 0.7).unwrap();
    assert_eq!(key_times(&doc), [0.0, 1.0, 1.5, 2.0]);
    assert_eq!(segments(&doc)[2], Segment::Linear { to: p(1.5, 0.7) });
    assert!(key(&mut doc, f32::NAN, 0.0).is_err());
    // 1 回のキーが Undo の 1 単位。
    assert!(doc.undo());
    assert_eq!(segments(&doc)[2], Segment::Linear { to: p(1.5, 0.2) });
}

#[test]
fn new_keys_follow_the_neighboring_segment_kind() {
    let mut doc = doc();
    key(&mut doc, 0.0, 0.0).unwrap();
    key(&mut doc, 1.0, 1.0).unwrap();
    doc.apply(Command::SetMotionSegment {
        motion: "Idle".into(),
        target: x(),
        t: 1.0,
        segment: SegmentShape::Stepped,
    })
    .unwrap();
    key(&mut doc, 2.0, 0.0).unwrap();
    key(&mut doc, 0.5, 0.3).unwrap();
    let segs = segments(&doc);
    assert_eq!(segs[1], Segment::Stepped { to: p(0.5, 0.3) });
    assert_eq!(segs[2], Segment::Stepped { to: p(1.0, 1.0) });
    assert_eq!(segs[3], Segment::Stepped { to: p(2.0, 0.0) });
}

#[test]
fn keying_on_a_bezier_curve_keeps_its_shape() {
    let mut doc = doc();
    key(&mut doc, 0.0, 0.0).unwrap();
    key(&mut doc, 2.0, 1.0).unwrap();
    doc.apply(Command::SetMotionSegment {
        motion: "Idle".into(),
        target: x(),
        t: 2.0,
        segment: SegmentShape::Bezier {
            c1: Some(p(0.9, 0.0)),
            c2: Some(p(1.2, 1.0)),
        },
    })
    .unwrap();
    let before = segments(&doc);
    let at = evaluate(&before, 0.7).unwrap();
    key(&mut doc, 0.7, at).unwrap();
    let after = segments(&doc);
    assert!(matches!(after[1], Segment::Bezier { .. }));
    assert!(matches!(after[2], Segment::Bezier { .. }));
    for i in 0..=40 {
        let t = i as f32 * 0.05;
        let (a, b) = (evaluate(&before, t).unwrap(), evaluate(&after, t).unwrap());
        assert!((a - b).abs() < 2e-3, "t = {t}: {a} vs {b}");
    }
}

#[test]
fn moving_a_key_keeps_order_and_carries_bezier_handles() {
    let mut doc = doc();
    for (t, v) in [(0.0, 0.0), (1.0, 1.0), (2.0, 0.0)] {
        key(&mut doc, t, v).unwrap();
    }
    for t in [1.0, 2.0] {
        doc.apply(Command::SetMotionSegment {
            motion: "Idle".into(),
            target: x(),
            t,
            segment: SegmentShape::Bezier { c1: None, c2: None },
        })
        .unwrap();
    }
    let mv = |t: f32, to: Point| Command::MoveMotionKey {
        motion: "Idle".into(),
        target: x(),
        t,
        to,
    };
    for to in [p(0.0, 0.0), p(2.0, 0.0), p(2.5, 0.0)] {
        assert!(matches!(
            doc.apply(mv(1.0, to)),
            Err(EditError::InvalidMotion { .. })
        ));
    }
    assert!(matches!(
        doc.apply(mv(0.4, p(0.5, 0.0))),
        Err(EditError::NotFound { kind: "key", .. })
    ));
    doc.apply(mv(1.0, p(1.25, 1.5))).unwrap();
    let segs = segments(&doc);
    // 入る区間の終点側の取っ手と、出る区間の始点側の取っ手が同じだけ動く。
    let Segment::Bezier { c1, c2, to } = segs[1] else {
        panic!("{segs:?}");
    };
    assert_eq!(to, p(1.25, 1.5));
    assert_close(c1.t, 1.0 / 3.0);
    assert_close(c2.t, 2.0 / 3.0 + 0.25);
    assert_close(c2.v, 1.5);
    let Segment::Bezier { c1, .. } = segs[2] else {
        panic!("{segs:?}");
    };
    assert_close(c1.t, 1.0 + 1.0 / 3.0 + 0.25);
    assert_close(c1.v, 1.5);
    // 取っ手は区間の中に収まる。
    doc.apply(mv(1.25, p(0.2, 1.0))).unwrap();
    let Segment::Bezier { c1, c2, .. } = segments(&doc)[1] else {
        panic!();
    };
    assert!(c1.t <= 0.2 && c2.t <= 0.2, "{c1:?} {c2:?}");
}

#[test]
fn removing_keys_joins_segments_and_drops_empty_curves() {
    let mut doc = doc();
    for (t, v) in [(0.0, 0.0), (1.0, 1.0), (2.0, 0.0)] {
        key(&mut doc, t, v).unwrap();
    }
    doc.apply(Command::SetMotionSegment {
        motion: "Idle".into(),
        target: x(),
        t: 1.0,
        segment: SegmentShape::Bezier {
            c1: Some(p(0.5, 0.0)),
            c2: None,
        },
    })
    .unwrap();
    let rm = |t: f32| Command::RemoveMotionKey {
        motion: "Idle".into(),
        target: x(),
        t,
    };
    doc.apply(rm(1.0)).unwrap();
    // 消したキーに入る区間 (ベジェ) の種類でつなぎ、始点側の取っ手を保つ。
    let segs = segments(&doc);
    assert_eq!(segs.len(), 2);
    let Segment::Bezier { c1, c2, to } = segs[1] else {
        panic!("{segs:?}");
    };
    assert_eq!((c1, to), (p(0.5, 0.0), p(2.0, 0.0)));
    assert!(c2.t >= 0.0 && c2.t <= 2.0);
    doc.apply(rm(0.0)).unwrap();
    assert_eq!(segments(&doc), vec![Segment::Start { p: p(2.0, 0.0) }]);
    doc.apply(rm(2.0)).unwrap();
    assert!(doc.motions()[0].curves.is_empty());
    // 3 回の削除を戻す。
    for _ in 0..3 {
        assert!(doc.undo());
    }
    assert_eq!(key_times(&doc), [0.0, 1.0, 2.0]);
}

#[test]
fn segments_can_be_changed_and_handles_are_checked() {
    let mut doc = doc();
    key(&mut doc, 0.0, 0.0).unwrap();
    key(&mut doc, 3.0, 3.0).unwrap();
    let set = |t: f32, segment| Command::SetMotionSegment {
        motion: "Idle".into(),
        target: x(),
        t,
        segment,
    };
    assert!(doc.apply(set(0.0, SegmentShape::Linear)).is_err());
    doc.apply(set(3.0, SegmentShape::Bezier { c1: None, c2: None }))
        .unwrap();
    // 既定の取っ手は 1/3 と 2/3 の時刻で、両端の値。
    assert_eq!(
        segments(&doc)[1],
        Segment::Bezier {
            c1: p(1.0, 0.0),
            c2: p(2.0, 3.0),
            to: p(3.0, 3.0),
        }
    );
    // 片方だけ指定すると、もう片方は今の取っ手を保つ。
    doc.apply(set(
        3.0,
        SegmentShape::Bezier {
            c1: Some(p(0.5, 2.0)),
            c2: None,
        },
    ))
    .unwrap();
    assert_eq!(
        segments(&doc)[1],
        Segment::Bezier {
            c1: p(0.5, 2.0),
            c2: p(2.0, 3.0),
            to: p(3.0, 3.0),
        }
    );
    let err = doc
        .apply(set(
            3.0,
            SegmentShape::Bezier {
                c1: Some(p(3.5, 0.0)),
                c2: None,
            },
        ))
        .unwrap_err();
    assert!(err.to_string().contains("outside its segment"), "{err}");
    doc.apply(set(3.0, SegmentShape::InverseStepped)).unwrap();
    assert_eq!(
        segments(&doc)[1],
        Segment::InverseStepped { to: p(3.0, 3.0) }
    );
}

#[test]
fn edits_of_invalid_loaded_motions_do_not_add_problems() {
    // 長さ 0 のモーション (読み込んだファイルなど) も、問題を増やさない編集はできる。
    let mut m = motion("Broken");
    m.duration = 0.0;
    let project = {
        let mut project = Project::new(
            ProjectMeta::new("p"),
            fixtures::single_quad(),
            Default::default(),
        );
        project.motions.push(m);
        project
    };
    let mut doc = project.document();
    doc.apply(Command::SetMotionProperties {
        name: "Broken".into(),
        duration: None,
        looping: Some(true),
        fade_in: None,
        fade_out: None,
    })
    .unwrap();
    assert!(
        doc.apply(Command::SetMotionProperties {
            name: "Broken".into(),
            duration: None,
            looping: None,
            fade_in: Some(-1.0),
            fade_out: None,
        })
        .is_err()
    );
}

#[test]
fn keying_a_loaded_curve_without_segments_starts_it() {
    // 点のないカーブ (読み込んだファイルなど) にキーを打つと、そのキーが始点になる (止まらない)。
    let mut m = motion("Idle");
    m.curves.push(Curve {
        target: x(),
        segments: Vec::new(),
        fade_in: None,
        fade_out: None,
    });
    let mut project = Project::new(
        ProjectMeta::new("p"),
        fixtures::single_quad(),
        Default::default(),
    );
    project.motions.push(m);
    let mut doc = project.document();
    key(&mut doc, 0.5, 1.0).unwrap();
    assert_eq!(segments(&doc), vec![Segment::Start { p: p(0.5, 1.0) }]);
}

#[test]
fn a_batch_of_keys_is_one_undo_step() {
    let mut doc = doc();
    doc.apply(Command::Batch {
        label: "key all".into(),
        commands: vec![
            Command::SetMotionKey {
                motion: "Idle".into(),
                target: x(),
                point: p(0.0, 0.0),
            },
            Command::SetMotionKey {
                motion: "Idle".into(),
                target: x(),
                point: p(1.0, 1.0),
            },
        ],
    })
    .unwrap();
    assert_eq!(key_times(&doc), [0.0, 1.0]);
    assert!(doc.undo());
    assert!(doc.motions()[0].curves.is_empty());
}

// ---- 表情 ----

fn expression(name: &str) -> Expression {
    Expression {
        name: name.into(),
        fade_in: 0.5,
        fade_out: 0.5,
        parameters: vec![ExpressionParameter {
            parameter: "X".into(),
            value: 0.5,
            op: ExpressionOp::Add,
        }],
    }
}

#[test]
fn expressions_are_added_edited_and_removed() {
    let mut doc = Document::new(fixtures::single_quad());
    doc.apply(Command::AddExpression {
        expression: expression("Smile"),
    })
    .unwrap();
    assert!(
        doc.apply(Command::AddExpression {
            expression: expression("SMILE"),
        })
        .is_err()
    );
    let mut missing = expression("Angry");
    missing.parameters[0].parameter = "Missing".into();
    assert!(matches!(
        doc.apply(Command::AddExpression {
            expression: missing
        }),
        Err(EditError::NotFound { .. })
    ));

    let params = |op, value| {
        vec![ExpressionParameter {
            parameter: "X".into(),
            value,
            op,
        }]
    };
    doc.apply(Command::SetExpressionParameters {
        name: "Smile".into(),
        parameters: params(ExpressionOp::Multiply, 2.0),
    })
    .unwrap();
    assert_eq!(
        doc.expressions()[0].parameters[0].op,
        ExpressionOp::Multiply
    );
    let mut twice = params(ExpressionOp::Add, 1.0);
    twice.extend(params(ExpressionOp::Overwrite, 1.0));
    assert!(matches!(
        doc.apply(Command::SetExpressionParameters {
            name: "Smile".into(),
            parameters: twice,
        }),
        Err(EditError::InvalidExpression { .. })
    ));
    assert!(
        doc.apply(Command::SetExpressionParameters {
            name: "Smile".into(),
            parameters: params(ExpressionOp::Add, f32::INFINITY),
        })
        .is_err()
    );

    doc.apply(Command::SetExpressionProperties {
        name: "Smile".into(),
        fade_in: Some(0.1),
        fade_out: None,
    })
    .unwrap();
    assert_eq!(
        (doc.expressions()[0].fade_in, doc.expressions()[0].fade_out),
        (0.1, 0.5)
    );
    assert!(
        doc.apply(Command::SetExpressionProperties {
            name: "Smile".into(),
            fade_in: None,
            fade_out: Some(-0.1),
        })
        .is_err()
    );
    doc.apply(Command::RenameExpression {
        name: "Smile".into(),
        new_name: "Happy".into(),
    })
    .unwrap();
    doc.apply(Command::RemoveExpression {
        name: "Happy".into(),
    })
    .unwrap();
    assert!(doc.expressions().is_empty());
    assert!(doc.undo());
    assert_eq!(doc.expressions()[0].name, "Happy");
}

// ---- 保存と書き出し ----

#[test]
fn motions_and_expressions_are_saved_and_exported_from_the_document() {
    let textures: Textures = [("white".to_owned(), Arc::from(solid_png(1, 1, [255; 4])))]
        .into_iter()
        .collect();
    let mut doc = Document::with_textures(fixtures::single_quad(), textures);
    doc.apply(Command::AddMotion {
        motion: motion("Idle"),
    })
    .unwrap();
    key(&mut doc, 0.0, 0.0).unwrap();
    key(&mut doc, 1.0, 1.0).unwrap();
    doc.apply(Command::AddExpression {
        expression: expression("Smile"),
    })
    .unwrap();
    let project = Project::from_document(ProjectMeta::new("p"), &doc);
    assert_eq!(project.motions, doc.motions());
    assert_eq!(project.expressions, doc.expressions());

    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("p.ugoeproj");
    project.save(&dir).unwrap();
    let loaded = Project::load(&dir).unwrap();
    let reopened = loaded.document();
    assert!(!reopened.is_modified());
    assert_eq!(reopened.motions(), doc.motions());
    assert_eq!(reopened.expressions(), doc.expressions());

    let bundle = ugoe_format::read_bundle(&export_bundle(&project).unwrap()).unwrap();
    assert_eq!(bundle.motions, doc.motions());
    assert_eq!(bundle.expressions, doc.expressions());
}
