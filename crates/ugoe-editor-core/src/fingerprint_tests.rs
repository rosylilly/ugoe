//! プロジェクトのファイルの指紋 (外での書き換えの検出) のテスト。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use ugoe_core::Vec2;
use ugoe_model::fixtures;
use ugoe_model::motion::Motion;
use ugoe_model::physics::PhysicsSettings;

use crate::texture::solid_png;
use crate::{Command, Project, ProjectError, ProjectFingerprint, ProjectMeta, Textures};

/// テスト用のプロジェクトのディレクトリ。破棄時に消す。
struct TempProject(PathBuf);

impl TempProject {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "ugoe-editor-core-fp-{}-{name}.ugoeproj",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        sample_project().save(&path).unwrap();
        Self(path)
    }

    fn dir(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn sample_project() -> Project {
    let textures: Textures = [("white".to_owned(), Arc::from(solid_png(1, 1, [255; 4])))]
        .into_iter()
        .collect();
    let mut project = Project::new(
        ProjectMeta::new("Character"),
        fixtures::single_quad(),
        textures,
    );
    project.motions = vec![motion("idle")];
    project
}

fn motion(name: &str) -> Motion {
    serde_json::from_value(json!({ "name": name, "duration": 1.0, "looping": false, "curves": [] }))
        .unwrap()
}

/// 更新日時を `seconds` 秒ずらす (中身は変えない)。
fn shift_mtime(path: &Path, seconds: i64) {
    let file = fs::File::options().write(true).open(path).unwrap();
    let old = file.metadata().unwrap().modified().unwrap();
    let new = if seconds >= 0 {
        old + Duration::from_secs(seconds as u64)
    } else {
        old - Duration::from_secs(seconds.unsigned_abs())
    };
    file.set_modified(new).unwrap();
}

#[test]
fn saving_the_same_content_keeps_the_fingerprint() {
    let project = TempProject::new("same");
    let before = ProjectFingerprint::read(project.dir()).unwrap();
    sample_project().save(project.dir()).unwrap();
    let after = before.reread(project.dir()).unwrap();
    assert_eq!(before, after);
    assert!(before.changed_files(&after).is_empty());
    assert_eq!(before.digest(), after.digest());
}

/// 自分の保存の後に指紋を取り直せば、自分の保存は外の変更に見えない。
#[test]
fn own_save_is_not_an_external_change_after_rereading() {
    let project = TempProject::new("own");
    let (loaded, known) = Project::load_tracked(project.dir()).unwrap();
    let mut document = loaded.document();
    let mut parameter = document.model().parameters[0].clone();
    parameter.id = "Extra".into();
    document.apply(Command::AddParameter { parameter }).unwrap();
    Project::from_document(loaded.meta.clone(), &document)
        .save(project.dir())
        .unwrap();
    let after_save = known.reread(project.dir()).unwrap();
    assert_eq!(known.changed_files(&after_save), vec!["model.json"]);
    // 保存の直後に取り直した指紋を覚えておけば、次に比べても変わっていない。
    assert_eq!(after_save.reread(project.dir()).unwrap(), after_save);
}

#[test]
fn external_json_edit_is_detected() {
    let project = TempProject::new("json");
    let known = ProjectFingerprint::read(project.dir()).unwrap();
    let path = project.dir().join("model.json");
    let text = fs::read_to_string(&path).unwrap();
    fs::write(&path, text.replacen("single_quad", "renamed", 1)).unwrap();
    let now = known.reread(project.dir()).unwrap();
    assert_ne!(known, now);
    assert_eq!(known.changed_files(&now), vec!["model.json"]);
    assert_ne!(known.digest(), now.digest());
}

#[test]
fn added_and_removed_files_are_detected() {
    let project = TempProject::new("files");
    let known = ProjectFingerprint::read(project.dir()).unwrap();
    let mut changed = sample_project();
    changed.motions = vec![motion("wave")];
    changed.physics = Some(PhysicsSettings {
        gravity: Vec2::new(0.0, -1.0),
        wind: Vec2::new(0.0, 0.0),
        fps: 60.0,
        rigs: Vec::new(),
        colliders: Vec::new(),
    });
    changed.save(project.dir()).unwrap();
    let now = known.reread(project.dir()).unwrap();
    assert_eq!(
        known.changed_files(&now),
        vec![
            "motions/idle.motion.json",
            "motions/wave.motion.json",
            "physics.json"
        ]
    );
}

#[test]
fn texture_edit_is_detected() {
    let project = TempProject::new("png");
    let known = ProjectFingerprint::read(project.dir()).unwrap();
    let path = project.dir().join("textures/white.png");
    fs::write(&path, solid_png(2, 2, [0, 0, 0, 255])).unwrap();
    // 更新日時の刻みが粗くても変わったことが分かるように、日時も進める。
    shift_mtime(&path, 5);
    let now = known.reread(project.dir()).unwrap();
    assert_eq!(known.changed_files(&now), vec!["textures/white.png"]);
}

/// 中身が同じなら、更新日時が変わっても (`touch`・同じ内容の書き直し) 変更としない。
#[test]
fn touching_without_changing_content_is_not_a_change() {
    let project = TempProject::new("touch");
    let known = ProjectFingerprint::read(project.dir()).unwrap();
    shift_mtime(&project.dir().join("model.json"), 5);
    shift_mtime(&project.dir().join("textures/white.png"), 5);
    let now = known.reread(project.dir()).unwrap();
    assert_eq!(known, now);
    assert!(known.changed_files(&now).is_empty());
}

/// PNG は大きさと更新日時が同じなら読み直さない (大きな画像を毎回読まないため)。JSON は毎回読む。
#[test]
fn png_hash_is_reused_when_size_and_mtime_are_unchanged() {
    let project = TempProject::new("reuse");
    let known = ProjectFingerprint::read(project.dir()).unwrap();
    let png = project.dir().join("textures/white.png");
    let mtime = fs::metadata(&png).unwrap().modified().unwrap();
    let mut bytes = fs::read(&png).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0xff;
    fs::write(&png, &bytes).unwrap();
    fs::File::options()
        .write(true)
        .open(&png)
        .unwrap()
        .set_modified(mtime)
        .unwrap();
    assert_eq!(known.reread(project.dir()).unwrap(), known);
    // 使い回さずに読むと違いが分かる。
    let fresh = ProjectFingerprint::read(project.dir()).unwrap();
    assert_eq!(known.changed_files(&fresh), vec!["textures/white.png"]);

    // JSON は同じ大きさ・同じ日時でも読み直す。
    let meta = project.dir().join("project.json");
    let mtime = fs::metadata(&meta).unwrap().modified().unwrap();
    let text = fs::read_to_string(&meta).unwrap();
    fs::write(&meta, text.replacen("Character", "Charactor", 1)).unwrap();
    fs::File::options()
        .write(true)
        .open(&meta)
        .unwrap()
        .set_modified(mtime)
        .unwrap();
    let now = known.reread(project.dir()).unwrap();
    assert_eq!(known.changed_files(&now), vec!["project.json"]);
}

#[test]
fn removed_project_is_not_a_project() {
    let project = TempProject::new("removed");
    let known = ProjectFingerprint::read(project.dir()).unwrap();
    fs::remove_file(project.dir().join("project.json")).unwrap();
    assert!(matches!(
        known.reread(project.dir()),
        Err(ProjectError::NotAProject { .. })
    ));
    assert!(matches!(
        ProjectFingerprint::read(&project.dir().join("missing")),
        Err(ProjectError::NotAProject { .. })
    ));
}

/// 読み込みの指紋は読み込む前のもの: 読み込んだ中身と同じ時点のファイルを表す。
#[test]
fn load_tracked_fingerprint_matches_the_loaded_files() {
    let project = TempProject::new("tracked");
    let (loaded, fingerprint) = Project::load_tracked(project.dir()).unwrap();
    assert_eq!(loaded, Project::load(project.dir()).unwrap());
    assert_eq!(
        fingerprint,
        ProjectFingerprint::read(project.dir()).unwrap()
    );
}
