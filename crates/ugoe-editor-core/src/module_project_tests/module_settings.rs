//! モジュールの設定 (`SetModuleSettings`) と、本体の読み直し (`rebase_module`) のテスト。

use super::*;

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
    assert!(
        matches!(error, EditError::Module(ModuleError::Compose(_))),
        "{error}"
    );
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

/// `ModuleError` は `Document::apply` のエラーでも種類ごとのコードと引数を保つ (英語の文に畳まない。T0678)。
#[test]
fn module_errors_keep_their_codes_through_apply() {
    use crate::Coded;

    let mut document = document(&long_hair_module());
    let error = document
        .apply(settings(
            "LongHair",
            "Nowhere",
            ModuleRequirements::default(),
        ))
        .unwrap_err();
    assert_eq!(
        error,
        EditError::Module(ModuleError::UnknownSlot("Nowhere".into()))
    );
    let message = error.to_message();
    assert_eq!(message.code, "module.unknown_slot");
    assert_eq!(message.args, crate::message_args! { "slot" => "Nowhere" });

    // `Outfit` は `Face` をマスクに公開しないので合成できない。
    let error = document
        .apply(settings(
            "LongHair",
            "Outfit",
            ModuleRequirements::default(),
        ))
        .unwrap_err();
    assert!(
        matches!(error, EditError::Module(ModuleError::Compose(_))),
        "{error:?}"
    );
    assert_eq!(error.to_message().code, "module.compose_failed");

    // 読み取り専用の本体・モジュールとして表せないものは、編集のコードのまま。
    assert_eq!(
        EditError::from(ModuleError::ReadOnlyBase {
            element: "x".into()
        })
        .code(),
        "edit.read_only_base"
    );
    assert_eq!(
        EditError::from(ModuleError::Shape { reason: "x".into() }).code(),
        "edit.module_shape"
    );
}

/// モジュールのプロジェクトでないときの `rebase_module_on` のエラーは、ほかの「モジュールでない」と同じ種類になる。
#[test]
fn rebasing_a_model_project_is_not_a_module_error() {
    let mut document = Document::new(dress_up_base());
    let error = document.rebase_module(base()).unwrap_err();
    assert_eq!(error, ModuleError::NotModule);
    assert_eq!(EditError::from(error.clone()), EditError::NotModule);
    assert!(matches!(
        ProjectError::from(error),
        ProjectError::NotAModule
    ));
}
