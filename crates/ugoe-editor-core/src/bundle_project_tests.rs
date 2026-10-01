//! バンドルとモジュールからのプロジェクト (`crate::bundle_project`) のテスト。

use std::path::PathBuf;

use ugoe_format::{Bundle, ModuleFile, TextureData, TextureFormat};
use ugoe_model::fixtures::{dress_module, dress_up_base, long_hair_module};

use crate::bundle_project::{module_project_name, project_from_bundle, save_bundle_projects};
use crate::texture::solid_png;
use crate::{Project, ProjectError, export_module, physics_edit};

fn png(rgba: [u8; 4]) -> TextureData {
    TextureData {
        format: TextureFormat::Png,
        bytes: solid_png(2, 2, rgba),
    }
}

fn bundle() -> Bundle {
    let mut bundle = Bundle::new(dress_up_base());
    bundle.textures = bundle
        .model
        .textures
        .iter()
        .map(|_| png([255; 4]))
        .collect();
    bundle.physics = Some(physics_edit::default_settings());
    bundle
}

fn modules() -> Vec<ModuleFile> {
    let mut hair = ModuleFile::new(long_hair_module());
    // 法線マップも名前で入る。
    hair.module.textures[0].normal_map = Some("hair-normal".into());
    hair.textures = vec![png([1, 2, 3, 255])];
    hair.normal_maps = vec![png([128, 128, 255, 255])];
    let mut dress = ModuleFile::new(dress_module());
    dress.textures = vec![png([4, 5, 6, 255])];
    vec![hair, dress]
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ugoe-editor-core-bundle-project-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn module_project_names_are_kebab_case() {
    assert_eq!(module_project_name("LongHair"), "long-hair.ugoeproj");
    assert_eq!(module_project_name("Sailor"), "sailor.ugoeproj");
}

/// 書いた本体はバンドルと同じ中身で開け、モジュールのプロジェクトを書き出すと元の `.ugoemod` とバイト列で同じになる。
#[test]
fn saved_projects_open_and_export_the_same_modules() {
    let dir = temp_dir("save");
    let bundle = bundle();
    let modules = modules();
    let written = save_bundle_projects(&dir, "base.ugoeproj", &bundle, &modules).unwrap();
    assert_eq!(
        written,
        [
            dir.join("base.ugoeproj"),
            dir.join("long-hair.ugoeproj"),
            dir.join("dress.ugoeproj")
        ]
    );

    let base = Project::load(&written[0]).unwrap();
    assert_eq!(base, project_from_bundle(&bundle).unwrap());
    assert_eq!(base.meta.name, bundle.model.meta.name);
    assert_eq!(base.physics, bundle.physics);

    for (path, file) in written[1..].iter().zip(&modules) {
        let project = Project::load(path).unwrap();
        assert!(project.module.is_some(), "{}", path.display());
        let export = export_module(&project).unwrap();
        assert!(export.warnings.is_empty(), "{:?}", export.warnings);
        assert!(
            export.bytes == ugoe_format::write_module(file).unwrap(),
            "{} is exported differently",
            file.module.name
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn textures_must_be_png() {
    let mut bundle = bundle();
    bundle.model.textures.push(ugoe_model::TextureRef {
        name: "ktx".into(),
        width: 2,
        height: 2,
        normal_map: None,
    });
    bundle.textures.push(TextureData {
        format: TextureFormat::Ktx2,
        bytes: Vec::new(),
    });
    assert!(matches!(
        project_from_bundle(&bundle).unwrap_err(),
        ProjectError::UnsupportedTexture { name } if name == "ktx"
    ));
    bundle.textures.pop();
    assert!(matches!(
        project_from_bundle(&bundle).unwrap_err(),
        ProjectError::MissingTexture { name } if name == "ktx"
    ));
}
