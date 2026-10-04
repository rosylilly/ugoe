//! モジュールのプロジェクトの保存・読み込み・書き出しのテスト。

use super::*;

#[test]
fn module_project_saves_only_the_module() {
    let temp = tempfile::tempdir().unwrap();
    let base_path = save_base(temp.path());
    let project = Project::new_module("LongHair", &base_path, &"Hair".into()).unwrap();
    let mut document = project.document();
    let module = long_hair_module();
    // モジュールの要素を足す (画像も含めて 1 つのバッチで)。
    let hair = &module.meshes[0];
    let mut mesh = hair.clone();
    mesh.masks.clear();
    mesh.mirror = None;
    mesh.parent = None;
    mesh.id = "Strand".into();
    mesh.part = "Strands".into();
    mesh.texture = document.model().textures.len() as u32;
    document
        .apply(Command::Batch {
            label: "add".into(),
            commands: vec![
                Command::AddPart {
                    part: Part {
                        id: "Strands".into(),
                        name: "Strands".into(),
                        parent: None,
                        keyforms: KeyformGrid::constant(ugoe_model::part::PartKeyform {
                            opacity: 1.0,
                        }),
                    },
                },
                Command::AddTexture {
                    name: "strand".into(),
                    png: solid_png(2, 2, [5, 5, 5, 255]),
                },
                Command::AddMesh { mesh },
            ],
        })
        .unwrap();

    let dir = temp.path().join("hair.ugoeproj");
    let meta = project.meta.clone();
    Project::from_document(meta, &document).save(&dir).unwrap();
    assert!(dir.join("module.json").is_file());
    assert!(!dir.join("model.json").exists());
    assert!(dir.join("textures/strand.png").is_file());
    // 本体の画像はモジュールのプロジェクトに書かない。
    assert_eq!(std::fs::read_dir(dir.join("textures")).unwrap().count(), 1);
    let meta: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("project.json")).unwrap()).unwrap();
    assert_eq!(meta["kind"], "module");
    assert_eq!(meta["module"]["base"], "../avatar.ugoeproj");
    assert_eq!(meta["module"]["slot"], "Hair");

    // 開き直すと同じモジュールになる (作業用のモデルでは、読み込んだ要素の ID に接頭辞が付く)。
    let reopened = Project::load(&dir).unwrap().document();
    let (module, images) = reopened.split_module().unwrap().unwrap();
    assert_eq!(
        (module.clone(), images),
        document.split_module().unwrap().unwrap()
    );
    assert!(
        reopened
            .model()
            .meshes
            .iter()
            .any(|m| m.id.as_str() == "LongHair/Strand")
    );
    assert_eq!(module.meshes[0].id.as_str(), "Strand");
    assert_eq!(module.meshes[0].parent, None);

    // 書き出したモジュールは本体に合成できる。
    let export = export_module(&Project::from_document(
        Project::load(&dir).unwrap().meta,
        &reopened,
    ))
    .unwrap();
    let file = ugoe_format::read_module(&export.bytes).unwrap();
    assert_eq!(file.module.name, "LongHair");
    assert_eq!(file.textures.len(), 1);
    ugoe_runtime::compose(&dress_up_base(), &[&file.module]).unwrap();
}

#[test]
fn missing_base_and_slot_are_reported() {
    let temp = tempfile::tempdir().unwrap();
    let base_path = save_base(temp.path());
    let project = Project::new_module("Hat", &base_path, &"Hair".into()).unwrap();
    let dir = temp.path().join("hat.ugoeproj");
    project.save(&dir).unwrap();
    std::fs::rename(&base_path, temp.path().join("moved.ugoeproj")).unwrap();
    assert!(matches!(
        Project::load(&dir),
        Err(ProjectError::BaseNotFound { .. })
    ));
    assert!(matches!(
        Project::new_module("Hat", &temp.path().join("moved.ugoeproj"), &"Nope".into()),
        Err(ProjectError::Module(ModuleError::UnknownSlot(_)))
    ));
    // 本体にモジュールのプロジェクトは選べない。その本体 (ここでは見つからない) までは読みに行かない。
    assert!(matches!(
        Project::new_module("Hat2", &dir, &"Hair".into()),
        Err(ProjectError::BaseIsModule { .. })
    ));
}

