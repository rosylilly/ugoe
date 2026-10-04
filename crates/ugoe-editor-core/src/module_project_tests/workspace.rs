//! 作業用のドキュメント (合成と分割・本体の要素の読み取り専用・モジュールの要素とテクスチャの編集) のテスト。

use super::*;

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

// 合成で接頭辞を付け、分けるときに外すミラー元 (`mirror_source`) は、ブレンドシェイプ格子のものも付け替える。
#[test]
fn blend_shape_mirror_sources_are_renamed_both_ways() {
    let mut module = long_hair_module();
    let front = &mut module.meshes[1];
    let rest = front.keyforms.keyforms[0].clone();
    let mut shape = KeyformGrid::new(
        vec![KeyformBinding {
            parameter: "Smile".into(),
            keys: vec![0.0, 1.0],
        }],
        vec![rest.clone(), rest],
    );
    shape.mirror_source = Some(MirrorSource {
        id: "Back".into(),
        axis: None,
        tolerance: None,
    });
    front.keyforms.mirror_source = shape.mirror_source.clone();
    front.keyforms.blend_shapes = vec![shape];

    // 本体にブレンドシェイプのパラメータ `Smile` を足し、スロット `Hair` で使えるようにする。
    let mut base = base();
    base.model.parameters.push(Parameter {
        id: "Smile".into(),
        name: "Smile".into(),
        min: 0.0,
        default: 0.0,
        max: 1.0,
        kind: ParameterKind::BlendShape,
        mirror: None,
        group: None,
        internal: false,
    });
    let slot = base
        .model
        .slots
        .iter_mut()
        .find(|s| s.id == module.slot)
        .unwrap();
    slot.parameters.push("Smile".into());
    let context = ModuleContext::new(
        base,
        &module.slot,
        module.name.clone(),
        module.requires.clone(),
    )
    .unwrap();
    let context = Arc::new(context);
    let workspace = context.workspace(&module, &images(&module)).unwrap();
    let document = Document::with_textures(workspace.model, workspace.textures)
        .with_physics(workspace.physics)
        .with_module(context);
    let workspace = document
        .model()
        .meshes
        .iter()
        .find(|m| m.id.as_str() == "LongHair/Front")
        .unwrap();
    let source = |grid: &KeyformGrid<MeshKeyform>| grid.mirror_source.clone().unwrap().id;
    assert_eq!(source(&workspace.keyforms), "LongHair/Back");
    assert_eq!(source(&workspace.keyforms.blend_shapes[0]), "LongHair/Back");
    assert_same_module(&split(&document), &module);
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

/// 法線マップの画像もテクスチャと同じ表に置くので、接頭辞を外すと名前が重なる画像は拒否する
/// (黙って片方の画像で上書きしない)。
#[test]
fn normal_maps_that_collide_without_the_prefix_are_rejected() {
    let mut module = long_hair_module();
    module.textures[0].normal_map = Some("normal".into());
    let mut images = images(&module);
    images.insert(
        "normal".into(),
        Arc::from(solid_png(2, 2, [128, 128, 255, 255])),
    );
    let context = Arc::new(context(&module));
    let workspace = context.workspace(&module, &images).unwrap();
    let mut document = Document::with_textures(workspace.model, workspace.textures)
        .with_physics(workspace.physics)
        .with_module(context);
    document
        .apply(Command::AddTexture {
            name: "ribbon".into(),
            png: solid_png(2, 2, [9, 9, 9, 255]),
        })
        .unwrap();
    // 作業用の名前は `LongHair/normal` なので、`normal` は別の名前として受け付けてしまう。
    let error = document
        .apply(Command::SetNormalMap {
            texture: "ribbon".into(),
            normal_map: Some("normal".into()),
            png: Some(solid_png(2, 2, [0, 0, 255, 255])),
        })
        .unwrap_err();
    assert!(
        matches!(&error, EditError::ModuleShape { reason } if reason.contains("would both be `normal`")),
        "{error}"
    );
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
