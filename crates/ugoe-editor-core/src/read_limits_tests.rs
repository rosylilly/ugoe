//! プロジェクト・本体・指紋が、通常のファイルだけを大きさの上限までで読むことのテスト (T1607)。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ugoe_model::fixtures;

use crate::message::Coded;
use crate::project::{load_base_checked, load_base_with_limits};
use crate::texture::solid_png;
use crate::{
    Project, ProjectError, ProjectFingerprint, ProjectMeta, ProjectReadLimits, Textures,
    export_bundle,
};

/// 確かめずにそのまま読む本体の確かめ。
fn unchecked(path: &Path) -> Result<PathBuf, String> {
    Ok(path.to_path_buf())
}

fn white() -> Arc<[u8]> {
    Arc::from(solid_png(1, 1, [255; 4]))
}

/// `single_quad` (テクスチャ `white`) のプロジェクトを `dir/character.ugoeproj` に保存する。
fn save_model_project(dir: &Path) -> PathBuf {
    let textures: Textures = [("white".to_owned(), white())].into_iter().collect();
    let path = dir.join("character.ugoeproj");
    Project::new(
        ProjectMeta::new("Character"),
        fixtures::single_quad(),
        textures,
    )
    .save(&path)
    .unwrap();
    path
}

/// 着せ替えの本体のプロジェクトと、その `Hair` のスロットのモジュールのプロジェクトを保存する (本体, モジュール)。
fn save_module_project(dir: &Path) -> (PathBuf, PathBuf) {
    let model = fixtures::dress_up_base();
    let textures: Textures = model
        .textures
        .iter()
        .map(|t| (t.name.clone(), Arc::from(solid_png(4, 4, [255; 4]))))
        .collect();
    let base = dir.join("avatar.ugoeproj");
    Project::new(ProjectMeta::new("avatar"), model, textures)
        .save(&base)
        .unwrap();
    let module = dir.join("hat.ugoeproj");
    Project::new_module("Hat", &base, &"Hair".into())
        .unwrap()
        .save(&module)
        .unwrap();
    (base, module)
}

#[cfg(unix)]
fn mkfifo(path: &Path) {
    let _ = fs::remove_file(path);
    let status = std::process::Command::new("mkfifo")
        .arg(path)
        .status()
        .unwrap();
    assert!(status.success());
}

/// `path` を `/dev/zero` (終わらない装置) へのリンクに替える。
#[cfg(unix)]
fn link_to_dev_zero(path: &Path) {
    fs::remove_file(path).unwrap();
    std::os::unix::fs::symlink("/dev/zero", path).unwrap();
}

fn assert_not_regular(result: Result<impl std::fmt::Debug, ProjectError>, expected: &Path) {
    match result {
        Err(ProjectError::NotRegularFile { path }) => assert_eq!(path, expected),
        other => panic!(
            "expected NotRegularFile for {}: {other:?}",
            expected.display()
        ),
    }
}

fn assert_too_large(result: Result<impl std::fmt::Debug, ProjectError>, expected: &Path) {
    match result {
        Err(ProjectError::FileTooLarge { path, len, max }) => {
            assert_eq!(path, expected);
            assert!(len > max, "{len} > {max}");
        }
        other => panic!(
            "expected FileTooLarge for {}: {other:?}",
            expected.display()
        ),
    }
}

#[test]
fn default_limits_read_ordinary_projects() {
    let temp = tempfile::tempdir().unwrap();
    let dir = save_model_project(temp.path());
    let limits = ProjectReadLimits::default();
    assert_eq!(limits.max_json, 256 * 1024 * 1024);
    assert_eq!(limits.max_image, 512 * 1024 * 1024);
    assert_eq!(limits.max_bundle, 512 * 1024 * 1024);
    let project = Project::load_with_limits(&dir, &unchecked, limits).unwrap();
    assert_eq!(project, Project::load(&dir).unwrap());
}

