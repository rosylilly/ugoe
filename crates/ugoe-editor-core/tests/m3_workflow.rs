//! M3 の受け入れテスト (T0310): エディタと同じ `Command` だけで物理の設定とモーション・表情を作り、
//! 保存・書き出しして、ランタイム SDK (`ugoe::Puppet`) で再生する。手順は `m3/mod.rs`。
//! `Puppet` だけで確かめる受け入れテストは `crates/ugoe/tests/m3_puppet.rs`。

#![allow(clippy::unwrap_used)]

mod m3;

use m3::{COLLISION_RADIUS, SHAKE, SHAKE_DURATION, SHOULDER_RADIUS, TILT};
use ugoe::motion::Priority;
use ugoe::motion::curve::evaluate;
use ugoe::{Puppet, Toggles};
use ugoe_editor_core::{Project, ProjectMeta, export_bundle};
use ugoe_model::standard;

const DT: f32 = 1.0 / 60.0;

fn param(puppet: &Puppet, id: &str) -> f32 {
    let p = puppet.instance().parameters();
    p.get(p.index_of(id).unwrap())
}

fn deformer(puppet: &Puppet, id: &str) -> usize {
    puppet.instance().compiled().deformer_index(id).unwrap()
}

/// 段階 1〜3: 物理・モーション・表情。各段階の検査は `m3::run` の中にある。
#[test]
fn workflow_creates_physics_motion_and_expression() {
    let wf = m3::run();
    let physics = wf.physics();
    assert_eq!(physics.rigs.len(), 1);
    assert!(
        physics.rigs[0].particles[1..]
            .iter()
            .all(|p| p.collision_radius == COLLISION_RADIUS)
    );
    assert_eq!(wf.doc.motions().len(), 1);
    assert_eq!(wf.doc.expressions().len(), 1);
    // モデル自体は変わらない (物理とアニメーションは別のデータ)。
    assert!(wf.doc.model() == &wf.history[0].model);
}

/// どのコマンドも 1 回の Undo で直前の状態に戻り、Redo で同じ状態をやり直す。
#[test]
fn every_step_undoes_and_redoes_exactly() {
    let mut wf = m3::run();
    let history = std::mem::take(&mut wf.history);
    for i in (0..history.len() - 1).rev() {
        assert!(wf.doc.undo(), "undo to state {i}");
        assert_eq!(
            m3::Snapshot::of(&wf.doc),
            history[i],
            "after undoing to {i}"
        );
    }
    assert!(!wf.doc.undo(), "nothing left to undo");
    assert!(!wf.doc.is_modified());
    for (i, state) in history.iter().enumerate().skip(1) {
        assert!(wf.doc.redo(), "redo to state {i}");
        assert_eq!(&m3::Snapshot::of(&wf.doc), state, "after redoing to {i}");
    }
    assert!(!wf.doc.redo(), "nothing left to redo");
}

/// `.ugoeproj` に保存して読み戻し、`.ugoe` に書き出して `Puppet` で再生する。
#[test]
fn saved_and_exported_bundle_plays_in_puppet() {
    let mut wf = m3::run();
    let dir = tempfile::tempdir().expect("create temp dir");
    let project_dir = dir.path().join("m3.ugoeproj");
    let project = Project::from_document(ProjectMeta::new("m3"), &wf.doc);
    project.save(&project_dir).expect("save project");
    wf.doc.mark_saved();
    let loaded = Project::load(&project_dir).expect("load project");
    assert!(
        loaded == project,
        "the loaded project must equal the saved one"
    );
    assert_eq!(
        m3::Snapshot::of(&loaded.document()),
        m3::Snapshot::of(&wf.doc)
    );

    let bundle_path = dir.path().join("m3.ugoe");
    std::fs::write(&bundle_path, export_bundle(&loaded).expect("export")).expect("write");
    let bundle = ugoe::format::read_bundle(&std::fs::read(&bundle_path).unwrap()).expect("read");
    assert_eq!(bundle.physics.as_ref(), wf.doc.physics());
    assert_eq!(bundle.motions, wf.doc.motions());
    assert_eq!(bundle.expressions, wf.doc.expressions());

    let shake = bundle
        .motions
        .iter()
        .find(|m| m.name == SHAKE)
        .unwrap()
        .clone();
    let tilt = bundle
        .expressions
        .iter()
        .find(|e| e.name == TILT)
        .unwrap()
        .clone();
    let mut puppet = Puppet::new(bundle.model)
        .unwrap()
        .with_physics(bundle.physics.unwrap());
    assert!(puppet.physics().unwrap().is_dynamic());
    assert!(puppet.start_motion(shake.clone(), Priority::Normal));
    puppet.set_expression(tilt);

    let hair1 = deformer(&puppet, "Hair1");
    let reach = SHOULDER_RADIUS + COLLISION_RADIUS;
    let mut swing = 0.0f32;
    let mut events = 0;
    for frame in 1..=240 {
        puppet.update(DT);
        let t = (frame as f32 * DT) % SHAKE_DURATION;
        // `AngleX` はエディタのタイムラインと同じカーブの値 (`ugoe_motion::curve::evaluate`)。
        let expected = evaluate(&shake.curves[0].segments, t).unwrap();
        let angle_x = param(&puppet, standard::ANGLE_X);
        assert!(
            (angle_x - expected).abs() < 0.05,
            "AngleX at frame {frame}: {angle_x} != {expected}"
        );
        // 表情が `AngleZ` に重なる。
        assert!((param(&puppet, standard::ANGLE_Z) - 10.0).abs() < 1e-5);
        swing = swing.max(
            puppet
                .instance()
                .dynamic_offsets()
                .unwrap()
                .angle(hair1)
                .abs(),
        );
        // 髪の粒子は肩の衝突体に入らない。
        let chain = puppet.physics().unwrap().chain(0).unwrap();
        for (i, p) in chain.particles().iter().enumerate().skip(1) {
            let d = m3::shoulder_distance(p.position);
            assert!(d >= reach - 1e-3, "frame {frame} particle {i}: {d}");
        }
        events += puppet
            .motion_events()
            .iter()
            .filter(|e| e.value == "right")
            .count();
    }
    // 揺れ用のパラメータなしで、頭の動きに遅れて髪が揺れる。
    assert!(swing > 5.0, "the hair swung only {swing} degrees");
    // 0.5 秒のイベントは 2 周で 2 回発火する。
    assert_eq!(events, 2);

    // 物理を止めると、髪は頭と一緒に動くだけになる。
    puppet.set_toggles(Toggles {
        physics: false,
        ..Toggles::default()
    });
    puppet.update(DT);
    assert_eq!(
        puppet.instance().dynamic_offsets().unwrap().angle(hair1),
        0.0
    );
}
