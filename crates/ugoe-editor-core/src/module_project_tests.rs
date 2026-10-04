//! モジュールのプロジェクト (`crate::module_project`) のテスト。

use std::sync::Arc;

use ugoe_core::Rgb;
use ugoe_core::Vec2;
use ugoe_model::fixtures::{dress_module, dress_up_base, long_hair_module};
use ugoe_model::mesh::MeshKeyform;
use ugoe_model::module::ModuleRequirements;
use ugoe_model::parameter::ParameterKind;
use ugoe_model::{
    KeyformBinding, KeyformGrid, Mesh, MirrorSource, Module, Parameter, Part, PartId,
};

use crate::module_project::{ColliderReference, ModuleBase, ModuleContext, ModuleError};
use crate::texture::{Textures, solid_png};
use crate::{
    Command, Document, EditError, Project, ProjectError, ProjectKind, ProjectMeta,
    export_base_bundle, export_bundle, export_module, export_module_as,
};

mod colliders;
mod module_settings;
mod pose;
mod rebuild;
mod saving;
mod workspace;

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

// ---- プロジェクト ----

/// 本体のプロジェクトを `dir/avatar.ugoeproj` に保存する。
fn save_base(dir: &std::path::Path) -> std::path::PathBuf {
    let base = base();
    let path = dir.join("avatar.ugoeproj");
    Project::new(ProjectMeta::new("avatar"), base.model, base.textures)
        .save(&path)
        .unwrap();
    path
}

fn settings(name: &str, slot: &str, requires: ModuleRequirements) -> Command {
    Command::SetModuleSettings {
        name: name.to_owned(),
        slot: slot.into(),
        requires,
        keep_look: false,
    }
}
