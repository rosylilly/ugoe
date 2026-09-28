//! プロジェクトの保存・読み込み・書き出しと、テクスチャのコマンドのテスト。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ugoe_core::Vec2;
use ugoe_model::expression::Expression;
use ugoe_model::motion::Motion;
use ugoe_model::physics::PhysicsSettings;
use ugoe_model::{Model, fixtures};

use crate::texture::solid_png;
use crate::{Command, Document, EditError, Project, ProjectError, ProjectMeta, Textures};

/// テスト用の一時ディレクトリ。破棄時に消す。
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("ugoe-editor-core-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn project(&self) -> PathBuf {
        self.0.join("character.ugoeproj")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn motion(name: &str) -> Motion {
    serde_json::from_value(serde_json::json!({
        "name": name,
        "duration": 1.5,
        "looping": true,
        "curves": [],
    }))
    .unwrap()
}

fn expression(name: &str) -> Expression {
    serde_json::from_value(serde_json::json!({
        "name": name,
        "parameters": [{ "parameter": "X", "value": 0.25, "op": "add" }],
    }))
    .unwrap()
}

/// `single_quad` にテクスチャ 2 枚・物理・モーション・表情を加えたプロジェクト。
fn sample_project() -> Project {
    let mut model = fixtures::single_quad();
    let mut extra = model.textures[0].clone();
    extra.name = "Extra-2".into();
    (extra.width, extra.height) = (2, 3);
    model.textures.push(extra);
    let textures: Textures = [
        ("white".to_owned(), Arc::from(solid_png(1, 1, [255; 4]))),
        (
            "Extra-2".to_owned(),
            Arc::from(solid_png(2, 3, [255, 0, 0, 128])),
        ),
    ]
    .into_iter()
    .collect();
    let mut project = Project::new(ProjectMeta::new("Character"), model, textures);
    project.physics = Some(PhysicsSettings {
        gravity: Vec2::new(0.0, -1.0),
        wind: Vec2::new(0.1, 0.0),
        fps: 60.0,
        rigs: Vec::new(),
        colliders: Vec::new(),
    });
    // 読み込みはファイル名の順なので、名前の順に並べておく。
    project.motions = vec![motion("idle"), motion("tap_body")];
    project.expressions = vec![expression("smile")];
    project
}

/// ディレクトリの中のファイル (相対パス → 中身) を名前の順に集める。
fn snapshot(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in fs::read_dir(&d).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let bytes = fs::read(&path).unwrap();
                out.push((path.strip_prefix(dir).unwrap().to_path_buf(), bytes));
            }
        }
    }
    out.sort();
    out
}

fn file_names(dir: &Path) -> Vec<String> {
    snapshot(dir)
        .into_iter()
        .map(|(p, _)| p.to_string_lossy().replace('\\', "/"))
        .collect()
}

// ---- 保存と読み込み ----

#[test]
fn save_and_load_round_trip() {
    let tmp = TempDir::new("round-trip");
    let project = sample_project();
    project.save(&tmp.project()).unwrap();

    assert_eq!(
        file_names(&tmp.project()),
        [
            ".gitignore",
            "expressions/smile.expression.json",
            "model.json",
            "motions/idle.motion.json",
            "motions/tap_body.motion.json",
            "physics.json",
            "project.json",
            "textures/Extra-2.png",
            "textures/white.png",
        ]
    );
    let loaded = Project::load(&tmp.project()).unwrap();
    assert_eq!(loaded, project);
}

#[test]
fn saved_json_is_pretty_and_ends_with_newline() {
    let tmp = TempDir::new("pretty");
    sample_project().save(&tmp.project()).unwrap();
    let meta = fs::read_to_string(tmp.project().join("project.json")).unwrap();
    assert_eq!(
        meta,
        "{\n  \"format_version\": 0,\n  \"name\": \"Character\"\n}\n"
    );
    let model = fs::read_to_string(tmp.project().join("model.json")).unwrap();
    assert!(model.starts_with("{\n  \"schema_version\": "), "{model}");
    assert!(model.ends_with("}\n"));
}

#[test]
fn saving_twice_is_byte_identical() {
    let tmp = TempDir::new("stable");
    let project = sample_project();
    project.save(&tmp.project()).unwrap();
    let first = snapshot(&tmp.project());

    // 読み込み直したものを別の場所に保存しても同じバイト列になる。
    project.save(&tmp.project()).unwrap();
    assert_eq!(snapshot(&tmp.project()), first);
    let other = tmp.path().join("other.ugoeproj");
    Project::load(&tmp.project()).unwrap().save(&other).unwrap();
    assert_eq!(snapshot(&other), first);
}

