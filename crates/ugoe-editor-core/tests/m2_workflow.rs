//! M2 の受け入れテスト (T0616): PSD の取り込みから `.ugoe` の書き出しまでを、
//! エディタと同じ `Command` だけで通して行う。手順は `m2/mod.rs`。

mod m2;

use std::path::PathBuf;

use m2::{ANGLE_X, ANGLE_Y, FACE, FACE_TURN, evaluate};
use ugoe_editor_core::{Project, ProjectMeta, export_bundle};
use ugoe_model::validate::validate;

/// 段階 1〜5: 取り込み → デフォーマ → パラメータとキー → 加算合成 → ミラー → 深度からの回転。
/// 各段階の検査は `m2::run` の中にある。
#[test]
fn workflow_from_psd_to_depth_rotation() {
    let wf = m2::run();
    let model = wf.model();
    assert_eq!(validate(model), vec![]);
    assert_eq!(model.deformers.len(), 5);
    assert_eq!(
        m2::deformer(model, FACE)
            .parent
            .as_ref()
            .map(|p| p.as_str()),
        Some(FACE_TURN)
    );
}

/// 段階 6: どのコマンドも 1 回の Undo で直前の状態に戻り、Redo で同じ状態をやり直す。
#[test]
fn every_step_undoes_and_redoes_exactly() {
    let mut wf = m2::run();
    let history = std::mem::take(&mut wf.history);
    let last = history.len() - 1;
    for i in (0..last).rev() {
        assert!(wf.doc.undo(), "undo to state {i}");
        assert!(
            wf.doc.model() == &history[i].0,
            "model after undoing to state {i}"
        );
        assert!(
            wf.doc.textures() == &history[i].1,
            "textures after undoing to state {i}"
        );
    }
    assert!(!wf.doc.undo(), "nothing left to undo");
    assert!(!wf.doc.is_modified(), "back at the empty project");
    for (i, (model, textures)) in history.iter().enumerate().skip(1) {
        assert!(wf.doc.redo(), "redo to state {i}");
        assert!(wf.doc.model() == model, "model after redoing to state {i}");
        assert!(
            wf.doc.textures() == textures,
            "textures after redoing to state {i}"
        );
    }
    assert!(!wf.doc.redo(), "nothing left to redo");
}

/// 段階 7: `.ugoeproj` に保存して読み戻し、`.ugoe` に書き出してランタイムで評価する。
#[test]
fn saves_loads_exports_and_evaluates_bundle() {
    let mut wf = m2::run();
    let dir = TempDir::new("project");
    let project_dir = dir.path.join("m2.ugoeproj");
    let project = Project::from_document(
        ProjectMeta::new("m2"),
        &wf.doc,
        None,
        Vec::new(),
        Vec::new(),
    );
    project.save(&project_dir).expect("save project");
    wf.doc.mark_saved();
    assert!(!wf.doc.is_modified());

    let loaded = Project::load(&project_dir).expect("load project");
    assert!(
        loaded == project,
        "the loaded project must equal the saved one"
    );
    let reopened = loaded.document();
    assert!(reopened.model() == wf.doc.model());
    assert!(reopened.textures() == wf.doc.textures());

    let bytes = export_bundle(&loaded).expect("export bundle");
    let bundle_path = dir.path.join("m2.ugoe");
    std::fs::write(&bundle_path, &bytes).expect("write bundle");
    let bundle =
        ugoe_format::read_bundle(&std::fs::read(&bundle_path).unwrap()).expect("read bundle");
    assert_eq!(bundle.textures.len(), bundle.model.textures.len());
    assert!(
        bundle.model == *wf.doc.model(),
        "the bundle keeps the model"
    );

    let rest = evaluate(&bundle.model, &[]);
    m2::assert_same_look(&wf.rest_look, &rest, "bundle at rest");
    // 中間 (キーの間) と斜めの値。
    let poses: [&[(&str, f32)]; 4] = [
        &[(ANGLE_X, 15.0), (ANGLE_Y, 0.0)],
        &[(ANGLE_X, 30.0), (ANGLE_Y, 30.0)],
        &[(ANGLE_X, -22.5), (ANGLE_Y, 7.5)],
        &[(ANGLE_X, -30.0), (ANGLE_Y, -30.0)],
    ];
    for values in poses {
        let from_bundle = evaluate(&bundle.model, values);
        let from_document = evaluate(wf.doc.model(), values);
        assert_eq!(from_bundle, from_document, "{values:?}");
        assert!(
            from_bundle.iter().flatten().all(|p| p.is_finite()),
            "{values:?}"
        );
        // 顔 (`肌` = `Layer5`) は基準の姿から動く。背景 (`Layer1`) は動かない。
        let face = mesh_index(&bundle.model, "Layer5");
        let moved = from_bundle[face]
            .iter()
            .zip(&rest[face])
            .map(|(a, b)| (*a - *b).length())
            .fold(0.0, f32::max);
        assert!(moved > 0.5, "{values:?}: the face moved only {moved}px");
        let background = mesh_index(&bundle.model, "Layer1");
        assert_eq!(from_bundle[background], rest[background], "{values:?}");
    }
}

fn mesh_index(model: &ugoe_model::Model, id: &str) -> usize {
    model
        .meshes
        .iter()
        .position(|m| m.id.as_str() == id)
        .unwrap_or_else(|| panic!("mesh {id}"))
}

/// テストごとの一時ディレクトリ (リポジトリの外)。終わると消す。
struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("ugoe-m2-workflow-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create temp dir");
        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
