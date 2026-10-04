//! File > Open Sample…: 着せ替えの見本 (`assets/samples/dress-up/`、T0710) を、エディタで開けるプロジェクトとして
//! 書き出して開く (T0715)。見本の `.ugoe`・`.ugoemod` はバイナリに埋め込み、`ugoe sample dress-up --project` と
//! 同じ形 (本体 `dress-up.ugoeproj` と、それを参照するモジュールのプロジェクト) にする。
//! 引数・戻り値の型を変えたら `apps/editor/src/lib/ipc.ts` も同じ PR で更新すること。

use std::path::{Path, PathBuf};

use tauri::State;
use ugoe_editor_core::Message;
use ugoe_editor_core::Project;
use ugoe_editor_core::bundle_project::save_bundle_projects;

use crate::AppState;
use crate::error::{EditorError, IpcResult};
use crate::project::{ProjectSession, ProjectStatus, absolute_path, lock_both, status};

/// 見本の本体。
const BASE: &[u8] = include_bytes!("../../../../assets/samples/dress-up/dress-up.ugoe");

/// 見本のモジュール。プロジェクトのディレクトリ名は名前の kebab-case (`long-hair.ugoeproj` など)。
const MODULES: [&[u8]; 4] = [
    include_bytes!("../../../../assets/samples/dress-up/long-hair.ugoemod"),
    include_bytes!("../../../../assets/samples/dress-up/twin-tails.ugoemod"),
    include_bytes!("../../../../assets/samples/dress-up/cat-ears.ugoemod"),
    include_bytes!("../../../../assets/samples/dress-up/sailor.ugoemod"),
];

/// 本体のプロジェクトのディレクトリ名。
const BASE_PROJECT: &str = "dress-up.ugoeproj";

/// 見本を書き出すフォルダーの名前 (`parent` の中に作る。あれば `-2`・`-3`… を付ける)。
const FOLDER: &str = "ugoe-dress-up-sample";

/// 見本のプロジェクトを `parent` の中の新しいフォルダーに書き出し、本体のプロジェクトを開く。
/// 既にあるフォルダーには書かない (前に書き出して編集した見本を上書きしない)。未保存の変更は破棄する
/// (確認はフロントエンドで行う)。失敗したら今のプロジェクトはそのまま。
#[tauri::command]
pub fn open_sample_project(state: State<'_, AppState>, parent: String) -> IpcResult<ProjectStatus> {
    let dir = new_folder(&absolute_path(&parent)?)?;
    let base = write_projects(&dir)?
        .into_iter()
        .next()
        .ok_or_else(|| EditorError::Internal("no sample project was written".into()))?;
    let (project, disk) = Project::load_tracked(&base)?;
    let (mut document, mut session) = lock_both(&state)?;
    (*session, *document) = ProjectSession::from_project(project, Some((base, disk)));
    Ok(status(&session, &document))
}

/// `parent` の中の、まだ無いフォルダーを作る。
fn new_folder(parent: &Path) -> IpcResult<PathBuf> {
    for n in 1..100 {
        let name = if n == 1 {
            FOLDER.to_owned()
        } else {
            format!("{FOLDER}-{n}")
        };
        let dir = parent.join(name);
        match std::fs::create_dir(&dir) {
            Ok(()) => return Ok(dir),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => {
                return Err(EditorError::CreateFolder {
                    path: dir.display().to_string(),
                    detail: e.to_string(),
                }
                .into());
            }
        }
    }
    Err(EditorError::TooManySampleFolders(parent.display().to_string()).into())
}

