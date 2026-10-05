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
    assert!(document.model().meshes.iter().any(|m| m.id == "Braid/Back"));
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
    assert_eq!(split(&document).requires.slots, vec!["Hair"]);
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
    let temp = tempfile::tempdir().unwrap();
    let base_path = save_base(temp.path());
    let project = Project::new_module("Hat", &base_path, &"Hair".into()).unwrap();
    let dir = temp.path().join("hat.ugoeproj");
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
    assert!(document.model().parameters.iter().any(|p| p.id == "Extra"));
    // 合成し直すので、新しく作った要素の ID にも接頭辞が付く。
    assert!(document.model().parts.iter().any(|p| p.id == "Hat/Brim"));
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
    assert!(document.model().parts.iter().any(|p| p.id == "Hat/Brim"));
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
            .position(|m| m.id == "Dress/Skirt")
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
        .find(|m| m.id == "Dress/Skirt")
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

/// `keep_look` の写し直しでミラーし直せなかった左右の対は、拒否せずに `ApplyReport` の警告で返す (T1609)。
#[test]
fn module_settings_keep_look_reports_failed_remirrors() {
    let mut module = dress_module();
    module.requires = ModuleRequirements::default();
    let rest = module.meshes[0].keyforms.keyforms[0].clone();
    module.meshes[0].keyforms = KeyformGrid::constant(rest);
    // `Skirt` をミラー元にする `SkirtR`。頂点を対称軸から離しておくので、ミラーし直すと対応する頂点がない。
    let mut right = module.meshes[0].clone();
    right.id = "SkirtR".into();
    for keyform in right.keyforms.all_keyforms_mut() {
        for p in &mut keyform.positions {
            p.x += 5.0;
        }
    }
    right.keyforms.origins = vec![ugoe_model::KeyformOrigin::Mirror];
    right.keyforms.mirror_source = Some(MirrorSource {
        id: "Skirt".into(),
        axis: None,
        tolerance: None,
    });
    module.meshes.push(right);

    let mut document = self::document(&module);
    let right_before = document
        .model()
        .meshes
        .iter()
        .find(|m| m.id == "Dress/SkirtR")
        .unwrap()
        .keyforms
        .clone();
    let mut command = settings("Dress", "Hair", ModuleRequirements::default());
    let Command::SetModuleSettings { keep_look, .. } = &mut command else {
        unreachable!()
    };
    *keep_look = true;
    let report = document.apply_with_report(command.clone()).unwrap();
    assert_eq!(document.module().unwrap().slot().id.as_str(), "Hair");
    assert_eq!(report.warnings.len(), 1, "{:?}", report.warnings);
    let warning = &report.warnings[0];
    assert_eq!(warning.code, "inference.mirror_failed");
    assert_eq!(
        warning.target,
        crate::KeyformTarget::Mesh("Dress/SkirtR".into())
    );
    assert_eq!(
        warning.source,
        crate::KeyformTarget::Mesh("Dress/Skirt".into())
    );
    // ミラーのセルはミラーし直さず、付け替えで写し直した値になる (前の局所座標のままではない)。
    let right_after = &document
        .model()
        .meshes
        .iter()
        .find(|m| m.id == "Dress/SkirtR")
        .unwrap()
        .keyforms;
    assert_ne!(*right_after, right_before);

    // `keep_look` でなければ写し直さないので、警告はない。
    let mut document = self::document(&module);
    let report = document
        .apply_with_report(settings("Dress", "Hair", ModuleRequirements::default()))
        .unwrap();
    assert_eq!(report.warnings, vec![]);
}