#[test]
fn save_removes_stale_files_but_keeps_sources() {
    let tmp = TempDir::new("stale");
    let dir = tmp.project();
    sample_project().save(&dir).unwrap();
    fs::create_dir_all(dir.join("sources")).unwrap();
    fs::write(dir.join("sources/character.psd"), b"psd").unwrap();
    fs::write(dir.join("motions/notes.txt"), b"keep").unwrap();

    let mut project = sample_project();
    project.physics = None;
    project.motions.remove(1);
    project.expressions.clear();
    project.model.textures.remove(1);
    project.textures.remove("Extra-2");
    project.save(&dir).unwrap();

    assert_eq!(
        file_names(&dir),
        [
            ".gitignore",
            "model.json",
            "motions/idle.motion.json",
            "motions/notes.txt",
            "project.json",
            "sources/character.psd",
            "textures/white.png",
        ]
    );
    assert_eq!(Project::load(&dir).unwrap(), project);
}

#[test]
fn save_refuses_non_project_directory() {
    let tmp = TempDir::new("not-empty");
    fs::create_dir_all(tmp.path()).unwrap();
    fs::write(tmp.path().join("important.txt"), b"data").unwrap();
    let err = sample_project().save(tmp.path()).unwrap_err();
    assert!(matches!(err, ProjectError::NotEmpty { .. }), "{err}");
    assert_eq!(file_names(tmp.path()), ["important.txt"]);
}

#[test]
fn save_rejects_bad_names_without_writing() {
    let tmp = TempDir::new("bad-names");
    let mut project = sample_project();
    project.motions.push(motion("../escape"));
    let err = project.save(&tmp.project()).unwrap_err();
    assert!(
        matches!(err, ProjectError::InvalidName { kind: "motion", .. }),
        "{err}"
    );
    assert!(!tmp.project().exists());

    let mut project = sample_project();
    project.expressions.push(expression("SMILE"));
    let err = project.save(&tmp.project()).unwrap_err();
    assert!(
        matches!(
            err,
            ProjectError::DuplicateName {
                kind: "expression",
                ..
            }
        ),
        "{err}"
    );

    let mut project = sample_project();
    project.textures.remove("white");
    let err = project.save(&tmp.project()).unwrap_err();
    assert!(matches!(err, ProjectError::MissingTexture { .. }), "{err}");
    assert!(!tmp.project().exists());
}

#[test]
fn empty_project_round_trips() {
    let tmp = TempDir::new("empty");
    let project = Project::empty("New");
    assert!(ugoe_model::validate::validate(&project.model).is_empty());
    project.save(&tmp.project()).unwrap();
    assert_eq!(Project::load(&tmp.project()).unwrap(), project);
}

// ---- 読み込みのエラー ----

#[test]
fn load_missing_project_fails() {
    let tmp = TempDir::new("missing");
    let err = Project::load(&tmp.project()).unwrap_err();
    assert!(matches!(err, ProjectError::NotAProject { .. }), "{err}");
}

#[test]
fn load_corrupt_project_fails() {
    let tmp = TempDir::new("corrupt");
    let dir = tmp.project();
    let save = || sample_project().save(&dir).unwrap();

    save();
    fs::write(dir.join("model.json"), b"{ not json").unwrap();
    let err = Project::load(&dir).unwrap_err();
    assert!(matches!(err, ProjectError::Format { .. }), "{err}");

    save();
    fs::write(dir.join("project.json"), b"[]").unwrap();
    let err = Project::load(&dir).unwrap_err();
    assert!(matches!(err, ProjectError::Json { .. }), "{err}");

    save();
    fs::write(
        dir.join("project.json"),
        b"{\"format_version\": 99, \"name\": \"x\"}",
    )
    .unwrap();
    let err = Project::load(&dir).unwrap_err();
    assert!(
        matches!(err, ProjectError::UnsupportedVersion { found: 99, .. }),
        "{err}"
    );

    save();
    fs::write(dir.join("motions/idle.motion.json"), b"{}").unwrap();
    let err = Project::load(&dir).unwrap_err();
    assert!(matches!(err, ProjectError::Json { .. }), "{err}");

    save();
    fs::remove_file(dir.join("textures/white.png")).unwrap();
    let err = Project::load(&dir).unwrap_err();
    assert!(matches!(err, ProjectError::MissingTexture { .. }), "{err}");

    save();
    fs::write(dir.join("textures/white.png"), b"GIF89a").unwrap();
    let err = Project::load(&dir).unwrap_err();
    assert!(matches!(err, ProjectError::NotPng { .. }), "{err}");

    save();
    let mut model: Model = fixtures::single_quad();
    model.textures[0].name = "../../etc/passwd".into();
    fs::write(
        dir.join("model.json"),
        ugoe_format::model_to_json(&model).unwrap(),
    )
    .unwrap();
    let err = Project::load(&dir).unwrap_err();
    assert!(
        matches!(
            err,
            ProjectError::InvalidName {
                kind: "texture",
                ..
            }
        ),
        "{err}"
    );
}

// ---- 書き出し ----

#[test]
fn export_bundle_round_trips_through_read_bundle() {
    let project = sample_project();
    let bytes = crate::export_bundle(&project).unwrap();
    let bundle = ugoe_format::read_bundle(&bytes).unwrap();
    assert_eq!(bundle.model, project.model);
    assert_eq!(bundle.physics, project.physics);
    assert_eq!(bundle.motions, project.motions);
    assert_eq!(bundle.expressions, project.expressions);
    // テクスチャは `Model::textures` の順。
    let textures: Vec<&[u8]> = bundle.textures.iter().map(|t| t.bytes.as_slice()).collect();
    assert_eq!(
        textures,
        [&*project.textures["white"], &*project.textures["Extra-2"]]
    );
}

