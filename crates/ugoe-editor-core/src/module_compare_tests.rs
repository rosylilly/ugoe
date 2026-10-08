//! 今の編集と書き出した `.ugoemod` の違い (`crate::module_compare`) と、アトラス化を選べるモジュールの書き出し
//! (`crate::export::export_module_with_settings`) のテスト (T0654)。

use std::sync::Arc;

use ugoe_core::Vec2;
use ugoe_model::fixtures::{dress_up_base, long_hair_module};
use ugoe_model::mesh::{GlueKeyform, GluePair};
use ugoe_model::module::ModuleRequirements;
use ugoe_model::physics::ColliderShape;
use ugoe_model::pose::{PoseEntry, PoseGroup};
use ugoe_model::{Glue, KeyformGrid, Module, Part, TextureRef};

use crate::module_compare::{
    ChangedItems, CountedItems, DifferenceKind, DifferenceLine, ElementChange, ItemState,
    KeyformValue, ModuleDifference, NameList,
};
use crate::module_project::{ModuleBase, ModuleContext};
use crate::texture::{Textures, encode_rgba_png, solid_png};
use crate::{
    Command, Document, ExportSettings, Project, ProjectError, ProjectMeta,
    compare_with_module_file, export_module, export_module_with_settings,
};

/// 本体 (`dress_up_base`) の Hair に LongHair を付けたモジュールのプロジェクト。
fn project() -> Project {
    let module = long_hair_module();
    let images: Textures = module
        .textures
        .iter()
        .map(|t| (t.name.clone(), Arc::from(solid_png(2, 2, [1, 2, 3, 255]))))
        .collect();
    project_of(&module, &images)
}

/// `module` と画像 `images` のモジュールのプロジェクト (本体は `dress_up_base`)。
fn project_of(module: &Module, images: &Textures) -> Project {
    project_on(dress_up_base(), module, images)
}

/// 本体 `model` に `module` を付けたモジュールのプロジェクト。
fn project_on(model: ugoe_model::Model, module: &Module, images: &Textures) -> Project {
    let textures = model
        .textures
        .iter()
        .map(|t| (t.name.clone(), Arc::from(solid_png(4, 4, [255; 4]))))
        .collect();
    let base = ModuleBase {
        path: "/base.ugoeproj".into(),
        model,
        physics: None,
        textures,
    };
    let context = Arc::new(
        ModuleContext::new(
            base,
            &module.slot,
            module.name.clone(),
            module.requires.clone(),
        )
        .unwrap(),
    );
    let workspace = context.workspace(module, images).unwrap();
    let document = Document::with_textures(workspace.model, workspace.textures)
        .with_physics(workspace.physics)
        .with_module(context);
    Project::from_document(ProjectMeta::new("LongHair"), &document)
}

fn edited(project: &Project, command: Command) -> Project {
    let mut document = project.document();
    document.apply(command).unwrap();
    Project::from_document(project.meta.clone(), &document)
}

#[test]
fn same_export_has_no_differences() {
    let project = project();
    let bytes = export_module(&project).unwrap().bytes;
    assert_eq!(compare_with_module_file(&project, &bytes).unwrap(), vec![]);
}