#[test]
fn files_over_the_limit_are_refused_without_reading() {
    let temp = tempfile::tempdir().unwrap();
    let dir = save_model_project(temp.path());
    let project_json = fs::metadata(dir.join("project.json")).unwrap().len();
    let model_json = fs::metadata(dir.join("model.json")).unwrap().len();
    let png = fs::metadata(dir.join("textures/white.png")).unwrap().len();

    // JSON の上限は project.json から確かめる。
    let limits = ProjectReadLimits::default().with_max_json(project_json - 1);
    assert_too_large(
        Project::load_with_limits(&dir, &unchecked, limits),
        &dir.join("project.json"),
    );
    let limits = ProjectReadLimits::default().with_max_json(project_json.max(model_json) - 1);
    let bigger = if model_json > project_json {
        "model.json"
    } else {
        "project.json"
    };
    assert_too_large(
        Project::load_with_limits(&dir, &unchecked, limits),
        &dir.join(bigger),
    );
    let limits = ProjectReadLimits::default().with_max_image(png - 1);
    assert_too_large(
        Project::load_with_limits(&dir, &unchecked, limits),
        &dir.join("textures/white.png"),
    );
    // ちょうど上限なら読める。
    let limits = ProjectReadLimits::default()
        .with_max_json(project_json.max(model_json))
        .with_max_image(png);
    assert!(Project::load_with_limits(&dir, &unchecked, limits).is_ok());

    // 指紋も同じ上限で取る。
    let limits = ProjectReadLimits::default().with_max_image(png - 1);
    assert_too_large(
        ProjectFingerprint::default().reread_with_limits(&dir, limits),
        &dir.join("textures/white.png"),
    );
    let limits = ProjectReadLimits::default().with_max_json(project_json - 1);
    assert_too_large(
        ProjectFingerprint::default().reread_with_limits(&dir, limits),
        &dir.join("project.json"),
    );
}

#[test]
fn base_bundle_over_the_limit_is_refused() {
    let temp = tempfile::tempdir().unwrap();
    let dir = save_model_project(temp.path());
    let bundle = temp.path().join("avatar.ugoe");
    fs::write(
        &bundle,
        export_bundle(&Project::load(&dir).unwrap()).unwrap(),
    )
    .unwrap();
    let len = fs::metadata(&bundle).unwrap().len();

    let limits = ProjectReadLimits::default().with_max_bundle(len - 1);
    assert_too_large(load_base_with_limits(&bundle, &unchecked, limits), &bundle);
    assert_too_large(
        ProjectFingerprint::default().reread_base_with_limits(&bundle, limits),
        &bundle,
    );
    let limits = ProjectReadLimits::default().with_max_bundle(len);
    assert!(load_base_with_limits(&bundle, &unchecked, limits).is_ok());
    assert!(
        ProjectFingerprint::default()
            .reread_base_with_limits(&bundle, limits)
            .is_ok()
    );

    // 確かめて替えたパスから読んでも、エラーに出すのは本体のパス。
    let moved = temp.path().join("moved.ugoe");
    fs::rename(&bundle, &moved).unwrap();
    let to_moved = |_: &Path| Ok(moved.clone());
    assert_too_large(
        load_base_with_limits(
            &bundle,
            &to_moved,
            ProjectReadLimits::default().with_max_bundle(len - 1),
        ),
        &bundle,
    );
}

#[test]
fn directories_in_place_of_files_are_refused() {
    let temp = tempfile::tempdir().unwrap();
    let dir = save_model_project(temp.path());
    let physics = dir.join("physics.json");
    fs::create_dir(&physics).unwrap();
    assert_not_regular(Project::load(&dir), &physics);
    assert_not_regular(ProjectFingerprint::read(&dir), &physics);
}

#[test]
fn refusals_have_codes_and_arguments() {
    let message = ProjectError::NotRegularFile {
        path: "/p/model.json".into(),
    }
    .to_message();
    assert_eq!(message.code, "project.not_regular_file");
    assert_eq!(
        serde_json::to_value(&message.args).unwrap(),
        serde_json::json!({ "path": "/p/model.json" })
    );
    let message = ProjectError::FileTooLarge {
        path: "/p/model.json".into(),
        len: 300,
        max: 256,
    }
    .to_message();
    assert_eq!(message.code, "project.file_too_large");
    assert_eq!(
        serde_json::to_value(&message.args).unwrap(),
        serde_json::json!({ "path": "/p/model.json", "size": 300.0, "max": 256.0 })
    );
}

