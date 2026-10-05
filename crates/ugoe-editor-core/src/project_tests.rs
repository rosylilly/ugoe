//! プロジェクトの保存・読み込み・書き出しと、テクスチャのコマンドのテスト。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ugoe_core::Vec2;
use ugoe_model::expression::Expression;
use ugoe_model::motion::Motion;
use ugoe_model::physics::{PhysicsSettings, PhysicsTarget, VertexOwner};
use ugoe_model::{Model, fixtures};

use crate::message::Coded;
use crate::texture::{DEFAULT_MAX_IMAGE_SIDE, huge_png_header, png_header, solid_png};
use crate::{Command, Document, EditError, Project, ProjectError, ProjectMeta, Textures};

/// テスト用の一時ディレクトリ。`path()` はまだ作っていないパスで、破棄時に親ごと消す。
struct TempDir {
    _root: tempfile::TempDir,
    path: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join(name);
        Self { _root: root, path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn project(&self) -> PathBuf {
        self.path.join("character.ugoeproj")
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

/// 保存は一時ファイル `.<名前>.<プロセス ID>.<連番>.tmp` を経て書き (`ugoe_fs::write_atomic`)、一時ファイルを残さない。
/// 落ちて残った一時ファイルは、読み込み・指紋・古いファイルの削除のどれにも拾われない (T0025)。
#[test]
fn leftover_temporary_files_are_ignored() {
    let tmp = TempDir::new("leftover");
    let dir = tmp.project();
    let project = sample_project();
    project.save(&dir).unwrap();
    let saved = file_names(&dir);
    assert!(
        saved.iter().all(|name| !name.ends_with(".tmp")),
        "{saved:?}"
    );

    let (_, fingerprint) = Project::load_tracked(&dir).unwrap();
    let leftovers = [
        "motions/.idle.motion.json.1.0.tmp",
        "expressions/.smile.expression.json.1.1.tmp",
        "textures/.white.png.1.2.tmp",
    ];
    for name in leftovers {
        fs::write(dir.join(name), b"half").unwrap();
    }
    let (loaded, after) = Project::load_tracked(&dir).unwrap();
    assert_eq!(loaded, project);
    assert_eq!(after, fingerprint);
    project.save(&dir).unwrap();
    let mut expected = saved;
    expected.extend(leftovers.map(String::from));
    expected.sort();
    assert_eq!(file_names(&dir), expected);
}

/// 大文字小文字だけを変えた名前で保存し直しても、ファイルが 1 つだけ新しい名前で残る。大文字小文字を区別しない
/// ファイルシステム (Windows・macOS) では、古い名前のファイルを消すと新しく書いたファイルも消えるので、書き直す。
#[test]
fn renaming_only_the_case_keeps_the_file() {
    let tmp = TempDir::new("case-rename");
    let dir = tmp.project();
    sample_project().save(&dir).unwrap();

    let mut project = sample_project();
    project.motions[0].name = "Idle".into();
    project.save(&dir).unwrap();
    let motions: Vec<String> = file_names(&dir)
        .into_iter()
        .filter(|name| name.starts_with("motions/"))
        .collect();
    assert_eq!(
        motions,
        ["motions/Idle.motion.json", "motions/tap_body.motion.json"]
    );
    let loaded = Project::load(&dir).unwrap();
    let names: Vec<&str> = loaded.motions.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["Idle", "tap_body"]);
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

/// 新しい版のファイルから読んだ未知の値 (`Unknown`) は書けないので、`model.json` と同じく
/// [`ugoe_format::FormatError::UnknownValues`] で、どのファイルかを示して拒否する (何も書かない)。
#[test]
fn save_rejects_unknown_values_with_the_file_name() {
    let tmp = TempDir::new("unknown-values");
    let mut project = sample_project();
    project.motions[0] = serde_json::from_value(serde_json::json!({
        "name": "idle",
        "duration": 1.0,
        "curves": [{ "target": { "kind": "future_target", "id": "X" }, "segments": [] }],
    }))
    .unwrap();
    let err = project.save(&tmp.project()).unwrap_err();
    let ProjectError::Format { path, source } = &err else {
        panic!("expected a format error: {err}");
    };
    assert_eq!(path, Path::new("motions/idle.motion.json"));
    assert!(
        matches!(source, ugoe_format::FormatError::UnknownValues(v) if v == &["curves[0].target"]),
        "{err}"
    );
    assert!(!tmp.project().exists());
}

/// 頂点への出力の未知の対象 (`VertexOwner::Unknown`) も同じく、`physics.json` の場所を示して拒否する。
#[test]
fn save_rejects_unknown_vertex_owners() {
    let tmp = TempDir::new("unknown-owner");
    let mut project = sample_project();
    let mut physics = fixtures::dynamic_hair_settings();
    if let PhysicsTarget::Vertices { owner, .. } = &mut physics.rigs[1].outputs[0].target {
        *owner = VertexOwner::Unknown;
    }
    project.physics = Some(physics);
    let err = project.save(&tmp.project()).unwrap_err();
    let ProjectError::Format { path, source } = &err else {
        panic!("expected a format error: {err}");
    };
    assert_eq!(path, Path::new("physics.json"));
    assert!(
        matches!(source, ugoe_format::FormatError::UnknownValues(v)
            if v == &["rigs[Skirt].outputs[0].target.owner"]),
        "{err}"
    );
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

/// 「外部の AI に送らない」は `editor.ai.send_externally` に保存され、既定 (送ってよい) なら書かない。
#[test]
fn ai_send_externally_round_trips_and_defaults_to_allowed() {
    let tmp = TempDir::new("ai-send");
    let mut project = Project::empty("New");
    assert!(project.meta.ai_send_externally());
    project
        .meta
        .editor
        .insert("other".into(), serde_json::json!({ "keep": 1 }));
    project.meta.set_ai_send_externally(false);
    assert!(!project.meta.ai_send_externally());
    project.save(&tmp.project()).unwrap();
    let json = fs::read_to_string(tmp.project().join("project.json")).unwrap();
    assert!(json.contains("\"send_externally\": false"), "{json}");
    let mut loaded = Project::load(&tmp.project()).unwrap();
    assert!(!loaded.meta.ai_send_externally());

    // 送ってよいに戻すと項目が消え、ほかの設定は残る。
    loaded.meta.set_ai_send_externally(true);
    assert!(loaded.meta.ai_send_externally());
    assert!(!loaded.meta.editor.contains_key("ai"));
    assert_eq!(
        loaded.meta.editor["other"],
        serde_json::json!({ "keep": 1 })
    );

    // 項目のない (以前の) project.json は、送ってよい扱いで読める。
    let old: ProjectMeta =
        serde_json::from_str(r#"{ "format_version": 0, "name": "old" }"#).unwrap();
    assert!(old.ai_send_externally());
    // 形の違う値は既定 (送ってよい) として読み、上書きできる。
    let mut odd: ProjectMeta = serde_json::from_str(
        r#"{ "format_version": 0, "name": "odd", "editor": { "ai": "yes" } }"#,
    )
    .unwrap();
    assert!(odd.ai_send_externally());
    odd.set_ai_send_externally(false);
    assert!(!odd.ai_send_externally());
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

// T1606: モーション・表情の数値は、バンドルの読み込み (`ugoe-format`) と同じく検証して拒む。
// JSON に NaN は書けないので、`f32` で無限大になる `1e39` と負の長さで確かめる。
#[test]
fn load_rejects_motions_and_expressions_with_invalid_numbers() {
    let tmp = TempDir::new("invalid-motion");
    let dir = tmp.project();
    let save = || sample_project().save(&dir).unwrap();

    save();
    fs::write(
        dir.join("motions/idle.motion.json"),
        br#"{ "name": "idle", "duration": 1.0, "curves": [ {
            "target": { "kind": "parameter", "id": "X" },
            "segments": [ { "type": "start", "p": { "t": 0, "v": 1e39 } } ] } ] }"#,
    )
    .unwrap();
    let err = Project::load(&dir).unwrap_err();
    let ProjectError::Format { path, source } = &err else {
        panic!("{err}");
    };
    assert_eq!(path, &dir.join("motions/idle.motion.json"));
    assert!(
        matches!(source, ugoe_format::FormatError::InvalidMotion { name, errors } if name == "idle" && errors.len() == 1),
        "{err}"
    );
    // 文はファイルの場所とモーションの名前、検証エラーの場所を含む。
    let text = err.to_string();
    assert!(text.contains("idle.motion.json"), "{text}");
    assert!(
        text.contains("motion `idle` failed validation with 1 error(s)"),
        "{text}"
    );

    save();
    fs::write(
        dir.join("motions/tap_body.motion.json"),
        br#"{ "name": "tap_body", "duration": -1.0, "curves": [] }"#,
    )
    .unwrap();
    let err = Project::load(&dir).unwrap_err();
    assert!(
        matches!(&err, ProjectError::Format { source: ugoe_format::FormatError::InvalidMotion { name, .. }, .. } if name == "tap_body"),
        "{err}"
    );

    save();
    fs::write(
        dir.join("expressions/smile.expression.json"),
        br#"{ "name": "smile", "parameters": [ { "parameter": "X", "value": -1e39, "op": "add" } ] }"#,
    )
    .unwrap();
    let err = Project::load(&dir).unwrap_err();
    let ProjectError::Format { path, source } = &err else {
        panic!("{err}");
    };
    assert_eq!(path, &dir.join("expressions/smile.expression.json"));
    assert!(
        matches!(source, ugoe_format::FormatError::InvalidExpression { name, errors } if name == "smile" && errors.len() == 1),
        "{err}"
    );

    // 正しい値に戻せば読める。
    save();
    assert!(Project::load(&dir).is_ok());
}

// ---- 法線マップ ----

/// `sample_project` の `white` に法線マップ `white_normal` を付けたプロジェクト。
fn project_with_normal_map() -> Project {
    let mut project = sample_project();
    project.model.textures[0].normal_map = Some("white_normal".into());
    project.textures.insert(
        "white_normal".into(),
        Arc::from(solid_png(1, 1, [128, 128, 255, 255])),
    );
    project
}

#[test]
fn normal_map_round_trips_and_survives_resave() {
    let tmp = TempDir::new("normal-map");
    let dir = tmp.project();
    let project = project_with_normal_map();
    project.save(&dir).unwrap();
    assert!(dir.join("textures/white_normal.png").is_file());
    let loaded = Project::load(&dir).unwrap();
    assert_eq!(loaded, project);

    // 保存し直しても、使われていない画像として消されない。
    loaded.save(&dir).unwrap();
    assert!(dir.join("textures/white_normal.png").is_file());

    // 法線マップを外すと画像も消える。
    let mut project = project;
    project.model.textures[0].normal_map = None;
    project.textures.remove("white_normal");
    project.save(&dir).unwrap();
    assert!(!dir.join("textures/white_normal.png").exists());
    assert_eq!(Project::load(&dir).unwrap(), project);
}

#[test]
fn shared_normal_map_is_saved_once() {
    let tmp = TempDir::new("normal-map-shared");
    let mut project = project_with_normal_map();
    project.model.textures[1].normal_map = Some("white_normal".into());
    project.save(&tmp.project()).unwrap();
    let pngs: Vec<String> = file_names(&tmp.project())
        .into_iter()
        .filter(|n| n.starts_with("textures/"))
        .collect();
    assert_eq!(
        pngs,
        [
            "textures/Extra-2.png",
            "textures/white.png",
            "textures/white_normal.png"
        ]
    );
    assert_eq!(Project::load(&tmp.project()).unwrap(), project);
}

#[test]
fn normal_map_names_are_checked_before_writing() {
    let tmp = TempDir::new("normal-map-names");

    let mut project = project_with_normal_map();
    project.textures.remove("white_normal");
    let err = project.save(&tmp.project()).unwrap_err();
    assert!(
        matches!(&err, ProjectError::MissingTexture { name } if name == "white_normal"),
        "{err}"
    );
    assert!(!tmp.project().exists());

    // テクスチャの名前と大文字小文字だけが違う法線マップはファイル名が衝突する。
    let mut project = project_with_normal_map();
    project.model.textures[0].normal_map = Some("EXTRA-2".into());
    project
        .textures
        .insert("EXTRA-2".into(), Arc::from(solid_png(1, 1, [0; 4])));
    let err = project.save(&tmp.project()).unwrap_err();
    assert!(
        matches!(
            err,
            ProjectError::DuplicateName {
                kind: "texture",
                ..
            }
        ),
        "{err}"
    );

    let mut project = project_with_normal_map();
    project.model.textures[0].normal_map = Some("../escape".into());
    let err = project.save(&tmp.project()).unwrap_err();
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
    assert!(!tmp.project().exists());
}

#[test]
fn load_requires_normal_map_image() {
    let tmp = TempDir::new("normal-map-missing");
    let dir = tmp.project();
    project_with_normal_map().save(&dir).unwrap();
    fs::remove_file(dir.join("textures/white_normal.png")).unwrap();
    let err = Project::load(&dir).unwrap_err();
    assert!(
        matches!(&err, ProjectError::MissingTexture { name } if name == "white_normal"),
        "{err}"
    );
}

/// 保存 → 開き直す → 書き出す (アトラス化あり・なし) で、法線マップが `NRML` に入る。
#[test]
fn reopened_project_exports_normal_map() {
    let tmp = TempDir::new("normal-map-export");
    let project = project_with_normal_map();
    project.save(&tmp.project()).unwrap();
    let reopened = Project::load(&tmp.project()).unwrap();
    let normal = &project.textures["white_normal"];

    for atlas in [false, true] {
        let settings = crate::ExportSettings {
            atlas,
            ..Default::default()
        };
        let (bytes, report) = crate::export_bundle_with_settings(&reopened, &settings).unwrap();
        assert_eq!(report.atlas.is_some(), atlas);
        let bundle = ugoe_format::read_bundle(&bytes).unwrap();
        let index = bundle
            .model
            .textures
            .iter()
            .position(|t| t.name == "white")
            .unwrap();
        assert_eq!(
            bundle.model.textures[index].normal_map.as_deref(),
            Some("white_normal")
        );
        assert_eq!(bundle.normal_maps.len(), 1, "atlas: {atlas}");
        assert_eq!(bundle.normal_map(index).unwrap().bytes, **normal);
    }
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

/// 法線マップの画像と同じ名前のテクスチャは、`textures/` のファイルが衝突するので足せない。
#[test]
fn add_texture_rejects_normal_map_name() {
    let project = project_with_normal_map();
    let mut doc = project.document();
    let err = doc
        .apply(Command::AddTexture {
            name: "White_Normal".into(),
            png: solid_png(1, 1, [0; 4]),
        })
        .unwrap_err();
    assert_eq!(
        err,
        EditError::DuplicateId {
            kind: "texture",
            id: "White_Normal".into()
        }
    );
    assert_eq!(
        doc.texture("white_normal"),
        Some(&*project.textures["white_normal"])
    );
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

/// テクスチャと法線マップの追加・差し替えは、一辺がデコードの上限を超える PNG を置く前に拒む (T0560)。
/// 上限ちょうどの画像は受け入れる。
#[test]
fn texture_commands_reject_images_larger_than_the_limit() {
    let mut doc = Document::new(fixtures::single_quad());
    let before = doc.model().clone();
    let too_large = |texture: &str, width, height| EditError::TextureTooLarge {
        texture: texture.into(),
        width,
        height,
        max: DEFAULT_MAX_IMAGE_SIDE,
    };
    let wide = DEFAULT_MAX_IMAGE_SIDE + 1;

    let err = doc
        .apply(Command::AddTexture {
            name: "huge".into(),
            png: png_header(wide, 1),
        })
        .unwrap_err();
    assert_eq!(err, too_large("huge", wide, 1));
    assert_eq!(err.to_message().code, "edit.texture_too_large");
    let err = doc
        .apply(Command::SetTextureImage {
            name: "white".into(),
            png: png_header(1, wide),
        })
        .unwrap_err();
    assert_eq!(err, too_large("white", 1, wide));
    let err = doc
        .apply(set_normal_map("white", Some("n"), Some(huge_png_header())))
        .unwrap_err();
    assert_eq!(err, too_large("n", 65535, 65535));
    assert_eq!(doc.model(), &before);
    assert_eq!(doc.texture("huge"), None);
    assert_eq!(doc.texture("n"), None);

    let max = DEFAULT_MAX_IMAGE_SIDE;
    doc.apply(Command::SetTextureImage {
        name: "white".into(),
        png: png_header(max, max),
    })
    .unwrap();
    let texture = &doc.model().textures[0];
    assert_eq!((texture.width, texture.height), (max, max));
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
        texture: None,
        texture_name: Some("Extra-2".into()),
        blend: None,
        culling: None,
        masks: None,
        mask_mode: None,
        mask_only: None,
        lit: None,
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

#[test]
fn texture_by_name_survives_removing_an_earlier_texture() {
    let mut doc = sample_project().document();
    doc.apply(Command::AddTexture {
        name: "Third".into(),
        png: solid_png(1, 1, [0, 0, 255, 255]),
    })
    .unwrap();
    // 提案は今の状態で作る (`Third` は 3 番目、番号 2)。
    let proposal: Command =
        serde_json::from_str(r#"{"op":"set_mesh_properties","id":"Quad","texture_name":"Third"}"#)
            .unwrap();
    let by_index: Command =
        serde_json::from_str(r#"{"op":"set_mesh_properties","id":"Quad","texture":2}"#).unwrap();

    // 提案を適用する前に、前のテクスチャを消して番号が詰まる。
    doc.apply(Command::RemoveTexture {
        name: "Extra-2".into(),
    })
    .unwrap();
    doc.apply(proposal).unwrap();
    let quad = &doc.model().meshes[0];
    assert_eq!(doc.model().textures[quad.texture as usize].name, "Third");

    // 番号の形 (非推奨) は詰まった後の番号を指してしまう (ここでは範囲外で検証に拒否される)。
    assert!(matches!(
        doc.apply(by_index),
        Err(EditError::Invalid(errors))
            if errors.iter().any(|e| matches!(e, ugoe_model::validate::ValidationError::TextureOutOfRange { .. }))
    ));
}

// ---- 法線マップのコマンド ----

fn set_normal_map(texture: &str, normal_map: Option<&str>, png: Option<Vec<u8>>) -> Command {
    Command::SetNormalMap {
        texture: texture.into(),
        normal_map: normal_map.map(Into::into),
        png,
    }
}

#[test]
fn set_normal_map_sets_replaces_and_removes() {
    let mut doc = sample_project().document();
    let flat = solid_png(2, 2, [128, 128, 255, 255]);
    doc.apply(set_normal_map("white", Some("white_n"), Some(flat.clone())))
        .unwrap();
    assert_eq!(
        doc.model().textures[0].normal_map.as_deref(),
        Some("white_n")
    );
    assert_eq!(doc.texture("white_n"), Some(flat.as_slice()));

    // 同じ名前で差し替える。
    let right = solid_png(1, 1, [255, 128, 128, 255]);
    doc.apply(set_normal_map(
        "white",
        Some("white_n"),
        Some(right.clone()),
    ))
    .unwrap();
    assert_eq!(doc.texture("white_n"), Some(right.as_slice()));

    // 別の名前にすると、指されなくなった前の画像は表から消える。
    doc.apply(set_normal_map("white", Some("other"), Some(flat.clone())))
        .unwrap();
    assert_eq!(doc.texture("white_n"), None);
    assert_eq!(doc.texture("other"), Some(flat.as_slice()));

    // 外す。
    doc.apply(set_normal_map("white", None, None)).unwrap();
    assert_eq!(doc.model().textures[0].normal_map, None);
    assert_eq!(doc.texture("other"), None);

    // Undo で画像ごと戻る。
    assert!(doc.undo());
    assert_eq!(doc.model().textures[0].normal_map.as_deref(), Some("other"));
    assert_eq!(doc.texture("other"), Some(flat.as_slice()));
    assert!(ugoe_model::validate::validate(doc.model()).is_empty());
}

#[test]
fn set_normal_map_shares_an_existing_image() {
    let project = project_with_normal_map();
    let mut doc = project.document();
    doc.apply(set_normal_map("Extra-2", Some("white_normal"), None))
        .unwrap();
    assert_eq!(
        doc.model().textures[1].normal_map.as_deref(),
        Some("white_normal")
    );
    // 片方を外しても、もう片方が指しているので画像は残る。
    doc.apply(set_normal_map("white", None, None)).unwrap();
    assert!(doc.texture("white_normal").is_some());
    // 保存しても画像は 1 枚。
    let tmp = TempDir::new("normal-map-command");
    Project::from_document(project.meta.clone(), &doc)
        .save(&tmp.project())
        .unwrap();
    let loaded = Project::load(&tmp.project()).unwrap();
    assert_eq!(loaded.model.textures[0].normal_map, None);
    assert_eq!(
        loaded.model.textures[1].normal_map.as_deref(),
        Some("white_normal")
    );
    // 最後の 1 つを外すと画像も消える。
    doc.apply(set_normal_map("Extra-2", None, None)).unwrap();
    assert_eq!(doc.texture("white_normal"), None);
}

#[test]
fn set_normal_map_rejects_bad_requests() {
    let mut doc = project_with_normal_map().document();
    let before = (doc.model().clone(), doc.textures().clone());
    let png = || Some(solid_png(1, 1, [128, 128, 255, 255]));
    let mut reject = |command: Command| doc.apply(command).unwrap_err();
    assert_eq!(
        reject(set_normal_map("missing", Some("n"), png())),
        EditError::NotFound {
            kind: "texture",
            id: "missing".into()
        }
    );
    // テクスチャの名前と重なる (大文字小文字を区別しない)。
    assert_eq!(
        reject(set_normal_map("white", Some("EXTRA-2"), png())),
        EditError::DuplicateId {
            kind: "texture",
            id: "EXTRA-2".into()
        }
    );
    assert_eq!(
        reject(set_normal_map("white", Some("../n"), png())),
        EditError::InvalidTextureName {
            name: "../n".into()
        }
    );
    assert_eq!(
        reject(set_normal_map("white", Some("n"), Some(b"GIF89a".to_vec()))),
        EditError::NotPng { name: "n".into() }
    );
    // 画像のない名前は共有できない。
    assert_eq!(
        reject(set_normal_map("Extra-2", Some("nothing"), None)),
        EditError::NotFound {
            kind: "normal map",
            id: "nothing".into()
        }
    );
    // 大文字小文字だけが違う法線マップの名前。
    assert!(matches!(
        reject(set_normal_map("Extra-2", Some("WHITE_NORMAL"), png())),
        EditError::InvalidNormalMap { .. }
    ));
    // 名前のない画像。
    assert!(matches!(
        reject(set_normal_map("white", None, png())),
        EditError::InvalidNormalMap { .. }
    ));
    assert_eq!((doc.model().clone(), doc.textures().clone()), before);
}

#[test]
fn remove_texture_drops_its_unused_normal_map() {
    let mut doc = project_with_normal_map().document();
    doc.apply(Command::SetMeshProperties {
        id: "Quad".into(),
        part: None,
        texture: None,
        texture_name: Some("Extra-2".into()),
        blend: None,
        culling: None,
        masks: None,
        mask_mode: None,
        mask_only: None,
        lit: None,
    })
    .unwrap();
    doc.apply(Command::RemoveTexture {
        name: "white".into(),
    })
    .unwrap();
    assert_eq!(doc.texture("white_normal"), None);
    // Undo で表から除いた画像も戻る。
    assert!(doc.undo());
    assert!(doc.texture("white_normal").is_some());
    assert_eq!(
        doc.model().textures[0].normal_map.as_deref(),
        Some("white_normal")
    );
}

#[test]
fn set_lighting_toggles_the_model() {
    let mut doc = Document::new(fixtures::single_quad());
    doc.apply(Command::SetLighting { enabled: true }).unwrap();
    assert!(doc.model().lighting);
    doc.apply(Command::SetLighting { enabled: false }).unwrap();
    assert!(!doc.model().lighting);
}

#[test]
fn set_effects_replaces_the_effects_and_is_validated() {
    use ugoe_model::effects::{Breath, BreathParameter, Effects};
    let mut doc = Document::new(fixtures::single_quad());
    let param = doc.model().parameters[0].id.clone();
    let effects = Effects {
        breath: Breath {
            parameters: vec![BreathParameter {
                parameter: param,
                offset: 0.5,
                peak: 0.5,
                cycle: 3.0,
                weight: 1.0,
            }],
        },
        ..Effects::default()
    };
    doc.apply(Command::SetEffects {
        effects: effects.clone(),
    })
    .unwrap();
    assert_eq!(doc.model().effects, effects);
    // モデルにないパラメータを対象にした設定は検証で拒否し、モデルを変えない。
    let err = doc.apply(Command::SetEffects {
        effects: Effects::standard(),
    });
    assert!(err.is_err());
    assert_eq!(doc.model().effects, effects);
    // 値が範囲の外 (呼吸の周期が 0 以下・目パチの間隔が負) の設定も拒否する。
    let mut bad = effects.clone();
    bad.breath.parameters[0].cycle = 0.0;
    assert!(doc.apply(Command::SetEffects { effects: bad }).is_err());
    let mut bad = effects.clone();
    bad.eye_blink.interval = -1.0;
    assert!(doc.apply(Command::SetEffects { effects: bad }).is_err());
    assert_eq!(doc.model().effects, effects);
    assert!(doc.undo());
    assert_eq!(doc.model().effects, Effects::default());
}
