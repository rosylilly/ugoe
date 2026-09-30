//! Rust から生成してリポジトリに置くファイル (フロントエンドの型と `Command` の JSON Schema)。
//! `just gen` (`examples/gen.rs`) が書き出し、テストが最新かどうかを確かめる。
//! 手順は `docs/editor.md#型の生成`。

use std::path::{Path, PathBuf};

use ugoe_editor_core::avatar::AvatarCheck;
use ugoe_editor_core::codegen::{TsBindings, command_schema_json, typescript_samples};
use ugoe_editor_core::lighting::{BulgeSettings, PreviewLighting};
use ugoe_editor_core::overflow::{ElementOverflow, OverflowReport};
use ugoe_editor_core::physics_check::RestPoseCollision;
use ugoe_editor_core::{ApplyReport, ExportReport, ExportSettings, PsdImportWarning};

use crate::ai::{AiReply, AiSettings, ConsentAnswer};
use crate::ai_provider::ProviderSettings;
use crate::commands::{AnimationData, AppInfo};
use crate::dress_up::PreviewModule;
use crate::lighting::{NormalMapGroupChoice, NormalMapGroupView};
use crate::mcp::{BridgeEvent, McpBridgeStatus};
use crate::module::{
    BaseReload, BaseSummary, ModuleExportReport, ModuleProblem, ModuleProblemKind, ModuleStatus,
    SlotSummary,
};
use crate::project::{ExternalChange, ProjectStatus};
use crate::proposals::{Classification, ModuleRigChoice, ProposalPreview, ProposalState};

/// リポジトリのルート。
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

/// 生成するファイル (リポジトリのルートからのパス → 中身)。
pub fn generated_files() -> Result<Vec<(&'static str, String)>, serde_json::Error> {
    let mut types = TsBindings::new().with_editor_types();
    // IPC の戻り値 (apps/editor/src-tauri の型)
    types.add::<AppInfo>();
    types.add::<ProjectStatus>();
    types.add::<ExternalChange>();
    types.add::<PsdImportWarning>();
    types.add::<ApplyReport>();
    types.add::<AnimationData>();
    types.add::<ElementOverflow>();
    types.add::<OverflowReport>();
    types.add::<RestPoseCollision>();
    types.add::<AiSettings>();
    types.add::<ProviderSettings>();
    types.add::<ConsentAnswer>();
    types.add::<AiReply<()>>();
    types.add::<ProposalState>();
    types.add::<Classification>();
    types.add::<ProposalPreview>();
    types.add::<ModuleRigChoice>();
    types.add::<ExportSettings>();
    types.add::<ExportReport>();
    types.add::<AvatarCheck>();
    types.add::<McpBridgeStatus>();
    types.add::<BridgeEvent>();
    types.add::<BulgeSettings>();
    types.add::<NormalMapGroupView>();
    types.add::<NormalMapGroupChoice>();
    types.add::<PreviewLighting>();
    types.add::<PreviewModule>();
    types.add::<SlotSummary>();
    types.add::<BaseSummary>();
    types.add::<ModuleStatus>();
    types.add::<ModuleProblemKind>();
    types.add::<ModuleProblem>();
    types.add::<BaseReload>();
    types.add::<ModuleExportReport>();
    Ok(vec![
        ("apps/editor/src/lib/generated/types.ts", types.render()),
        (
            "apps/editor/src/lib/generated/samples.ts",
            typescript_samples("./types")?,
        ),
        (
            "crates/ugoe-editor-core/schema/command.schema.json",
            command_schema_json()?,
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_files_are_up_to_date() {
        for (path, content) in generated_files().unwrap() {
            let on_disk = std::fs::read_to_string(repo_root().join(path)).unwrap_or_default();
            assert!(on_disk == content, "{path} is stale; run `just gen`");
        }
    }
}