/// `keep_look` の付け替えでミラー元が動かず、ミラー先だけが付け替わって対が壊れたときも警告を返す。
/// 名前を同時に変えても、前のモデルと突き合わせて確かめる (T1621)。
#[test]
fn module_settings_keep_look_checks_reparented_pairs_when_renamed() {
    use ugoe_model::deformer::DeformerKind;

    // `Torso` を台形 (下が広い) にして、写像を双線形の非線形なものにする。`x = 128` で左右対称。
    let mut base = base();
    let torso = base.model.deformers.iter_mut().find(|d| d.id == "Torso");
    let DeformerKind::Warp(torso) = &mut torso.unwrap().kind else {
        unreachable!()
    };
    torso.keyforms.keyforms[0].points = vec![
        Vec2::new(64.0, 128.0),
        Vec2::new(192.0, 128.0),
        Vec2::new(0.0, 256.0),
        Vec2::new(256.0, 256.0),
    ];
    // 最上位のワープ `Frame` (`Torso` の UV で台形) の中に `Skirt`、その左右の反転の `SkirtR` は最上位に置く。
    // 付け替えでは `Frame` の制御点と `SkirtR` の頂点が写し直されるが、`Frame` の中の `Skirt` は双線形の
    // 補間が変わるので見た目が数ピクセルずれ、`SkirtR` と対応しなくなる (`Skirt` は変わらないので
    // ミラーし直さず、付け替えの確かめだけが見つける)。
    let mut module = dress_module();
    module.requires = ModuleRequirements::default();
    let frame_points = [
        Vec2::new(0.3, 0.5),
        Vec2::new(0.7, 0.5),
        Vec2::new(0.0, 1.0),
        Vec2::new(1.0, 1.0),
    ];
    let mut frame = dress_up_base()
        .deformers
        .into_iter()
        .find(|d| d.id == "Torso")
        .unwrap();
    frame.id = "Frame".into();
    frame.name = "Frame".into();
    frame.part = "Dress".into();
    let DeformerKind::Warp(warp) = &mut frame.kind else {
        unreachable!()
    };
    warp.keyforms.keyforms[0].points = frame_points.to_vec();
    module.deformers.push(frame);
    let mut rest = module.meshes[0].keyforms.keyforms[0].clone();
    rest.positions = [(0.1, 0.2), (0.4, 0.2), (0.4, 0.8), (0.1, 0.8)]
        .map(|(x, y)| Vec2::new(x, y))
        .to_vec();
    let skirt = &mut module.meshes[0];
    skirt.parent = Some("Frame".into());
    skirt.mirror = Some("SkirtR".into());
    skirt.keyforms = KeyformGrid::constant(rest.clone());
    let mut right = skirt.clone();
    right.id = "SkirtR".into();
    right.parent = None;
    right.mirror = Some("Skirt".into());
    right.keyforms = KeyformGrid::constant(rest);
    right.keyforms.origins = vec![ugoe_model::KeyformOrigin::Mirror];
    right.keyforms.mirror_source = Some(MirrorSource {
        id: "Skirt".into(),
        axis: None,
        tolerance: None,
    });
    module.meshes.push(right);
    let document_of = |module: &Module| {
        let context = ModuleContext::new(
            base.clone(),
            &module.slot,
            module.name.clone(),
            module.requires.clone(),
        )
        .unwrap();
        let workspace = context.workspace(module, &images(module)).unwrap();
        Document::with_textures(workspace.model, workspace.textures)
            .with_physics(workspace.physics)
            .with_module(Arc::new(context))
    };
    // `SkirtR` の頂点を、`Skirt` のキャンバスでの位置を `x = 128` (キャンバスの原点。親が異なる対の既定の
    // 対称軸) で反転した位置にする。
    let model = document_of(&module).model().clone();
    let mesh = |id: &str| crate::KeyformTarget::Mesh(id.into());
    let local = |id: &str| -> Vec<Vec2> {
        let mesh = model.meshes.iter().find(|m| m.id == id).unwrap();
        mesh.keyforms.keyforms[0].positions.clone()
    };
    let values = Default::default();
    let canvas =
        crate::space::local_to_canvas(&model, &mesh("Dress/Skirt"), &values, &local("Dress/Skirt"))
            .unwrap();
    let reflected: Vec<Vec2> = canvas.iter().map(|p| Vec2::new(256.0 - p.x, p.y)).collect();
    let guesses = local("Dress/SkirtR");
    module.meshes[1].keyforms.keyforms[0].positions =
        crate::space::canvas_to_local(&model, &mesh("Dress/SkirtR"), &values, &reflected, &guesses)
            .unwrap();

    for name in ["Dress", "Robe"] {
        let mut document = document_of(&module);
        // 付け替えの前は対応している。
        let mirror = Command::MirrorKeyforms {
            source: crate::KeyformTarget::Mesh("Dress/Skirt".into()),
            target: crate::KeyformTarget::Mesh("Dress/SkirtR".into()),
            axis: None,
            tolerance: None,
            overwrite_authored: false,
        };
        document.apply(mirror).unwrap();
        assert!(document.undo());

        let mut command = settings(name, "Hair", ModuleRequirements::default());
        let Command::SetModuleSettings { keep_look, .. } = &mut command else {
            unreachable!()
        };
        *keep_look = true;
        let report = document.apply_with_report(command).unwrap();
        assert_eq!(document.module().unwrap().name(), name);
        assert_eq!(
            report
                .warnings
                .iter()
                .map(|w| (w.code.as_str(), &w.target, &w.source))
                .collect::<Vec<_>>(),
            vec![(
                "inference.mirror_failed",
                &crate::KeyformTarget::Mesh(format!("{name}/SkirtR").as_str().into()),
                &crate::KeyformTarget::Mesh(format!("{name}/Skirt").as_str().into()),
            )],
            "name: {name}"
        );
    }
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
        let index = |id: &str| model.deformers.iter().position(|d| d.id == id).unwrap();
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
