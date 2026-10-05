//! 作業用のモデルを合成し直したあとに、消えた要素への参照が残らないことのテスト (T1602)。

use super::*;

/// 合成し直す前の作業用のモデルと物理 (`before`)。
type Workspaces = (
    ugoe_model::Model,
    Option<ugoe_model::physics::PhysicsSettings>,
);

fn workspaces(document: &Document) -> Workspaces {
    (document.model().clone(), document.physics().cloned())
}

/// `before` の要素のうち、今の `document` のモデルにない (合成し直して消えた) パーツ・デフォーマ・メッシュの ID。
fn removed_ids(before: &Workspaces, document: &Document) -> Vec<String> {
    let model = document.model();
    let mut removed: Vec<String> = before
        .0
        .parts
        .iter()
        .filter(|p| !model.parts.iter().any(|q| q.id == p.id))
        .map(|p| p.id.to_string())
        .collect();
    removed.extend(
        before
            .0
            .deformers
            .iter()
            .filter(|d| !model.deformers.iter().any(|e| e.id == d.id))
            .map(|d| d.id.to_string()),
    );
    removed.extend(
        before
            .0
            .meshes
            .iter()
            .filter(|m| !model.meshes.iter().any(|n| n.id == m.id))
            .map(|m| m.id.to_string()),
    );
    removed
}

/// 合成し直した `document` に、`before` から消えた要素への参照が残っていない: 適用の後の後始末
/// (`cleanup::forget_removed`) を通しても何も変わらず、モデルと物理の検証を通る (`docs/editor.md#削除の連鎖と拒否`)。
fn assert_no_references_to_removed(before: &Workspaces, document: &Document) {
    let mut model = document.model().clone();
    let mut physics = document.physics().cloned();
    crate::cleanup::forget_removed(&before.0, before.1.as_ref(), &mut model, &mut physics);
    assert_eq!(&model, document.model());
    assert_eq!(physics.as_ref(), document.physics());
    assert_eq!(ugoe_model::validate::validate(document.model()), vec![]);
    if let Some(physics) = document.physics() {
        assert_eq!(
            ugoe_model::validate::validate_physics(physics, document.model()),
            vec![]
        );
    }
}

/// LongHair を、スロット `Outfit` に移せる形 (`Face` をマスクに使わない) にし、ミラー・ポーズ・物理の参照を持たせたもの。
fn movable_long_hair() -> Module {
    use ugoe_model::pose::{PoseEntry, PoseGroup};
    let mut module = long_hair_module();
    module.meshes[0].masks.clear();
    module.meshes[0].mirror = Some("Front".into());
    module.meshes[1].mirror = Some("Back".into());
    module.parts.push(Part {
        id: "HairAlt".into(),
        name: "Hair Alt".into(),
        parent: Some("Hair".into()),
        keyforms: KeyformGrid::constant(ugoe_model::part::PartKeyform { opacity: 1.0 }),
    });
    module.pose.groups = vec![PoseGroup {
        entries: ["Hair", "HairAlt"]
            .map(|part| PoseEntry {
                part: part.into(),
                links: Vec::new(),
            })
            .into(),
    }];
    module
}

/// `set_module_settings` は作業用のモデルを合成し直すので、名前を変えるとモジュールの要素の ID がすべて替わり、
/// スロットを変えると前のスロットが置き換えるパーツを隠すパーツ (`#replaced/…`) が消える。どちらでも、消えた要素への
/// 参照は合成が揃えるので残らない (T1602)。
#[test]
fn module_settings_leave_no_references_to_removed_elements() {
    let module = movable_long_hair();

    // 名前を変える: `LongHair/…` の要素 (物理のリグ・衝突体を含む) が消え、`Braid/…` になる。
    let mut document = document(&module);
    let before = workspaces(&document);
    document
        .apply(settings("Braid", "Hair", module.requires.clone()))
        .unwrap();
    let removed = removed_ids(&before, &document);
    assert!(removed.contains(&"LongHair/Back".to_owned()), "{removed:?}");
    assert!(removed.contains(&"LongHair/Sway".to_owned()), "{removed:?}");
    assert_no_references_to_removed(&before, &document);
    let physics = serde_json::to_string(document.physics().unwrap()).unwrap();
    assert!(!physics.contains("LongHair/"), "{physics}");
    let model = serde_json::to_string(document.model()).unwrap();
    assert!(!model.contains("LongHair/"), "{model}");

    // スロットを変える (取り付け先も Head から Torso に替わる): `Hair` を隠すパーツが消える。
    for keep_look in [false, true] {
        let mut document = self::document(&module);
        let before = workspaces(&document);
        let mut command = settings("LongHair", "Outfit", module.requires.clone());
        let Command::SetModuleSettings { keep_look: k, .. } = &mut command else {
            unreachable!();
        };
        *k = keep_look;
        document.apply(command).unwrap();
        assert_eq!(document.module().unwrap().slot().id.as_str(), "Outfit");
        let removed = removed_ids(&before, &document);
        assert_eq!(removed, ["#replaced/Hair"], "keep_look: {keep_look}");
        assert_no_references_to_removed(&before, &document);
        let hair = document
            .model()
            .parts
            .iter()
            .find(|p| p.id == "Hair")
            .unwrap();
        assert_eq!(hair.parent.as_ref().map(|p| p.as_str()), Some("Root"));
    }
}

