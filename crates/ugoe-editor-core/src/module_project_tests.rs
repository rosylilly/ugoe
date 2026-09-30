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
    Command, Document, EditError, Project, ProjectError, ProjectKind, ProjectMeta,
    export_base_bundle, export_bundle, export_module, export_module_as,
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

fn settings(name: &str, slot: &str, requires: ModuleRequirements) -> Command {
    Command::SetModuleSettings {
        name: name.to_owned(),
        slot: slot.into(),
        requires,
        keep_look: false,
    }
}

#[test]
fn module_settings_rename_the_module_and_undo() {
    let module = long_hair_module();
    let mut document = document(&module);
    document
        .apply(settings("Braid", "Hair", module.requires.clone()))
        .unwrap();
    let context = document.module().unwrap();
    assert_eq!(context.name(), "Braid");
    assert!(
        document
            .model()
            .meshes
            .iter()
            .any(|m| m.id.as_str() == "Braid/Back")
    );
    assert_eq!(context.display_id("Braid/Back"), "Back");
    assert_eq!(context.display_id("Face"), "Face");
    let (split, _) = document.split_module().unwrap().unwrap();
    assert_eq!(split.name, "Braid");
    assert_same_module(&split, &module);
    assert!(document.is_modified());

    assert!(document.undo());
    assert_eq!(document.module().unwrap().name(), "LongHair");
    assert!(!document.is_modified());
    assert!(document.redo());
    assert_eq!(document.module().unwrap().name(), "Braid");
}

#[test]
fn module_settings_move_to_another_slot() {
    let mut document = Document::new(dress_up_base());
    assert_eq!(
        document.apply(settings("X", "Hair", ModuleRequirements::default())),
        Err(EditError::NotModule)
    );

    let module = long_hair_module();
    let mut document = self::document(&module);
    // `Outfit` は `Face` をマスクに公開しないので、LongHair は移せない (何も変わらない)。
    let error = document
        .apply(settings(
            "LongHair",
            "Outfit",
            ModuleRequirements::default(),
        ))
        .unwrap_err();
    assert!(matches!(error, EditError::ModuleShape { .. }), "{error}");
    assert_eq!(document.module().unwrap().slot().id.as_str(), "Hair");
    assert!(!document.is_modified());

    // 空のモジュールは移せる。描画順の写し方と取り付け先が新しいスロットのものになる。
    let mut empty = module.clone();
    empty.meshes.clear();
    empty.deformers.clear();
    empty.physics = Default::default();
    empty.textures.clear();
    let mut document = self::document(&empty);
    document
        .apply(settings("Hat", "Outfit", ModuleRequirements::default()))
        .unwrap();
    let context = document.module().unwrap();
    assert_eq!(context.slot().id.as_str(), "Outfit");
    assert_eq!(context.map_draw_order(0.0), 300.0);
    // 条件は本体が満たすものだけ。
    let error = document
        .apply(settings(
            "Hat",
            "Outfit",
            ModuleRequirements {
                parameters: vec!["Missing".into()],
                ..Default::default()
            },
        ))
        .unwrap_err();
    assert!(error.to_string().contains("Missing"), "{error}");
    document
        .apply(settings(
            "Hat",
            "Outfit",
            ModuleRequirements {
                slots: vec!["Hair".into()],
                ..Default::default()
            },
        ))
        .unwrap();
    assert_eq!(split(&document).requires.slots, vec!["Hair".into()]);
    // バッチの中では使えない。
    let error = document
        .apply(Command::Batch {
            label: "x".into(),
            commands: vec![settings("Cap", "Outfit", ModuleRequirements::default())],
        })
        .unwrap_err();
    assert!(matches!(error, EditError::ModuleShape { .. }), "{error}");
}

#[test]
fn read_only_message_points_to_the_base_project() {
    let mut document = document(&long_hair_module());
    let error = document
        .apply(Command::RemoveMesh { id: "Face".into() })
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "mesh `Face` belongs to the base model and can't be edited here. Edit it in the base project"
    );
}

