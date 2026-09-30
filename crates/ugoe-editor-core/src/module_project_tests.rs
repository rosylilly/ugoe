//! モジュールのプロジェクト (`crate::module_project`) のテスト。

use std::sync::Arc;

use ugoe_core::Rgb;
use ugoe_core::Vec2;
use ugoe_model::fixtures::{dress_module, dress_up_base, long_hair_module};
use ugoe_model::mesh::MeshKeyform;
use ugoe_model::module::ModuleRequirements;
use ugoe_model::{KeyformGrid, Mesh, Module, Part, PartId};

use crate::module_project::{ModuleBase, ModuleContext, ModuleError};
use crate::texture::{Textures, solid_png};
use crate::{
    Command, Document, EditError, Project, ProjectError, ProjectKind, ProjectMeta, export_bundle,
    export_module,
};

fn base() -> ModuleBase {
    let model = dress_up_base();
    let mut textures = Textures::new();
    for texture in &model.textures {
        textures.insert(texture.name.clone(), Arc::from(solid_png(4, 4, [255; 4])));
    }
    ModuleBase {
        path: "/base.ugoeproj".into(),
        model,
        physics: None,
        textures,
    }
}

fn images(module: &Module) -> Textures {
    module
        .textures
        .iter()
        .map(|t| (t.name.clone(), Arc::from(solid_png(2, 2, [1, 2, 3, 255]))))
        .collect()
}

fn context(module: &Module) -> ModuleContext {
    ModuleContext::new(
        base(),
        &module.slot,
        module.name.clone(),
        module.requires.clone(),
    )
    .unwrap()
}

/// `module` の作業用のドキュメント。
fn document(module: &Module) -> Document {
    let context = Arc::new(context(module));
    let workspace = context.workspace(module, &images(module)).unwrap();
    Document::with_textures(workspace.model, workspace.textures)
        .with_physics(workspace.physics)
        .with_module(context)
}

fn split(document: &Document) -> Module {
    document.split_module().unwrap().unwrap().0
}

fn assert_same_module(actual: &Module, expected: &Module) {
    assert_eq!(actual.parts, expected.parts);
    assert_eq!(actual.deformers, expected.deformers);
    assert_eq!(actual.glues, expected.glues);
    assert_eq!(actual.textures, expected.textures);
    assert_eq!(actual.physics, expected.physics);
    assert_eq!(actual.pose, expected.pose);
    assert_eq!(actual.meshes.len(), expected.meshes.len());
    for (a, e) in actual.meshes.iter().zip(&expected.meshes) {
        let orders = |m: &Mesh| -> Vec<f32> {
            m.keyforms
                .all_keyforms()
                .map(|k| k.draw_order)
                .collect::<Vec<_>>()
        };
        for (x, y) in orders(a).into_iter().zip(orders(e)) {
            assert!((x - y).abs() < 1e-3, "{}: {x} != {y}", a.id);
        }
        let mut a = a.clone();
        let mut e = e.clone();
        for k in a.keyforms.all_keyforms_mut() {
            k.draw_order = 0.0;
        }
        for k in e.keyforms.all_keyforms_mut() {
            k.draw_order = 0.0;
        }
        assert_eq!(a, e);
    }
}

#[test]
fn workspace_splits_back_into_the_module() {
    for module in [long_hair_module(), dress_module()] {
        let document = document(&module);
        // 作業用のモデルは本体と合成したもの (ID に接頭辞、最上位は取り付け先の下)。
        let prefix = format!("{}/", module.name);
        assert!(
            document
                .model()
                .meshes
                .iter()
                .any(|m| m.id.as_str().starts_with(&prefix))
        );
        let (split, images) = document.split_module().unwrap().unwrap();
        assert_same_module(&split, &module);
        assert_eq!(split.name, module.name);
        assert_eq!(split.slot, module.slot);
        for texture in &module.textures {
            assert!(images.contains_key(&texture.name));
        }
    }
}

#[test]
fn base_elements_are_read_only() {
    let mut document = document(&long_hair_module());
    let error = document
        .apply(Command::SetParameterRange {
            id: "Secret".into(),
            min: 0.0,
            default: 0.0,
            max: 2.0,
        })
        .unwrap_err();
    assert_eq!(
        error,
        EditError::ReadOnlyBase {
            element: "parameter `Secret`".into()
        }
    );
    let error = document
        .apply(Command::SetMeshDrawOrder {
            id: "Face".into(),
            draw_order: 10.0,
        })
        .unwrap_err();
    assert_eq!(
        error,
        EditError::ReadOnlyBase {
            element: "mesh `Face`".into()
        }
    );
    assert!(!document.is_modified());
}