/// メッシュを除き、条件を変え、画像を描き替えると、それぞれの行が出る (ファイル → 今の編集の向き)。
#[test]
fn differences_list_elements_textures_and_requirements() {
    let project = project();
    let bytes = export_module(&project).unwrap().bytes;
    let document = project.document();
    let mesh = document
        .model()
        .meshes
        .iter()
        .find(|m| !document.module().unwrap().is_base_mesh(&m.id))
        .unwrap()
        .clone();
    let texture = document.model().textures[mesh.texture as usize]
        .name
        .clone();
    let requires = ModuleRequirements {
        slots: vec!["Outfit".into()],
        ..ModuleRequirements::default()
    };
    let mut changed = edited(
        &project,
        Command::SetModuleSettings {
            name: "LongHair".into(),
            slot: "Hair".into(),
            requires,
            keep_look: false,
        },
    );
    changed
        .textures
        .insert(texture.clone(), Arc::from(solid_png(2, 2, [9, 9, 9, 255])));
    let lines = lines(&compare_with_module_file(&changed, &bytes).unwrap());
    assert!(
        lines.contains(&DifferenceLine::Names {
            what: NameList::RequiredSlots,
            added: strings(&["Outfit"]),
            removed: Vec::new(),
        }),
        "{lines:?}"
    );
    assert!(
        lines.contains(&DifferenceLine::Textures {
            before: 1,
            after: 1,
            added: Vec::new(),
            removed: Vec::new(),
            changed: vec![long_hair_module().textures[0].name.clone()],
        }),
        "{lines:?}"
    );

    let removed = edited(
        &project,
        Command::RemoveMesh {
            id: mesh.id.clone(),
        },
    );
    let differences = compare_with_module_file(&removed, &bytes).unwrap();
    let id = mesh.id.as_str().trim_start_matches("LongHair/");
    let DifferenceLine::Elements { removed, .. } = &find(&differences, DifferenceKind::Mesh).line
    else {
        unreachable!();
    };
    assert!(removed.iter().any(|r| r == id), "{differences:?}");
}

#[test]
fn broken_files_are_errors() {
    // 読めないのは読み込みの失敗で、書き出しの失敗 (`project.export_failed`) と区別する (T0678)。
    let error = compare_with_module_file(&project(), b"not a module").unwrap_err();
    assert!(
        matches!(error, ProjectError::UnreadableModuleFile(_)),
        "{error:?}"
    );
    let message = crate::Coded::to_message(&error);
    assert_eq!(message.code, "project.module_file_unreadable");
    assert!(message.cause.is_some());
}

/// アトラス化すると、テクスチャが `atlas-0` にまとまり、比べるとその旨が出る。しなければ `export_module` と同じ。
#[test]
fn modules_can_be_exported_with_an_atlas() {
    let project = project();
    let plain = export_module_with_settings(&project, &ExportSettings::default()).unwrap();
    assert_eq!(plain.bytes, export_module(&project).unwrap().bytes);
    assert_eq!(plain.atlas, None);

    let settings = ExportSettings {
        atlas: true,
        ..ExportSettings::default()
    };
    let atlased = export_module_with_settings(&project, &settings).unwrap();
    let report = atlased.atlas.unwrap();
    assert_eq!(report.packed, 1);
    let file = ugoe_format::read_module(&atlased.bytes).unwrap();
    assert_eq!(file.module.textures.len(), 1);
    assert_eq!(file.module.textures[0].name, "atlas-0");

    // アトラスの中の場所を見つけて UV を戻すので、今の編集と同じ (T0656)。
    assert_eq!(
        compare_with_module_file(&project, &atlased.bytes).unwrap(),
        vec![]
    );

    let invalid = ExportSettings {
        atlas: true,
        atlas_max_size: 0,
        ..ExportSettings::default()
    };
    assert!(export_module_with_settings(&project, &invalid).is_err());
}

fn lines(differences: &[ModuleDifference]) -> Vec<DifferenceLine> {
    differences.iter().map(|d| d.line.clone()).collect()
}

fn strings(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|&id| id.to_owned()).collect()
}

/// 要素 `what` の行 (数 `before` → `after` と、足した・除いた・変えた ID)。
fn elements(
    what: DifferenceKind,
    (before, after): (usize, usize),
    added: &[&str],
    removed: &[&str],
    changed: &[&str],
) -> DifferenceLine {
    DifferenceLine::Elements {
        what,
        before,
        after,
        added: strings(added),
        removed: strings(removed),
        changed: strings(changed),
    }
}

/// 要素 `what` の行。
fn find(differences: &[ModuleDifference], what: DifferenceKind) -> &ModuleDifference {
    differences
        .iter()
        .find(|d| matches!(d.line, DifferenceLine::Elements { what: w, .. } if w == what))
        .unwrap_or_else(|| panic!("{differences:?}"))
}

fn changed_count(what: ChangedItems, count: usize) -> ElementChange {
    ElementChange::Changed { what, count }
}