/// 見本の本体とモジュールのプロジェクトを `dir` に書く。書いたプロジェクトのパスを、本体を先に返す。
/// 変換は `ugoe sample dress-up --project` と同じ `bundle_project::save_bundle_projects` (本体の物理・モーション・
/// 表情も写す。T0655)。
fn write_projects(dir: &Path) -> IpcResult<Vec<PathBuf>> {
    let bundle = ugoe_format::read_bundle(BASE)?;
    let modules = MODULES
        .iter()
        .map(|bytes| ugoe_format::read_module(bytes))
        .collect::<Result<Vec<_>, _>>()?;
    save_bundle_projects(dir, BASE_PROJECT, &bundle, &modules).map_err(Message::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ugoe-editor-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// 書き出した見本を開ける: 本体は見本の本体と同じモデル・物理・モーション・表情、モジュールのプロジェクトは
    /// 本体を参照し、書き出す (Export Module) と埋め込んだ `.ugoemod` とバイト列で同じになる。
    #[test]
    fn sample_projects_open_and_export_the_same_modules() {
        let dir = temp_dir("sample");
        let written = write_projects(&dir).unwrap();
        assert_eq!(written.len(), 1 + MODULES.len());

        let bundle = ugoe_format::read_bundle(BASE).unwrap();
        let base = Project::load(&written[0]).unwrap();
        assert!(base.module.is_none());
        assert_eq!(base.model, bundle.model);
        assert_eq!(base.physics, bundle.physics);
        assert!(!bundle.motions.is_empty() && !bundle.expressions.is_empty());
        assert_eq!(base.motions, bundle.motions);
        assert_eq!(base.expressions, bundle.expressions);
        let names: Vec<_> = written
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            names,
            [
                "dress-up.ugoeproj",
                "long-hair.ugoeproj",
                "twin-tails.ugoeproj",
                "cat-ears.ugoeproj",
                "sailor.ugoeproj"
            ]
        );

        for (path, bytes) in written[1..].iter().zip(MODULES) {
            let project = Project::load(path).unwrap();
            assert!(project.module.is_some(), "{}", path.display());
            let export = ugoe_editor_core::export_module(&project).unwrap();
            assert!(export.warnings.is_empty(), "{:?}", export.warnings);
            assert!(
                export.bytes == bytes,
                "{} is exported differently",
                path.display()
            );
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// 見本のスロット `Hair` は本体の肩の衝突体を公開する。本体から物理を外すと、モジュールのプロジェクトの
    /// 状態に当たれない衝突体として出て、リグがそれに当たるモジュールは本体に物理を足すよう案内する問題で
    /// 開けない (T0660)。
    #[test]
    fn exposed_colliders_without_base_physics_are_explained() {
        use crate::module::{ModuleProblemKind, module_open_problem, module_status_of};

        let dir = temp_dir("sample-colliders");
        let bundle = ugoe_format::read_bundle(BASE).unwrap();
        // `TwinTails` の房は肩に当たり、`LongHair` は当たらない (T0662)。
        let colliding = ugoe_format::read_module(MODULES[1]).unwrap();
        assert!(
            colliding.module.physics.rigs[0]
                .colliders
                .contains(&"Shoulders".into())
        );
        let plain = ugoe_format::read_module(MODULES[0]).unwrap();
        assert!(
            plain
                .module
                .physics
                .rigs
                .iter()
                .all(|r| r.colliders.is_empty())
        );
        let written =
            save_bundle_projects(&dir, BASE_PROJECT, &bundle, &[colliding, plain]).unwrap();
        let open = |path: &Path| {
            let (_, document) = ProjectSession::from_project(Project::load(path).unwrap(), None);
            module_status_of(&document).unwrap()
        };
        let problem = |path: &Path| module_open_problem(path.display().to_string(), None).unwrap();
        assert!(problem(&written[1]).is_none());
        assert!(open(&written[1]).unavailable_colliders.is_empty());
        assert_eq!(open(&written[2]).base_colliders, ["Shoulders"]);

        let mut base = Project::load(&written[0]).unwrap();
        base.physics = None;
        base.save(&written[0]).unwrap();
        let found = problem(&written[1]).unwrap();
        assert_eq!(found.kind, ModuleProblemKind::BaseNeedsPhysics);
        assert!(
            found.error.message.contains("`Shoulders`"),
            "{}",
            found.error.message
        );
        assert!(
            found.error.message.contains("no physics"),
            "{}",
            found.error.message
        );
        assert_eq!(found.error.code, "module.collider_needs_base_physics");
        // 外せば開ける参照を知らせる (T0661)。`TwinTails` の房のリグは 2 つとも肩に当たる (T0662)。
        let rigs: Vec<_> = ugoe_format::read_module(MODULES[1])
            .unwrap()
            .module
            .physics
            .rigs
            .iter()
            .filter(|r| r.colliders.contains(&"Shoulders".into()))
            .map(|r| r.id.to_string())
            .collect();
        assert_eq!(rigs.len(), 2);
        let found_rigs: Vec<_> = found
            .collider_references
            .iter()
            .map(|c| c.rig.clone())
            .collect();
        assert_eq!(found_rigs, rigs);
        assert!(
            found
                .collider_references
                .iter()
                .all(|c| c.collider == "Shoulders")
        );
        let (reopened, _) =
            Project::load_module_without_unavailable_colliders(&written[1], None, None).unwrap();
        assert!(
            reopened
                .physics
                .unwrap()
                .rigs
                .iter()
                .all(|r| r.colliders.is_empty())
        );
        assert!(problem(&written[2]).is_none());
        assert_eq!(open(&written[2]).unavailable_colliders, ["Shoulders"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// 書き出すフォルダーは毎回新しく作り、前の見本を上書きしない。
    #[test]
    fn sample_folders_do_not_overwrite_earlier_ones() {
        let dir = temp_dir("sample-folders");
        let first = new_folder(&dir).unwrap();
        let second = new_folder(&dir).unwrap();
        assert_eq!(first.file_name().unwrap(), FOLDER);
        assert_eq!(
            second.file_name().unwrap().to_string_lossy(),
            format!("{FOLDER}-2")
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
