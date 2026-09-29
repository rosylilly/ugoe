//! 書き出しの設定とアトラス化を含む書き出し (`export.rs`) のテスト。

use crate::atlas_tests::model_with;
use crate::export::{
    DrawEstimate, ExportError, ExportSettings, estimate_draws, export_bundle_with_settings,
};
use crate::{Project, ProjectMeta};

#[test]
fn export_settings_are_kept_in_the_editor_section() {
    let mut meta = ProjectMeta::new("p");
    assert_eq!(meta.export_settings(), ExportSettings::default());
    assert!(!meta.export_settings().atlas);

    let settings = ExportSettings {
        atlas: true,
        atlas_max_size: 1024,
        atlas_padding: 1,
    };
    assert!(meta.set_export_settings(settings).unwrap());
    assert!(!meta.set_export_settings(settings).unwrap());
    let json = serde_json::to_value(&meta).unwrap();
    assert_eq!(
        json["editor"]["export"],
        serde_json::json!({ "atlas": true, "atlas_max_size": 1024, "atlas_padding": 1 })
    );
    let loaded: ProjectMeta = serde_json::from_value(json).unwrap();
    assert_eq!(loaded.export_settings(), settings);

    // 既定値に戻すと `editor` から消える (既存のプロジェクトの project.json を変えない)。
    assert!(meta.set_export_settings(ExportSettings::default()).unwrap());
    assert!(meta.editor.is_empty());
}

#[test]
fn missing_or_broken_export_settings_read_as_defaults() {
    let meta: ProjectMeta = serde_json::from_value(serde_json::json!({
        "format_version": 0,
        "name": "p",
        "editor": { "export": { "atlas": true } }
    }))
    .unwrap();
    assert_eq!(
        meta.export_settings(),
        ExportSettings {
            atlas: true,
            ..ExportSettings::default()
        }
    );
    let broken: ProjectMeta = serde_json::from_value(serde_json::json!({
        "format_version": 0,
        "name": "p",
        "editor": { "export": "yes" }
    }))
    .unwrap();
    assert_eq!(broken.export_settings(), ExportSettings::default());
}

#[test]
fn invalid_export_settings_are_rejected() {
    let mut meta = ProjectMeta::new("p");
    for settings in [
        ExportSettings {
            atlas_max_size: 0,
            ..ExportSettings::default()
        },
        ExportSettings {
            atlas_max_size: 100_000,
            ..ExportSettings::default()
        },
        ExportSettings {
            atlas_padding: 65,
            ..ExportSettings::default()
        },
    ] {
        assert!(matches!(
            meta.set_export_settings(settings),
            Err(ExportError::InvalidMaxSize(_) | ExportError::InvalidPadding(_))
        ));
    }
    assert!(meta.editor.is_empty());
}

#[test]
fn estimates_texture_switches_in_draw_order() {
    // 描画順: M0 (tex0) → M1 (tex1) → M2 (tex0) → M3 (tex0)。
    let (model, _) = model_with(&[(0, 0.0), (1, 1.0), (0, 2.0), (0, 3.0)], &[(4, 4), (4, 4)]);
    assert_eq!(
        estimate_draws(&model),
        DrawEstimate {
            textures: 2,
            drawables: 4,
            draw_calls: 3,
            state_changes: 2,
        }
    );
    // 表示しないメッシュは描かない。
    let mut hidden = model.clone();
    hidden.meshes[1].mask_only = true;
    assert_eq!(estimate_draws(&hidden).draw_calls, 1);
}

#[test]
fn export_with_atlas_reports_before_and_after() {
    let (model, textures) = model_with(
        &[(0, 0.0), (1, 1.0), (0, 2.0), (2, 3.0)],
        &[(4, 4), (5, 5), (6, 6)],
    );
    let project = Project::new(ProjectMeta::new("p"), model, textures);

    let (bytes, report) =
        export_bundle_with_settings(&project, &ExportSettings::default()).unwrap();
    assert!(report.atlas.is_none());
    assert_eq!(report.before, report.after);
    assert_eq!(report.bytes, bytes.len());
    assert_eq!(bytes, crate::export_bundle(&project).unwrap());

    let settings = ExportSettings {
        atlas: true,
        ..ExportSettings::default()
    };
    let (bytes, report) = export_bundle_with_settings(&project, &settings).unwrap();
    assert_eq!(
        (report.before.textures, report.before.state_changes),
        (3, 3)
    );
    assert_eq!((report.after.textures, report.after.state_changes), (1, 0));
    assert_eq!(report.after.draw_calls, 1);
    assert_eq!(report.atlas.as_ref().map(|a| a.packed), Some(3));
    let bundle = ugoe_format::read_bundle(&bytes).unwrap();
    assert_eq!(bundle.textures.len(), 1);
    assert_eq!(bundle.model.textures[0].name, "atlas-0");

    let invalid = ExportSettings {
        atlas_padding: 1000,
        ..settings
    };
    assert!(matches!(
        export_bundle_with_settings(&project, &invalid),
        Err(ExportError::InvalidPadding(1000))
    ));
}