fn in_keyforms(what: KeyformValue, count: usize) -> ElementChange {
    ElementChange::InKeyforms { what, count }
}

/// 画素がどこも違う (探す場所を 1 つに決められる) `width`×`height` の画像。
fn pattern_png(width: u32, height: u32, seed: u8) -> Arc<[u8]> {
    let rgba: Vec<u8> = (0..width * height)
        .flat_map(|i| {
            let v = u8::try_from(i % 251).unwrap();
            [v, seed, v.wrapping_mul(7), 255]
        })
        .collect();
    Arc::from(encode_rgba_png(width, height, &rgba))
}

/// LongHair の `Front` を 2 つ目のテクスチャ `front` に分け、どちらにも模様のある画像を持たせたプロジェクト。
fn two_texture_project() -> Project {
    let mut module = long_hair_module();
    module.textures[0].width = 9;
    module.textures[0].height = 7;
    module.textures.push(TextureRef {
        name: "front".into(),
        width: 6,
        height: 5,
        normal_map: None,
    });
    let front = module.meshes.iter_mut().find(|m| m.id == "Front").unwrap();
    front.texture = 1;
    let images: Textures = [
        ("long_hair".to_owned(), pattern_png(9, 7, 1)),
        ("front".to_owned(), pattern_png(6, 5, 2)),
    ]
    .into_iter()
    .collect();
    project_of(&module, &images)
}

/// `front` のテクスチャの画像を描き替える (作業用のモデルでの名前で)。
fn repaint(project: &mut Project, png: Arc<[u8]>) {
    let name = project
        .model
        .textures
        .iter()
        .map(|t| t.name.clone())
        .find(|n| n.ends_with("front"))
        .unwrap();
    project.textures.insert(name, png);
}

fn atlased(project: &Project) -> Vec<u8> {
    let settings = ExportSettings {
        atlas: true,
        ..ExportSettings::default()
    };
    let export = export_module_with_settings(project, &settings).unwrap();
    assert_eq!(export.atlas.unwrap().packed, 2);
    export.bytes
}

fn mesh_mut<'a>(project: &'a mut Project, id: &str) -> &'a mut ugoe_model::Mesh {
    project
        .model
        .meshes
        .iter_mut()
        .find(|m| m.id == id)
        .unwrap()
}

/// 変えたメッシュには、何が違うか (頂点・キーフォーム・描画順) と、ツリーで選ぶ作業用のモデルの ID が付く (T0656)。
#[test]
fn changed_meshes_tell_what_changed_and_where_they_are() {
    let project = project();
    let bytes = export_module(&project).unwrap().bytes;
    let mut changed = project.clone();
    let front = mesh_mut(&mut changed, "LongHair/Front");
    let before = front.keyforms.keyforms[0].draw_order;
    for keyform in front.keyforms.keyforms.iter_mut() {
        keyform.draw_order -= 10.0;
        keyform.positions[0].x += 1.0;
    }
    front.uvs[1] = Vec2::new(0.5, 0.5);
    let differences = compare_with_module_file(&changed, &bytes).unwrap();
    let meshes = find(&differences, DifferenceKind::Mesh);
    assert_eq!(
        meshes.line,
        elements(DifferenceKind::Mesh, (2, 2), &[], &[], &["Front"])
    );
    let [item] = &meshes.items[..] else {
        panic!("{meshes:?}");
    };
    assert_eq!(item.kind, DifferenceKind::Mesh);
    assert_eq!(item.id, "Front");
    assert_eq!(item.state, ItemState::Changed);
    assert_eq!(item.element.as_deref(), Some("LongHair/Front"));
    // 描画順は、ツリーやインスペクタと同じスロットの範囲の値で書く。
    assert_eq!(
        item.changes,
        [
            changed_count(ChangedItems::Uvs, 1),
            in_keyforms(KeyformValue::VertexPositions, 1),
            ElementChange::DrawOrder {
                before: f64::from(before),
                after: f64::from(before - 10.0),
            },
        ]
    );

    // 頂点の数が変われば、数で書く。本体の要素は選ばない。
    let mut changed = project.clone();
    let front = mesh_mut(&mut changed, "LongHair/Front");
    front.uvs.push(Vec2::new(0.5, 0.5));
    for keyform in front.keyforms.keyforms.iter_mut() {
        keyform.positions.push(Vec2::ZERO);
    }
    let differences = compare_with_module_file(&changed, &bytes).unwrap();
    let item = &find(&differences, DifferenceKind::Mesh).items[0];
    assert_eq!(
        item.changes,
        [ElementChange::Count {
            what: CountedItems::Vertices,
            before: 4,
            after: 5,
        }]
    );
}

