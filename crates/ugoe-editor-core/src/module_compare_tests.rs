//! 今の編集と書き出した `.ugoemod` の違い (`crate::module_compare`) と、アトラス化を選べるモジュールの書き出し
//! (`crate::export::export_module_with_settings`) のテスト (T0654)。

use std::sync::Arc;

use ugoe_model::fixtures::{dress_up_base, long_hair_module};
use ugoe_model::module::ModuleRequirements;

use crate::module_project::{ModuleBase, ModuleContext};
use crate::texture::{Textures, solid_png};
use crate::{
    Command, Document, ExportSettings, Project, ProjectError, ProjectMeta,
    compare_with_module_file, export_module, export_module_with_settings,
};

/// 本体 (`dress_up_base`) の Hair に LongHair を付けたモジュールのプロジェクト。
fn project() -> Project {
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
    let module = long_hair_module();
    let context = Arc::new(
        ModuleContext::new(
            base,
            &module.slot,
            module.name.clone(),
            module.requires.clone(),
        )
        .unwrap(),
    );
    let images: Textures = module
        .textures
        .iter()
        .map(|t| (t.name.clone(), Arc::from(solid_png(2, 2, [1, 2, 3, 255]))))
        .collect();
    let workspace = context.workspace(&module, &images).unwrap();
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
    assert_eq!(
        compare_with_module_file(&project, &bytes).unwrap(),
        Vec::<String>::new()
    );
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
    let lines = compare_with_module_file(&changed, &bytes).unwrap();
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
    let lines = compare_with_module_file(&removed, &bytes).unwrap();
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

    let lines = compare_with_module_file(&project, &atlased.bytes).unwrap();
    assert!(
        lines.iter().any(|l| l.contains("packed into atlases")),
        "{lines:?}"
    );

    let invalid = ExportSettings {
        atlas: true,
        atlas_max_size: 0,
        ..ExportSettings::default()
    };
    assert!(export_module_with_settings(&project, &invalid).is_err());
}