#[test]
fn rebase_keeps_unsaved_module_edits() {
    let temp = TempDir::new("rebase");
    let base_path = save_base(&temp.0);
    let project = Project::new_module("Hat", &base_path, &"Hair".into()).unwrap();
    let dir = temp.0.join("hat.ugoeproj");
    project.save(&dir).unwrap();
    let before = crate::ProjectFingerprint::read_base(&base_path).unwrap();
    let mut document = Project::load(&dir).unwrap().document();
    document
        .apply(Command::AddPart {
            part: Part {
                id: "Brim".into(),
                name: "Brim".into(),
                parent: None,
                keyforms: KeyformGrid::constant(ugoe_model::part::PartKeyform { opacity: 1.0 }),
            },
        })
        .unwrap();

    // 本体を外で変える (パラメータを足し、スロットの範囲を変える)。
    let mut base = Project::load(&base_path).unwrap();
    base.model
        .parameters
        .push(dress_up_base().parameters[0].clone());
    base.model.parameters.last_mut().unwrap().id = "Extra".into();
    base.model.slots[0].draw_order.max = 600.0;
    base.save(&base_path).unwrap();
    let after = before.reread_base(&base_path).unwrap();
    assert_ne!(before, after);

    let state = document.state_id();
    document
        .rebase_module(crate::project::load_base(&base_path).unwrap())
        .unwrap();
    assert!(
        document
            .model()
            .parameters
            .iter()
            .any(|p| p.id.as_str() == "Extra")
    );
    // 合成し直すので、新しく作った要素の ID にも接頭辞が付く。
    assert!(
        document
            .model()
            .parts
            .iter()
            .any(|p| p.id.as_str() == "Hat/Brim")
    );
    assert_eq!(document.module().unwrap().slot().draw_order.max, 600.0);
    assert!(document.is_modified());
    assert!(document.state_id() > state);
    assert!(!document.undo());

    // 本体からスロットがなくなると読み直せない。別のスロットを選んで開ける。
    base.model.slots.remove(0);
    base.model.avatar = None;
    base.save(&base_path).unwrap();
    let before = document.model().clone();
    assert!(matches!(
        document.rebase_module(crate::project::load_base(&base_path).unwrap()),
        Err(ModuleError::UnknownSlot(_))
    ));
    assert_eq!(*document.model(), before);
    assert!(matches!(
        Project::load(&dir),
        Err(ProjectError::Module(ModuleError::UnknownSlot(_)))
    ));
    let (link_base, link_slot) = crate::project::read_module_link(&dir).unwrap();
    assert_eq!(link_base, base_path);
    assert_eq!(link_slot.as_str(), "Hair");
    let reopened = Project::load_module_with(&dir, None, Some(&"Outfit".into())).unwrap();
    assert_eq!(
        reopened.module.as_ref().unwrap().slot().id.as_str(),
        "Outfit"
    );
    // 開き直さずに、別のスロットで本体を読み直すこともできる (保存していない編集は残る)。
    document
        .rebase_module_on(
            crate::project::load_base(&base_path).unwrap(),
            Some(&"Outfit".into()),
        )
        .unwrap();
    assert_eq!(document.module().unwrap().slot().id.as_str(), "Outfit");
    assert!(
        document
            .model()
            .parts
            .iter()
            .any(|p| p.id.as_str() == "Hat/Brim")
    );
    assert!(document.is_modified());
}

/// 取り付け先の違うスロットへ移すとき、`keep_look` なら見た目 (基準の姿) を保つ。
#[test]
fn module_settings_keep_the_look_when_the_anchor_changes() {
    // Dress (取り付け先 Torso) を、パラメータとマスクを使わない形にして Hair (取り付け先 Head) へ移す。
    let mut module = dress_module();
    module.requires = ModuleRequirements::default();
    let rest = module.meshes[0].keyforms.keyforms[0].clone();
    module.meshes[0].keyforms = KeyformGrid::constant(rest);
    let positions = |document: &Document| -> Vec<Vec2> {
        let model = document.model().clone();
        let mut instance = ugoe_runtime::ModelInstance::new(
            ugoe_runtime::CompiledModel::compile(model.clone()).unwrap(),
        );
        instance.update();
        let index = model
            .meshes
            .iter()
            .position(|m| m.id.as_str() == "Dress/Skirt")
            .unwrap();
        instance.drawables()[index].positions.clone()
    };

    let mut document = self::document(&module);
    let before = positions(&document);
    let mut command = settings("Dress", "Hair", ModuleRequirements::default());
    let Command::SetModuleSettings { keep_look, .. } = &mut command else {
        unreachable!()
    };
    *keep_look = true;
    document.apply(command).unwrap();
    assert_eq!(document.module().unwrap().slot().id.as_str(), "Hair");
    let skirt = document
        .model()
        .meshes
        .iter()
        .find(|m| m.id.as_str() == "Dress/Skirt")
        .unwrap();
    assert_eq!(skirt.parent.as_ref().map(|p| p.as_str()), Some("Head"));
    for (a, b) in before.iter().zip(positions(&document)) {
        assert!((*a - b).length() < 1e-3, "{a:?} != {b:?}");
    }
    // 分けたモジュールの座標は Head の局所座標になっている (Torso の UV のままではない)。
    let split = split(&document);
    assert_ne!(split.meshes[0].keyforms, module.meshes[0].keyforms);
    assert!(document.undo());
    assert_eq!(document.module().unwrap().slot().id.as_str(), "Outfit");

    // `keep_look` でなければ局所座標のままなので、見た目が変わる。
    let mut document = self::document(&module);
    document
        .apply(settings("Dress", "Hair", ModuleRequirements::default()))
        .unwrap();
    let moved = positions(&document);
    assert!(
        before
            .iter()
            .zip(&moved)
            .any(|(a, b)| (*a - *b).length() > 1.0)
    );
}

