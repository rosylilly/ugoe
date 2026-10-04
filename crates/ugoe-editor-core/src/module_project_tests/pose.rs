//! モジュールのポーズのグループのテスト。

use super::*;

/// `set_pose` で衣装のパーツだけのグループを足せる (モジュールのポーズになる)。本体のパーツを入れたグループは断る。
#[test]
fn module_pose_groups_can_be_set() {
    use ugoe_model::pose::{Pose, PoseEntry, PoseGroup};
    let module = long_hair_module();
    let mut document = document(&module);
    let entry = |part: &str| PoseEntry {
        part: part.into(),
        links: Vec::new(),
    };
    let own = |id: &str| format!("{}/{id}", module.name);
    let first = own(module.parts[0].id.as_str());
    for (id, name) in [("HairA", "Hair A"), ("HairB", "Hair B")] {
        document
            .apply(Command::AddPart {
                part: Part {
                    id: id.into(),
                    name: name.into(),
                    parent: Some(first.as_str().into()),
                    keyforms: KeyformGrid::constant(ugoe_model::part::PartKeyform { opacity: 1.0 }),
                },
            })
            .unwrap();
    }
    document
        .apply(Command::SetPose {
            pose: Pose {
                groups: vec![PoseGroup {
                    entries: vec![entry("HairA"), entry("HairB")],
                }],
                ..Pose::default()
            },
        })
        .unwrap();
    let split = split(&document);
    assert_eq!(split.pose.groups.len(), 1);
    assert_eq!(split.pose.groups[0].entries[1].part.as_str(), "HairB");

    let base_part = dress_up_base().parts[0].id.clone();
    let error = document
        .apply(Command::SetPose {
            pose: Pose {
                groups: vec![PoseGroup {
                    entries: vec![entry(base_part.as_str()), entry("HairA")],
                }],
                ..Pose::default()
            },
        })
        .unwrap_err();
    assert!(
        matches!(
            error,
            EditError::ModuleShape { .. } | EditError::ReadOnlyBase { .. }
        ),
        "{error:?}"
    );
    // Undo で最初のポーズの設定の前に戻る。
    assert!(document.undo());
    assert!(document.model().pose.groups.is_empty());
}

/// 本体にポーズのグループがあるとき、その要素を並べ替える・消す `set_pose` は本体の変更として断る (本体の検査)。
/// 衣装のグループを後ろに足すのは通る。
#[test]
fn base_pose_groups_are_read_only() {
    use ugoe_model::pose::{PoseEntry, PoseGroup};
    let entry = |part: &str| PoseEntry {
        part: part.into(),
        links: Vec::new(),
    };
    let mut base = base();
    base.model.pose.groups.push(PoseGroup {
        entries: vec![entry("Hair"), entry("Body")],
    });
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
    let workspace = context.workspace(&module, &images(&module)).unwrap();
    let mut document = Document::with_textures(workspace.model, workspace.textures)
        .with_physics(workspace.physics)
        .with_module(context);
    let original = document.model().pose.clone();
    assert_eq!(original.groups.len(), 1);

    let mut reordered = original.clone();
    reordered.groups[0].entries.reverse();
    let mut removed = original.clone();
    removed.groups[0].entries.pop();
    let mut emptied = original.clone();
    emptied.groups.clear();
    for pose in [reordered, removed, emptied] {
        let error = document.apply(Command::SetPose { pose }).unwrap_err();
        assert_eq!(
            error,
            EditError::ReadOnlyBase {
                element: "the pose".into()
            }
        );
        assert_eq!(document.model().pose, original);
    }
    assert!(!document.is_modified());

    // 衣装のパーツのグループを後ろに足すのは通り、Undo で戻る。
    let own = format!("{}/{}", module.name, module.parts[0].id);
    let mut added = original.clone();
    added.groups.push(PoseGroup {
        entries: vec![entry(&own)],
    });
    document.apply(Command::SetPose { pose: added }).unwrap();
    assert_eq!(document.model().pose.groups.len(), 2);
    assert!(document.undo());
    assert_eq!(document.model().pose, original);
}