/// 足した要素も選べる。除いた要素も添えるが、今の編集に無いので選べない (ファイルを付けたプレビューで強調する)。
#[test]
fn added_elements_can_be_selected() {
    let project = project();
    let bytes = export_module(&project).unwrap().bytes;
    let mut removed = project.document();
    removed
        .apply(Command::RemoveMesh {
            id: "LongHair/Back".into(),
        })
        .unwrap();
    let removed = Project::from_document(project.meta.clone(), &removed);
    // Back の無いファイルと比べると、今の編集では Back を足したことになる。
    let without_back = export_module(&removed).unwrap().bytes;
    let differences = compare_with_module_file(&project, &without_back).unwrap();
    let meshes = find(&differences, DifferenceKind::Mesh);
    // Front は Back でマスクしているので、Front も変わる。
    assert_eq!(
        meshes.line,
        elements(DifferenceKind::Mesh, (1, 2), &["Back"], &[], &["Front"])
    );
    assert_eq!(meshes.items[0].id, "Back");
    assert_eq!(meshes.items[0].state, ItemState::Added);
    assert!(meshes.items[0].changes.is_empty());
    assert_eq!(meshes.items[0].element.as_deref(), Some("LongHair/Back"));
    assert_eq!(meshes.items[1].changes, [ElementChange::BlendingOrMasks]);
    let differences = compare_with_module_file(&removed, &bytes).unwrap();
    let meshes = find(&differences, DifferenceKind::Mesh);
    assert_eq!(
        meshes.line,
        elements(DifferenceKind::Mesh, (2, 1), &[], &["Back"], &["Front"])
    );
    assert_eq!(meshes.items.len(), 2);
    assert_eq!(meshes.items[1].id, "Back");
    assert_eq!(meshes.items[1].state, ItemState::Removed);
    assert_eq!(meshes.items[1].element, None);
    // ファイルのメッシュなので、プレビューでファイルを付けたときに強調できる。
    assert_eq!(meshes.items[1].meshes, ["Back"]);
}