/// 本体を読む前に、解決した本体のパスを確かめる口 (T0524。MCP の `--root`)。
#[test]
fn base_check_runs_before_the_base_is_read() {
    use std::cell::RefCell;

    let temp = tempfile::tempdir().unwrap();
    let base_path = save_base(temp.path());
    let dir = temp.path().join("mods/hat.ugoeproj");
    Project::new_module("Hat", &base_path, &"Hair".into())
        .unwrap()
        .save(&dir)
        .unwrap();

    // `project.json` の参照 (`../../avatar.ugoeproj`) を絶対パスに解決して渡す。断れば本体を読まない。
    let seen = RefCell::new(Vec::new());
    let refuse = |path: &std::path::Path| {
        seen.borrow_mut().push(path.to_path_buf());
        Err("outside".to_owned())
    };
    let err = Project::load_checking_base(&dir, &refuse).unwrap_err();
    assert!(
        matches!(&err, ProjectError::BaseRefused { path, reason } if *path == base_path && reason == "outside"),
        "{err}"
    );
    assert_eq!(*seen.borrow(), vec![base_path.clone()]);
    // 本体が無くても、確かめるのは開く前 (見つからないことは確かめた後で分かる)。
    let moved = temp.path().join("moved.ugoeproj");
    std::fs::rename(&base_path, &moved).unwrap();
    assert!(matches!(
        Project::load_checking_base(&dir, &refuse),
        Err(ProjectError::BaseRefused { .. })
    ));

    // 返したパスから読み、保存する参照は元のまま。
    let redirect = |_: &std::path::Path| Ok(moved.clone());
    let project = Project::load_checking_base(&dir, &redirect).unwrap();
    assert_eq!(project.module.as_ref().unwrap().base().path, base_path);
    assert_eq!(project.model.meshes.len(), dress_up_base().meshes.len());

    // モデルのプロジェクトでは呼ばない。
    let never = |_: &std::path::Path| -> Result<std::path::PathBuf, String> {
        panic!("a model project has no base")
    };
    assert!(Project::load_checking_base(&moved, &never).is_ok());
}

#[test]
fn base_can_be_a_bundle() {
    let temp = tempfile::tempdir().unwrap();
    let base_path = save_base(temp.path());
    let bytes = export_bundle(&Project::load(&base_path).unwrap()).unwrap();
    std::fs::create_dir_all(temp.path()).unwrap();
    let bundle = temp.path().join("avatar.ugoe");
    std::fs::write(&bundle, bytes).unwrap();
    let project = Project::new_module("Hat", &bundle, &"Outfit".into()).unwrap();
    assert_eq!(project.model.meshes.len(), dress_up_base().meshes.len());
    let dir = temp.path().join("mods/hat.ugoeproj");
    project.save(&dir).unwrap();
    let meta: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("project.json")).unwrap()).unwrap();
    assert_eq!(meta["module"]["base"], "../../avatar.ugoe");
    let project = Project::load(&dir).unwrap();
    // プレビューで比べるための本体だけのバンドル (モジュールの要素を含まない)。
    let base_only = ugoe_format::read_bundle(&export_base_bundle(&project).unwrap()).unwrap();
    assert_eq!(base_only.model, dress_up_base());
    assert!(matches!(
        export_base_bundle(&Project::load(&base_path).unwrap()),
        Err(ProjectError::NotAModule)
    ));
}

#[test]
fn model_projects_keep_their_project_json() {
    let meta: ProjectMeta = serde_json::from_str(r#"{"format_version":0,"name":"x"}"#).unwrap();
    assert_eq!(meta.kind, ProjectKind::Model);
    assert_eq!(
        serde_json::to_string(&meta).unwrap(),
        r#"{"format_version":0,"name":"x"}"#
    );
}

/// 本体の版は、名前・スロットを替えても変わらず、本体を読み直すと変わる (プレビューの本体だけのバンドルを作り直す印)。
#[test]
fn base_revision_changes_only_with_the_base() {
    let mut document = self::document(&dress_module());
    let first = document.module().unwrap().base_revision();
    document
        .apply(settings("Gown", "Outfit", ModuleRequirements::default()))
        .unwrap();
    assert_eq!(document.module().unwrap().base_revision(), first);
    assert!(document.undo());
    assert_eq!(document.module().unwrap().base_revision(), first);
    document.rebase_module(base()).unwrap();
    let second = document.module().unwrap().base_revision();
    assert_ne!(second, first);
    // 同じ中身の本体でも、別に読み込んだものは別の版 (プロセスの中で重ならない)。
    assert_ne!(context(&dress_module()).base_revision(), second);
}

/// プレビュー用に名前を替えて書き出すと、名前のほかは書き出しと同じ。使えない名前は拒否する。
#[test]
fn module_can_be_exported_under_another_name() {
    let temp = tempfile::tempdir().unwrap();
    let base_path = save_base(temp.path());
    let project = Project::new_module("Hat", &base_path, &"Hair".into()).unwrap();
    let read = |bytes: Vec<u8>| ugoe_format::read_module(&bytes).unwrap().module;
    let plain = read(export_module(&project).unwrap().bytes);
    let mut renamed = read(export_module_as(&project, "Hat (editing)").unwrap().bytes);
    assert_eq!(renamed.name, "Hat (editing)");
    renamed.name = plain.name.clone();
    assert_eq!(renamed, plain);
    assert!(matches!(
        export_module_as(&project, "a/b"),
        Err(ProjectError::Module(ModuleError::InvalidName(_)))
    ));
}

#[test]
fn base_bundle_fingerprint_follows_the_file() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(temp.path()).unwrap();
    let path = temp.path().join("avatar.ugoe");
    std::fs::write(&path, b"one").unwrap();
    let before = crate::ProjectFingerprint::read_base(&path).unwrap();
    assert_eq!(before, before.reread_base(&path).unwrap());
    std::fs::write(&path, b"two!").unwrap();
    let after = before.reread_base(&path).unwrap();
    assert_ne!(before, after);
    assert_eq!(before.changed_files(&after), vec!["avatar.ugoe".to_owned()]);
}
