//! 今の編集と書き出した `.ugoemod` の違い (`crate::module_compare`) と、アトラス化を選べるモジュールの書き出し
//! (`crate::export::export_module_with_settings`) のテスト (T0654)。

use std::sync::Arc;

use ugoe_core::Vec2;
use ugoe_model::fixtures::{dress_up_base, long_hair_module};
use ugoe_model::module::ModuleRequirements;
use ugoe_model::{Module, TextureRef};

use crate::module_compare::{DifferenceKind, ModuleDifference};
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
    let model = dress_up_base();
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
    let lines = texts(&compare_with_module_file(&changed, &bytes).unwrap());
    assert!(
        lines.contains(&"Required slots: added: Outfit".to_owned()),
        "{lines:?}"
    );
    assert!(
        lines.contains(&format!(
            "Textures: 1 → 1 (different image: {})",
            long_hair_module().textures[0].name
        )),
        "{lines:?}"
    );

    let removed = edited(
        &project,
        Command::RemoveMesh {
            id: mesh.id.clone(),
        },
    );
    let lines = texts(&compare_with_module_file(&removed, &bytes).unwrap());
    let id = mesh.id.as_str().trim_start_matches("LongHair/");
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("Meshes: ") && l.contains(&format!("removed: {id}"))),
        "{lines:?}"
    );
}

#[test]
fn broken_files_are_errors() {
    assert!(matches!(
        compare_with_module_file(&project(), b"not a module"),
        Err(ProjectError::Export(_))
    ));
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

fn texts(differences: &[ModuleDifference]) -> Vec<String> {
    differences.iter().map(|d| d.text.clone()).collect()
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
    let front = module
        .meshes
        .iter_mut()
        .find(|m| m.id.as_str() == "Front")
        .unwrap();
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
        .find(|m| m.id.as_str() == id)
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
    let meshes = differences
        .iter()
        .find(|d| d.text.starts_with("Meshes: "))
        .unwrap();
    assert_eq!(meshes.text, "Meshes: 2 → 2 (changed: Front)");
    let [item] = &meshes.items[..] else {
        panic!("{meshes:?}");
    };
    assert_eq!(item.kind, DifferenceKind::Mesh);
    assert_eq!(item.id, "Front");
    assert_eq!(item.element.as_deref(), Some("LongHair/Front"));
    // 描画順は、ツリーやインスペクタと同じスロットの範囲の値で書く。
    assert_eq!(
        item.text,
        format!(
            "UVs of 1 vertex; vertex positions in 1 keyform; draw order {before} → {}",
            before - 10.0
        )
    );

    // 頂点の数が変われば、数で書く。本体の要素は選ばない。
    let mut changed = project.clone();
    let front = mesh_mut(&mut changed, "LongHair/Front");
    front.uvs.push(Vec2::new(0.5, 0.5));
    for keyform in front.keyforms.keyforms.iter_mut() {
        keyform.positions.push(Vec2::ZERO);
    }
    let differences = compare_with_module_file(&changed, &bytes).unwrap();
    let item = &differences
        .iter()
        .find(|d| d.text.starts_with("Meshes: "))
        .unwrap()
        .items[0];
    assert_eq!(item.text, "vertices 4 → 5");
}

/// 足した要素も選べる。除いた要素は今の編集に無いので添えない。
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
    let meshes = differences
        .iter()
        .find(|d| d.text.starts_with("Meshes: "))
        .unwrap();
    // Front は Back でマスクしているので、Front も変わる。
    assert_eq!(meshes.text, "Meshes: 1 → 2 (added: Back; changed: Front)");
    assert_eq!(meshes.items[0].id, "Back");
    assert_eq!(meshes.items[0].text, "added");
    assert_eq!(meshes.items[0].element.as_deref(), Some("LongHair/Back"));
    assert_eq!(meshes.items[1].text, "blending or masks");
    let differences = compare_with_module_file(&removed, &bytes).unwrap();
    let meshes = differences
        .iter()
        .find(|d| d.text.starts_with("Meshes: "))
        .unwrap();
    assert_eq!(meshes.text, "Meshes: 2 → 1 (removed: Back; changed: Front)");
    assert_eq!(meshes.items.len(), 1);
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
    assert_eq!(texts(&differences), ["Meshes: 2 → 2 (changed: Front)"]);
    assert_eq!(differences[0].items[0].text, "UVs of 1 vertex");

    // 画像を描き替えると、そのテクスチャの画像が違う。
    let mut repainted = project.clone();
    repaint(&mut repainted, pattern_png(6, 5, 3));
    assert_eq!(
        texts(&compare_with_module_file(&repainted, &bytes).unwrap()),
        ["Textures: 2 → 2 (different image: front)"]
    );

    // メッシュを除いても、ファイルのそのメッシュのテクスチャを画素で見つける。
    let mut document = project.document();
    document
        .apply(Command::RemoveMesh {
            id: "LongHair/Back".into(),
        })
        .unwrap();
    let removed = Project::from_document(project.meta.clone(), &document);
    let lines = texts(&compare_with_module_file(&removed, &bytes).unwrap());
    assert!(
        lines
            .iter()
            .all(|l| !l.starts_with("Textures") && !l.starts_with("Some meshes")),
        "{lines:?}"
    );
    assert!(
        lines.contains(&"Meshes: 2 → 1 (removed: Back; changed: Front)".to_owned()),
        "{lines:?}"
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
    assert_eq!(texts(&differences), ["Meshes: 2 → 2 (changed: Front)"]);
    assert_eq!(differences[0].items[0].text, "UVs of 4 vertices");

    // 画像も UV も違えば、場所を確かめられない。
    repaint(&mut changed, pattern_png(6, 5, 4));
    let lines = texts(&compare_with_module_file(&changed, &bytes).unwrap());
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("Some meshes in the file's texture atlases")),
        "{lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("Textures: ") && l.contains("atlas-0")),
        "{lines:?}"
    );
}