/// 物理のリグと衝突体も、何が違うかと、Physics タブで選ぶ作業用の物理の ID を添える。強調するメッシュは、
/// リグなら出力先のデフォーマの下のメッシュ (T0657)。
#[test]
fn physics_differences_tell_what_changed() {
    let project = project();
    let bytes = export_module(&project).unwrap().bytes;
    let mut changed = project.clone();
    let physics = changed.physics.as_mut().unwrap();
    let rig = physics
        .rigs
        .iter_mut()
        .find(|r| r.id == "LongHair/Sway")
        .unwrap();
    rig.particles[1].mobility = 0.5;
    rig.colliders.clear();
    let collider = physics
        .colliders
        .iter_mut()
        .find(|c| c.id == "LongHair/Collar")
        .unwrap();
    collider.shape = ColliderShape::Circle {
        center: Vec2::new(0.0, 40.0),
        radius: 12.0,
    };
    let differences = compare_with_module_file(&changed, &bytes).unwrap();
    let rigs = find(&differences, DifferenceKind::PhysicsRig);
    assert_eq!(
        rigs.line,
        elements(DifferenceKind::PhysicsRig, (1, 1), &[], &[], &["Sway"])
    );
    let [rig] = &rigs.items[..] else {
        panic!("{rigs:?}");
    };
    assert_eq!(rig.kind, DifferenceKind::PhysicsRig);
    assert_eq!(
        rig.changes,
        [
            changed_count(ChangedItems::Particles, 1),
            ElementChange::Colliders {
                added: Vec::new(),
                removed: strings(&["Collar"]),
            },
        ]
    );
    assert_eq!(rig.element.as_deref(), Some("LongHair/Sway"));
    assert_eq!(rig.meshes, ["Back", "Front"]);
    let colliders = find(&differences, DifferenceKind::Collider);
    let [collider] = &colliders.items[..] else {
        panic!("{colliders:?}");
    };
    assert_eq!(collider.kind, DifferenceKind::Collider);
    assert_eq!(
        collider.changes,
        [ElementChange::Radius {
            before: 8.0,
            after: 12.0,
        }]
    );
    assert_eq!(collider.element.as_deref(), Some("LongHair/Collar"));
    assert!(collider.meshes.is_empty());

    // デフォーマ・パーツは、その下のメッシュを強調する。
    let mut changed = project.clone();
    let sway = changed
        .model
        .deformers
        .iter_mut()
        .find(|d| d.id == "LongHair/Sway")
        .unwrap();
    sway.name = "Swing".into();
    let differences = compare_with_module_file(&changed, &bytes).unwrap();
    let deformers = &differences[0];
    assert_eq!(
        deformers.line,
        elements(DifferenceKind::Deformer, (1, 1), &[], &[], &["Sway"])
    );
    assert_eq!(
        deformers.items[0].changes,
        [ElementChange::Name {
            before: "Sway".into(),
            after: "Swing".into(),
        }]
    );
    assert_eq!(deformers.items[0].meshes, ["Back", "Front"]);
}

/// グルーは両端のメッシュを強調する。除いたグルーはファイルの両端を強調し、選べない (T0657)。
#[test]
fn glue_differences_tell_what_changed() {
    let mut module = long_hair_module();
    module.glues.push(Glue {
        id: "Seam".into(),
        mesh_a: "Back".into(),
        mesh_b: "Front".into(),
        pairs: vec![GluePair {
            a: 0,
            b: 0,
            weight: 0.5,
        }],
        keyforms: KeyformGrid::constant(GlueKeyform { intensity: 1.0 }),
    });
    let images: Textures = module
        .textures
        .iter()
        .map(|t| (t.name.clone(), Arc::from(solid_png(2, 2, [1, 2, 3, 255]))))
        .collect();
    let project = project_of(&module, &images);
    let bytes = export_module(&project).unwrap().bytes;
    let mut changed = project.clone();
    let glue = &mut changed.model.glues[0];
    assert_eq!(glue.id.as_str(), "LongHair/Seam");
    glue.pairs[0].weight = 0.25;
    glue.keyforms.keyforms[0].intensity = 0.5;
    let differences = compare_with_module_file(&changed, &bytes).unwrap();
    assert_eq!(
        lines(&differences),
        [elements(DifferenceKind::Glue, (1, 1), &[], &[], &["Seam"])]
    );
    let item = &differences[0].items[0];
    assert_eq!(item.kind, DifferenceKind::Glue);
    assert_eq!(
        item.changes,
        [
            changed_count(ChangedItems::VertexPairs, 1),
            in_keyforms(KeyformValue::Intensity, 1),
        ]
    );
    assert_eq!(item.element.as_deref(), Some("LongHair/Seam"));
    assert_eq!(item.meshes, ["Back", "Front"]);

    let mut removed = project.clone();
    removed.model.glues.clear();
    let differences = compare_with_module_file(&removed, &bytes).unwrap();
    assert_eq!(
        lines(&differences),
        [elements(DifferenceKind::Glue, (1, 0), &[], &["Seam"], &[])]
    );
    let item = &differences[0].items[0];
    assert_eq!(
        (item.state, item.element.as_deref()),
        (ItemState::Removed, None)
    );
    assert_eq!(item.meshes, ["Back", "Front"]);
}