#[test]
fn module_elements_can_be_edited() {
    let module = long_hair_module();
    let mut document = document(&module);
    let id = format!("{}/{}", module.name, module.meshes[0].id);
    // 本体の空間の描画順 (スロットの範囲 550〜650) で編集し、モジュールの空間に戻して保存する。
    document
        .apply(Command::SetMeshDrawOrder {
            id: id.as_str().into(),
            draw_order: 625.0,
        })
        .unwrap();
    let split = split(&document);
    for keyform in split.meshes[0].keyforms.all_keyforms() {
        assert!((keyform.draw_order - 750.0).abs() < 1e-3);
    }
}

fn canvas_mesh(id: &str, part: &str, texture: u32) -> Mesh {
    let rect = [
        Vec2::new(100.0, 40.0),
        Vec2::new(140.0, 40.0),
        Vec2::new(100.0, 80.0),
        Vec2::new(140.0, 80.0),
    ];
    Mesh {
        id: id.into(),
        name: id.into(),
        part: part.into(),
        parent: None,
        texture,
        uvs: vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
        ],
        indices: vec![0, 1, 2, 1, 3, 2],
        blend: Default::default(),
        culling: false,
        masks: Vec::new(),
        mask_mode: Default::default(),
        mask_only: false,
        keyforms: KeyformGrid::constant(MeshKeyform {
            positions: rect.to_vec(),
            opacity: 1.0,
            draw_order: 500.0,
            multiply_color: Rgb::WHITE,
            screen_color: Rgb::BLACK,
            mask_strength: 1.0,
        }),
        mirror: None,
        mirror_axis: None,
        depth: None,
        lit: false,
    }
}

#[test]
fn new_canvas_elements_move_under_the_slot() {
    let module = dress_module();
    let mut document = document(&module);
    let texture = document.model().textures.len() as u32 - 1;
    let mut command = Command::Batch {
        label: "add".into(),
        commands: vec![
            Command::AddPart {
                part: Part {
                    id: "Coat".into(),
                    name: "Coat".into(),
                    parent: None,
                    keyforms: KeyformGrid::constant(ugoe_model::part::PartKeyform { opacity: 1.0 }),
                },
            },
            Command::AddMesh {
                mesh: canvas_mesh("CoatMesh", "Coat", texture),
            },
        ],
    };
    document.module().unwrap().map_new_meshes(&mut command);
    document.apply(command).unwrap();
    let mesh = document
        .model()
        .meshes
        .iter()
        .find(|m| m.id.as_str() == "CoatMesh")
        .unwrap();
    // 見た目を保ったまま取り付け先 (ワープ `Torso`) の下へ移り、描画順はスロットの範囲 (300〜400) に写る。
    assert_eq!(mesh.parent.as_ref().unwrap().as_str(), "Torso");
    assert!((mesh.keyforms.keyforms[0].draw_order - 350.0).abs() < 1e-3);
    let coat = document
        .model()
        .parts
        .iter()
        .find(|p| p.id.as_str() == "Coat")
        .unwrap();
    assert_eq!(coat.parent, Some(PartId::new("Body")));

    let split = split(&document);
    let mesh = split
        .meshes
        .iter()
        .find(|m| m.id.as_str() == "CoatMesh")
        .unwrap();
    assert_eq!(mesh.parent, None);
    assert_eq!(mesh.texture, 0);
    assert!((mesh.keyforms.keyforms[0].draw_order - 500.0).abs() < 1e-3);
    // ワープの UV 空間 (Torso は 64〜192 × 128〜256) で表す。
    let p = mesh.keyforms.keyforms[0].positions[0];
    assert!((p.x - (100.0 - 64.0) / 128.0).abs() < 1e-3, "{p:?}");
    let coat = split
        .parts
        .iter()
        .find(|p| p.id.as_str() == "Coat")
        .unwrap();
    assert_eq!(coat.parent, None);
}

#[test]
fn references_to_base_elements_are_rejected() {
    let module = dress_module();
    let mut document = document(&module);
    // 本体のパーツに属するメッシュはモジュールとして表せない。
    let texture = document.model().textures.len() as u32 - 1;
    let error = document
        .apply(Command::AddMesh {
            mesh: canvas_mesh("Stray", "Body", texture),
        })
        .unwrap_err();
    assert!(matches!(error, EditError::ModuleShape { .. }), "{error}");
    // 本体のテクスチャを使うメッシュも同じ。
    let error = document
        .apply(Command::AddMesh {
            mesh: canvas_mesh("Stray", "Dress/Dress", 0),
        })
        .unwrap_err();
    assert!(matches!(error, EditError::ModuleShape { .. }), "{error}");
}