/// `keep_look` で取り付け先を替えると、モジュールの衝突体 (取り付け先に付くもの) も見た目の位置を保ち、
/// リグの根元 (モジュールの回転デフォーマに付く) も同じ位置に残る。Undo で戻る。
#[test]
fn module_settings_keep_physics_in_place_when_the_anchor_changes() {
    // LongHair (取り付け先 Head) を、マスクとパラメータを使わない形にして Outfit (取り付け先 Torso) へ移す。
    let mut module = long_hair_module();
    for mesh in &mut module.meshes {
        mesh.masks.clear();
    }
    if let ugoe_model::deformer::DeformerKind::Rotation(r) = &mut module.deformers[0].kind {
        let rest = r.keyforms.keyforms[0];
        r.keyforms = KeyformGrid::constant(rest);
    }
    module.physics.rigs[0].inputs.clear();
    // 衝突体の位置 (キャンバス座標) と、リグの根元 (`Sway` の局所座標の粒子 0) のキャンバス座標。
    let canvas = |document: &Document| -> (Vec2, Vec2) {
        let model = document.model().clone();
        let physics = document.physics().unwrap();
        let mut instance = ugoe_runtime::ModelInstance::new(
            ugoe_runtime::CompiledModel::compile(model.clone()).unwrap(),
        );
        instance.update();
        let index = |id: &str| {
            model
                .deformers
                .iter()
                .position(|d| d.id.as_str() == id)
                .unwrap()
        };
        let collider = &physics.colliders[0];
        let ugoe_model::physics::ColliderShape::Circle { center, .. } = collider.shape else {
            unreachable!()
        };
        let attach = index(collider.attach.as_ref().unwrap().as_str());
        let rig = &physics.rigs[0];
        (
            instance.deformer_world_point(attach, center),
            instance.deformer_world_point(index("LongHair/Sway"), rig.particles[0].position),
        )
    };

    let mut document = self::document(&module);
    let (collider, root) = canvas(&document);
    let original = document.physics().cloned();
    let mut command = settings("LongHair", "Outfit", ModuleRequirements::default());
    let Command::SetModuleSettings { keep_look, .. } = &mut command else {
        unreachable!()
    };
    *keep_look = true;
    document.apply(command).unwrap();
    let physics = document.physics().unwrap();
    assert_eq!(
        physics.colliders[0].attach.as_ref().map(|a| a.as_str()),
        Some("Torso")
    );
    // 粒子は写し直さない (根元はモジュールのデフォーマの局所座標なので、デフォーマと一緒に保たれる)。
    assert_eq!(physics.rigs[0].particles, module.physics.rigs[0].particles);
    let (moved_collider, moved_root) = canvas(&document);
    assert!(
        (collider - moved_collider).length() < 1e-3,
        "{collider:?} != {moved_collider:?}"
    );
    assert!(
        (root - moved_root).length() < 1e-3,
        "{root:?} != {moved_root:?}"
    );
    // 分けたモジュールの衝突体は Torso の UV で表される。
    assert_ne!(
        split(&document).physics.colliders[0].shape,
        module.physics.colliders[0].shape
    );
    assert!(document.undo());
    assert_eq!(document.physics().cloned(), original);

    // `keep_look` でなければ、衝突体は Head の局所座標のまま Torso に付く。
    let mut document = self::document(&module);
    document
        .apply(settings(
            "LongHair",
            "Outfit",
            ModuleRequirements::default(),
        ))
        .unwrap();
    assert_eq!(
        split(&document).physics.colliders[0].shape,
        module.physics.colliders[0].shape
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
    let temp = TempDir::new("export-as");
    let base_path = save_base(&temp.0);
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
    let temp = TempDir::new("bundle-fingerprint");
    std::fs::create_dir_all(&temp.0).unwrap();
    let path = temp.0.join("avatar.ugoe");
    std::fs::write(&path, b"one").unwrap();
    let before = crate::ProjectFingerprint::read_base(&path).unwrap();
    assert_eq!(before, before.reread_base(&path).unwrap());
    std::fs::write(&path, b"two!").unwrap();
    let after = before.reread_base(&path).unwrap();
    assert_ne!(before, after);
    assert_eq!(before.changed_files(&after), vec!["avatar.ugoe".to_owned()]);
}