/// 本体にポーズのグループ `ArmA / ArmB` (スロット Hair が `ArmA` を公開) を足し、LongHair にグループ
/// `HatA / HatB` (Back と Front) と、`ArmA` に追従する `Sleeve` を足したプロジェクト。
fn pose_project() -> Project {
    let entry = |part: &str, links: &[&str]| PoseEntry {
        part: part.into(),
        links: links.iter().map(|&l| l.into()).collect(),
    };
    let mut base = dress_up_base();
    let template = base.parts[0].clone();
    for id in ["ArmA", "ArmB"] {
        base.parts.push(Part {
            id: id.into(),
            name: id.into(),
            parent: None,
            ..template.clone()
        });
    }
    base.pose.groups = vec![PoseGroup {
        entries: vec![entry("ArmA", &[]), entry("ArmB", &[])],
    }];
    base.slots[0].pose_parts = vec!["ArmA".into()];

    let mut module = long_hair_module();
    let template = module.parts[0].clone();
    for id in ["HatA", "HatB", "Sleeve"] {
        module.parts.push(Part {
            id: id.into(),
            name: id.into(),
            parent: Some("Hair".into()),
            ..template.clone()
        });
    }
    module.meshes[0].part = "HatA".into();
    module.meshes[1].part = "HatB".into();
    module.pose.groups = vec![PoseGroup {
        entries: vec![entry("HatA", &[]), entry("HatB", &[])],
    }];
    module.pose.links = vec![entry("ArmA", &["Sleeve"])];
    let images: Textures = module
        .textures
        .iter()
        .map(|t| (t.name.clone(), Arc::from(solid_png(2, 2, [1, 2, 3, 255]))))
        .collect();
    project_on(base, &module, &images)
}

/// ポーズのグループと追従の違いは、要素のパーツまで出し、パーツで選び、その下のメッシュを強調する (T0659)。
#[test]
fn pose_differences_tell_what_changed() {
    let project = pose_project();
    let bytes = export_module(&project).unwrap().bytes;
    assert_eq!(compare_with_module_file(&project, &bytes).unwrap(), vec![]);

    // グループの並びを入れ替え、追従を外す。
    let mut changed = project.clone();
    let pose = &mut changed.model.pose;
    pose.groups[1].entries.reverse();
    pose.groups[0].entries[0].links.clear();
    let differences = compare_with_module_file(&changed, &bytes).unwrap();
    assert_eq!(
        lines(&differences),
        [
            elements(
                DifferenceKind::PoseGroup,
                (1, 1),
                &[],
                &[],
                &["HatB / HatA"]
            ),
            elements(DifferenceKind::PoseFollow, (1, 0), &[], &["ArmA"], &[]),
        ]
    );
    let group = &differences[0].items[0];
    assert_eq!(group.kind, DifferenceKind::PoseGroup);
    assert_eq!(
        group.changes,
        [ElementChange::Entries {
            before: "HatA / HatB".into(),
            after: "HatB / HatA".into(),
        }]
    );
    assert_eq!(group.element.as_deref(), Some("HatB / HatA"));
    assert_eq!(group.parts, ["LongHair/HatB", "LongHair/HatA"]);
    assert_eq!(group.meshes, ["Front", "Back"]);
    let follow = &differences[1].items[0];
    assert_eq!(follow.kind, DifferenceKind::PoseFollow);
    assert_eq!(
        (follow.state, follow.element.as_deref()),
        (ItemState::Removed, None)
    );
    assert!(follow.parts.is_empty());

    // リンクを変えた追従は、本体の要素とリンクのパーツで選ぶ。除いたグループは選べない。
    let mut relinked = project.clone();
    let pose = &mut relinked.model.pose;
    pose.groups.truncate(1);
    pose.groups[0].entries[0].links = vec!["LongHair/HatA".into()];
    let differences = compare_with_module_file(&relinked, &bytes).unwrap();
    assert_eq!(
        lines(&differences),
        [
            elements(
                DifferenceKind::PoseGroup,
                (1, 0),
                &[],
                &["HatA / HatB"],
                &[]
            ),
            elements(DifferenceKind::PoseFollow, (1, 1), &[], &[], &["ArmA"]),
        ]
    );
    let removed = &differences[0].items[0];
    assert_eq!(removed.element, None);
    assert_eq!(removed.meshes, ["Back", "Front"]);
    let follow = &differences[1].items[0];
    assert_eq!(
        follow.changes,
        [ElementChange::Links {
            entry: None,
            added: strings(&["HatA"]),
            removed: strings(&["Sleeve"]),
        }]
    );
    assert_eq!(follow.element.as_deref(), Some("ArmA"));
    assert_eq!(follow.parts, ["ArmA", "LongHair/HatA"]);
    assert_eq!(follow.meshes, ["Back"]);
}