#[test]
fn ids_that_collide_without_the_prefix_are_rejected() {
    let module = dress_module();
    let mut document = document(&module);
    let error = document
        .apply(Command::AddPart {
            part: Part {
                id: "Dress".into(),
                name: "Dress".into(),
                parent: None,
                keyforms: KeyformGrid::constant(ugoe_model::part::PartKeyform { opacity: 1.0 }),
            },
        })
        .unwrap_err();
    assert!(
        matches!(&error, EditError::ModuleShape { reason } if reason.contains("would both be `Dress`")),
        "{error}"
    );
}

#[test]
fn module_textures_can_be_added() {
    let module = long_hair_module();
    let mut document = document(&module);
    document
        .apply(Command::AddTexture {
            name: "ribbon".into(),
            png: solid_png(2, 2, [9, 9, 9, 255]),
        })
        .unwrap();
    let (split, images) = document.split_module().unwrap().unwrap();
    assert_eq!(
        split.textures.last().map(|t| t.name.as_str()),
        Some("ribbon")
    );
    assert!(images.contains_key("ribbon"));
    // 本体のテクスチャの画像は変えられない。
    let base_texture = document.model().textures[0].name.clone();
    let error = document
        .apply(Command::SetTextureImage {
            name: base_texture,
            png: solid_png(4, 4, [0, 0, 0, 255]),
        })
        .unwrap_err();
    assert!(matches!(error, EditError::ReadOnlyBase { .. }), "{error}");
}

#[test]
fn unknown_slot_and_invalid_name() {
    let error = ModuleContext::new(base(), &"Nope".into(), "Hat", ModuleRequirements::default())
        .unwrap_err();
    assert_eq!(error, ModuleError::UnknownSlot("Nope".into()));
    let error = ModuleContext::new(base(), &"Hair".into(), "a/b", ModuleRequirements::default())
        .unwrap_err();
    assert_eq!(error, ModuleError::InvalidName("a/b".into()));
}

#[test]
fn empty_module_splits_to_empty() {
    let context = Arc::new(
        ModuleContext::new(base(), &"Hair".into(), "Hat", ModuleRequirements::default()).unwrap(),
    );
    let workspace = context
        .workspace(&context.empty_module(), &Textures::new())
        .unwrap();
    assert_eq!(workspace.physics, None);
    let document =
        Document::with_textures(workspace.model, workspace.textures).with_module(context.clone());
    let split = split(&document);
    assert_eq!(split, context.empty_module());
}

// ---- プロジェクト ----

/// テスト用の一時ディレクトリ。破棄時に消す。
struct TempDir(std::path::PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "ugoe-editor-core-module-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// 本体のプロジェクトを `dir/avatar.ugoeproj` に保存する。
fn save_base(dir: &std::path::Path) -> std::path::PathBuf {
    let base = base();
    let path = dir.join("avatar.ugoeproj");
    Project::new(ProjectMeta::new("avatar"), base.model, base.textures)
        .save(&path)
        .unwrap();
    path
}

#[test]
fn module_project_saves_only_the_module() {
    let temp = TempDir::new("save");
    let base_path = save_base(&temp.0);
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

    let dir = temp.0.join("hair.ugoeproj");
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
    let temp = TempDir::new("missing");
    let base_path = save_base(&temp.0);
    let project = Project::new_module("Hat", &base_path, &"Hair".into()).unwrap();
    let dir = temp.0.join("hat.ugoeproj");
    project.save(&dir).unwrap();
    std::fs::rename(&base_path, temp.0.join("moved.ugoeproj")).unwrap();
    assert!(matches!(
        Project::load(&dir),
        Err(ProjectError::BaseNotFound { .. })
    ));
    assert!(matches!(
        Project::new_module("Hat", &temp.0.join("moved.ugoeproj"), &"Nope".into()),
        Err(ProjectError::Module(ModuleError::UnknownSlot(_)))
    ));
    // 本体にモジュールのプロジェクトは選べない。
    assert!(matches!(
        Project::new_module("Hat2", &dir, &"Hair".into()),
        Err(ProjectError::BaseIsModule { .. } | ProjectError::Base { .. })
    ));
}

#[test]
fn base_can_be_a_bundle() {
    let temp = TempDir::new("bundle");
    let base_path = save_base(&temp.0);
    let bytes = export_bundle(&Project::load(&base_path).unwrap()).unwrap();
    std::fs::create_dir_all(&temp.0).unwrap();
    let bundle = temp.0.join("avatar.ugoe");
    std::fs::write(&bundle, bytes).unwrap();
    let project = Project::new_module("Hat", &bundle, &"Outfit".into()).unwrap();
    assert_eq!(project.model.meshes.len(), dress_up_base().meshes.len());
    let dir = temp.0.join("mods/hat.ugoeproj");
    project.save(&dir).unwrap();
    let meta: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("project.json")).unwrap()).unwrap();
    assert_eq!(meta["module"]["base"], "../../avatar.ugoe");
    Project::load(&dir).unwrap();
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