/// `rebase_module` も作業用のモデルを合成し直す。本体から要素が消えても、モジュールが参照していなければ参照は
/// 残らない。モジュールが参照する本体の要素 (マスクの元・衝突体) が消えたら、合成できないので何も変えずにエラー (T1602)。
#[test]
fn rebase_leaves_no_references_to_removed_elements() {
    use ugoe_model::physics::{ColliderShape, PhysicsCollider};
    use ugoe_runtime::ComposeError;
    let shoulders = PhysicsCollider {
        id: "Shoulders".into(),
        name: "Shoulders".into(),
        attach: Some("Torso".into()),
        shape: ColliderShape::Circle {
            center: Vec2::new(0.0, 0.0),
            radius: 4.0,
        },
    };
    let mut base = base();
    let mut physics = crate::physics_edit::default_settings();
    physics.colliders.push(shoulders);
    base.physics = Some(physics);
    let hair = base
        .model
        .slots
        .iter_mut()
        .find(|s| s.id == "Hair")
        .unwrap();
    hair.colliders = vec!["Shoulders".into()];
    let mut module = long_hair_module();
    module.physics.rigs[0].colliders.push("Shoulders".into());
    let open = |base: &ModuleBase| {
        let context = Arc::new(
            ModuleContext::new(
                base.clone(),
                &module.slot,
                module.name.clone(),
                module.requires.clone(),
            )
            .unwrap(),
        );
        let workspace = context.workspace(&module, &images(&module)).unwrap();
        Document::with_textures(workspace.model, workspace.textures)
            .with_physics(workspace.physics)
            .with_module(context)
    };

    // 本体のスロットが `Hair` を置き換えなくなり、本体のメッシュ `HairDefault` も消えた: どちらもモジュールは参照しない。
    let mut changed = base.clone();
    changed.model.slots[0].replaces.clear();
    changed.model.meshes.retain(|m| m.id != "HairDefault");
    let mut document = open(&base);
    let before = workspaces(&document);
    document.rebase_module(changed.clone()).unwrap();
    let removed = removed_ids(&before, &document);
    assert_eq!(removed, ["#replaced/Hair", "HairDefault"]);
    assert_no_references_to_removed(&before, &document);

    // 別のスロットに付け替えて読み直す (`rebase_module_on`): `LongHair/…` の要素は残り、隠すパーツが消える。
    let mut movable = movable_long_hair();
    movable.physics = Default::default();
    let context = Arc::new(context(&movable));
    let workspace = context.workspace(&movable, &images(&movable)).unwrap();
    let mut document = Document::with_textures(workspace.model, workspace.textures)
        .with_physics(workspace.physics)
        .with_module(context);
    let before = workspaces(&document);
    document
        .rebase_module_on(self::base(), Some(&"Outfit".into()))
        .unwrap();
    assert_eq!(removed_ids(&before, &document), ["#replaced/Hair"]);
    assert_no_references_to_removed(&before, &document);

    // モジュールが参照する本体の要素が消えると、合成できないので何も変えない。
    let mut document = open(&base);
    let before = workspaces(&document);
    let state = document.state_id();
    let mut no_face = base.clone();
    no_face.model.meshes.retain(|m| m.id != "Face");
    no_face.model.slots[0].mask_sources.clear();
    let error = document.rebase_module(no_face).unwrap_err();
    assert!(
        matches!(
            error,
            ModuleError::Compose(ComposeError::UnknownMask { .. })
        ),
        "{error:?}"
    );
    let mut no_collider = base.clone();
    no_collider.physics.as_mut().unwrap().colliders.clear();
    no_collider.model.slots[0].colliders.clear();
    let error = document.rebase_module(no_collider).unwrap_err();
    assert!(
        matches!(
            error,
            ModuleError::Compose(ComposeError::UnknownCollider { .. })
        ),
        "{error:?}"
    );
    assert_eq!(workspaces(&document), before);
    assert_eq!(document.state_id(), state);
}