/// アトラス化したファイルとは、今の編集の画像をアトラスの中で見つけ、UV を戻して比べる (T0656)。
#[test]
fn atlased_files_are_compared_before_packing() {
    let project = two_texture_project();
    let bytes = atlased(&project);
    assert_eq!(compare_with_module_file(&project, &bytes).unwrap(), vec![]);

    // UV を 1 つ動かすと、そのメッシュの UV だけが違う (テクスチャの違いやアトラスの注記は出ない)。
    let mut moved = project.clone();
    mesh_mut(&mut moved, "LongHair/Front").uvs[2] = Vec2::new(0.25, 0.75);
    let differences = compare_with_module_file(&moved, &bytes).unwrap();
    assert_eq!(
        lines(&differences),
        [elements(DifferenceKind::Mesh, (2, 2), &[], &[], &["Front"])]
    );
    assert_eq!(
        differences[0].items[0].changes,
        [changed_count(ChangedItems::Uvs, 1)]
    );

    // 画像を描き替えると、そのテクスチャの画像が違う。
    let mut repainted = project.clone();
    repaint(&mut repainted, pattern_png(6, 5, 3));
    assert_eq!(
        lines(&compare_with_module_file(&repainted, &bytes).unwrap()),
        [DifferenceLine::Textures {
            before: 2,
            after: 2,
            added: Vec::new(),
            removed: Vec::new(),
            changed: strings(&["front"]),
        }]
    );

    // メッシュを除いても、ファイルのそのメッシュのテクスチャを画素で見つける。
    let mut document = project.document();
    document
        .apply(Command::RemoveMesh {
            id: "LongHair/Back".into(),
        })
        .unwrap();
    let removed = Project::from_document(project.meta.clone(), &document);
    assert_eq!(
        lines(&compare_with_module_file(&removed, &bytes).unwrap()),
        [elements(
            DifferenceKind::Mesh,
            (2, 1),
            &[],
            &["Back"],
            &["Front"]
        )]
    );
}

/// 今の編集のテクスチャに戻せないメッシュがあれば、アトラスをテクスチャのまま比べ、その旨を添える。
#[test]
fn unmatched_atlas_meshes_are_noted() {
    let project = two_texture_project();
    let bytes = atlased(&project);
    // UV がどれも違っても (多数決で決まらなくても)、画像が同じなら画素で探して戻す。
    let mut changed = project.clone();
    let front = mesh_mut(&mut changed, "LongHair/Front");
    for uv in &mut front.uvs {
        *uv = Vec2::new(1.0 - uv.x, 1.0 - uv.y);
    }
    let differences = compare_with_module_file(&changed, &bytes).unwrap();
    assert_eq!(
        lines(&differences),
        [elements(DifferenceKind::Mesh, (2, 2), &[], &[], &["Front"])]
    );
    assert_eq!(
        differences[0].items[0].changes,
        [changed_count(ChangedItems::Uvs, 4)]
    );

    // 画像も UV も違えば、場所を確かめられない。
    repaint(&mut changed, pattern_png(6, 5, 4));
    let lines = lines(&compare_with_module_file(&changed, &bytes).unwrap());
    assert!(lines.contains(&DifferenceLine::UnmatchedAtlas), "{lines:?}");
    assert!(
        lines.iter().any(|l| matches!(
            l,
            DifferenceLine::Textures { removed, .. } if removed.iter().any(|r| r == "atlas-0")
        )),
        "{lines:?}"
    );
}