#[test]
fn export_bundle_requires_texture_data() {
    let mut project = sample_project();
    project.textures.remove("Extra-2");
    let err = crate::export_bundle(&project).unwrap_err();
    assert!(matches!(err, ProjectError::MissingTexture { .. }), "{err}");
}

// ---- ドキュメントとの受け渡し・変更の有無 ----

#[test]
fn document_round_trip_and_modified_flag() {
    let project = sample_project();
    let mut doc = project.document();
    assert!(!doc.is_modified());
    assert_eq!(doc.texture("white"), Some(&*project.textures["white"]));

    doc.apply(Command::RenameParameter {
        id: "X".into(),
        name: "Move".into(),
    })
    .unwrap();
    assert!(doc.is_modified());
    assert!(doc.undo());
    assert!(!doc.is_modified(), "undo back to the saved state is clean");
    assert!(doc.redo());
    assert!(doc.is_modified());
    doc.mark_saved();
    assert!(!doc.is_modified());
    assert!(doc.undo());
    assert!(doc.is_modified());

    // 失敗した適用は状態を変えない。
    doc.mark_saved();
    assert!(
        doc.apply(Command::RemovePart { id: "Root".into() })
            .is_err()
    );
    assert!(!doc.is_modified());

    let back = Project::from_document(project.meta.clone(), &doc);
    assert_eq!(back.textures, project.textures);
    assert_eq!(back.model, project.model);
    // 物理はドキュメントが持ち、そのまま戻る。
    assert_eq!(back.physics, project.physics);
}

// ---- テクスチャのコマンド ----

#[test]
fn add_texture() {
    let mut doc = Document::new(fixtures::single_quad());
    let png = solid_png(4, 2, [0, 0, 0, 255]);
    doc.apply(Command::AddTexture {
        name: "hair".into(),
        png: png.clone(),
    })
    .unwrap();
    let added = doc.model().textures.last().unwrap();
    assert_eq!(
        (added.name.as_str(), added.width, added.height),
        ("hair", 4, 2)
    );
    assert_eq!(doc.texture("hair"), Some(png.as_slice()));

    assert!(doc.undo());
    assert_eq!(doc.model().textures.len(), 1);
    assert_eq!(doc.texture("hair"), None);
    assert!(doc.redo());
    assert_eq!(doc.texture("hair"), Some(png.as_slice()));

    let before = doc.model().clone();
    let reject = |doc: &mut Document, name: &str, png: Vec<u8>| {
        doc.apply(Command::AddTexture {
            name: name.into(),
            png,
        })
        .unwrap_err()
    };
    assert_eq!(
        reject(&mut doc, "HAIR", png.clone()),
        EditError::DuplicateId {
            kind: "texture",
            id: "HAIR".into()
        }
    );
    assert_eq!(
        reject(&mut doc, "a/b", png.clone()),
        EditError::InvalidTextureName { name: "a/b".into() }
    );
    assert_eq!(
        reject(&mut doc, "gif", b"GIF89a".to_vec()),
        EditError::NotPng { name: "gif".into() }
    );
    assert_eq!(doc.model(), &before);
}

#[test]
fn set_texture_image_updates_size() {
    let mut doc = Document::new(fixtures::single_quad());
    doc.apply(Command::SetTextureImage {
        name: "white".into(),
        png: solid_png(8, 16, [255; 4]),
    })
    .unwrap();
    let texture = &doc.model().textures[0];
    assert_eq!((texture.width, texture.height), (8, 16));
    assert!(doc.texture("white").is_some());
    assert!(matches!(
        doc.apply(Command::SetTextureImage {
            name: "missing".into(),
            png: solid_png(1, 1, [0; 4]),
        }),
        Err(EditError::NotFound {
            kind: "texture",
            ..
        })
    ));
}

#[test]
fn remove_texture_rejects_used_and_shifts_indices() {
    let mut doc = sample_project().document();
    let err = doc
        .apply(Command::RemoveTexture {
            name: "white".into(),
        })
        .unwrap_err();
    assert_eq!(
        err,
        EditError::HasChildren {
            kind: "texture",
            id: "white".into(),
            children: vec!["mesh `Quad`".into()],
        }
    );

    // メッシュを後ろのテクスチャに付け替えてから前のテクスチャを消すと、インデックスが詰まる。
    doc.apply(Command::SetMeshProperties {
        id: "Quad".into(),
        part: None,
        texture: Some(1),
        blend: None,
        culling: None,
        masks: None,
        mask_mode: None,
        mask_only: None,
    })
    .unwrap();
    doc.apply(Command::RemoveTexture {
        name: "white".into(),
    })
    .unwrap();
    assert_eq!(doc.model().meshes[0].texture, 0);
    assert_eq!(doc.model().textures[0].name, "Extra-2");
    assert_eq!(doc.texture("white"), None);
    assert!(ugoe_model::validate::validate(doc.model()).is_empty());
}