/// FIFO (開くだけで書き手を待って止まる) と装置 (終わらない) は、開かずに拒む。止まればテストが終わらない。
#[cfg(unix)]
#[test]
fn fifos_and_devices_in_a_project_are_refused_without_opening() {
    let temp = tempfile::tempdir().unwrap();
    let dir = save_model_project(temp.path());

    let model = dir.join("model.json");
    let saved_model = fs::read(&model).unwrap();
    mkfifo(&model);
    assert_not_regular(Project::load(&dir), &model);
    assert_not_regular(ProjectFingerprint::read(&dir), &model);
    assert_not_regular(Project::load_tracked(&dir), &model);
    fs::remove_file(&model).unwrap();
    fs::write(&model, &saved_model).unwrap();

    let project_json = dir.join("project.json");
    let saved_meta = fs::read(&project_json).unwrap();
    mkfifo(&project_json);
    assert_not_regular(Project::load(&dir), &project_json);
    assert_not_regular(ProjectFingerprint::read(&dir), &project_json);
    fs::remove_file(&project_json).unwrap();
    fs::write(&project_json, &saved_meta).unwrap();

    let png = dir.join("textures/white.png");
    link_to_dev_zero(&png);
    assert_not_regular(Project::load(&dir), &png);
    // 指紋は、ディレクトリの中の通常のファイルでないものを (読み込みと同じく) 数えない。
    assert!(ProjectFingerprint::read(&dir).is_ok());
}

/// 保存で今のファイルと比べるときも、FIFO を読まずに置き換える。
#[cfg(unix)]
#[test]
fn saving_replaces_a_fifo_without_reading_it() {
    let temp = tempfile::tempdir().unwrap();
    let dir = save_model_project(temp.path());
    let model = dir.join("model.json");
    let saved_model = fs::read(&model).unwrap();
    mkfifo(&model);
    let project = Project::new(
        ProjectMeta::new("Character"),
        fixtures::single_quad(),
        [("white".to_owned(), white())].into_iter().collect(),
    );
    project.save(&dir).unwrap();
    assert!(fs::metadata(&model).unwrap().is_file());
    assert_eq!(fs::read(&model).unwrap(), saved_model);
}

#[cfg(unix)]
#[test]
fn fifos_in_a_module_project_and_its_base_are_refused() {
    let temp = tempfile::tempdir().unwrap();
    let (base, module) = save_module_project(temp.path());
    assert!(Project::load(&module).is_ok());

    let module_json = module.join("module.json");
    let saved = fs::read(&module_json).unwrap();
    mkfifo(&module_json);
    assert_not_regular(Project::load(&module), &module_json);
    assert_not_regular(ProjectFingerprint::read(&module), &module_json);
    fs::remove_file(&module_json).unwrap();
    fs::write(&module_json, &saved).unwrap();

    // 本体のプロジェクトの中のファイル。
    let base_model = base.join("model.json");
    mkfifo(&base_model);
    match Project::load(&module) {
        Err(ProjectError::Base { source, .. }) => {
            assert!(
                matches!(*source, ProjectError::NotRegularFile { ref path } if *path == base_model),
                "{source:?}"
            );
        }
        other => panic!("expected Base: {other:?}"),
    }

    // 本体の `.ugoe` が FIFO・装置。
    let bundle = temp.path().join("avatar.ugoe");
    mkfifo(&bundle);
    assert_not_regular(load_base_checked(&bundle, &unchecked), &bundle);
    assert_not_regular(
        Project::load_with_base(&module, &bundle).map(|_| ()),
        &bundle,
    );
    assert_not_regular(ProjectFingerprint::read_base(&bundle), &bundle);
    let zero = temp.path().join("zero.ugoe");
    std::os::unix::fs::symlink("/dev/zero", &zero).unwrap();
    assert_not_regular(load_base_checked(&zero, &unchecked), &zero);
}
