//! Rust から生成してリポジトリに置くファイル (フロントエンドの型と `Command` の JSON Schema、
//! エラーと警告のコードの一覧)。
//! `just gen` (`examples/gen.rs`) が書き出し、テストが最新かどうかを確かめる。
//! 手順は `docs/editor.md#型の生成`。

use std::path::{Path, PathBuf};

use ugoe_editor_core::avatar::AvatarCheck;
use ugoe_editor_core::codegen::{
    GENERATED_HEADER, TsBindings, command_schema_json, typescript_samples,
};
use ugoe_editor_core::lighting::{BulgeSettings, PreviewLighting};
use ugoe_editor_core::message_catalog::{message_codes, message_samples};
use ugoe_editor_core::overflow::{ElementOverflow, OverflowReport};
use ugoe_editor_core::physics_check::RestPoseCollision;
use ugoe_editor_core::{
    ApplyReport, ColliderReference, DifferenceItem, DifferenceKind, ExportReport, ExportSettings,
    Message, ModuleDifference, PsdImportWarning,
};

use crate::ai::{AiReply, AiSettings, ConsentAnswer};
use crate::ai_provider::ProviderSettings;
use crate::commands::{AnimationData, AppInfo};
use crate::dress_up::PreviewModule;
use crate::error;
use crate::history::HistoryStatus;
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
    types.add::<HistoryStatus>();
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
    types.add::<DifferenceKind>();
    types.add::<DifferenceItem>();
    types.add::<ModuleDifference>();
    types.add::<ColliderReference>();
    // IPC コマンドのエラー (ADR 0029)。
    types.add::<Message>();
    Ok(vec![
        ("apps/editor/src/lib/generated/types.ts", types.render()),
        (
            "apps/editor/src/lib/generated/samples.ts",
            typescript_samples("./types")?,
        ),
        (
            "apps/editor/src/lib/generated/messageCodes.ts",
            typescript_message_codes()?,
        ),
        (
            "crates/ugoe-editor-core/schema/command.schema.json",
            command_schema_json()?,
        ),
    ])
}

/// バックエンド (editor-core とエディタ) が画面に出すエラー・警告のコードと引数の名前の一覧。
/// 翻訳のカタログ (`locales/<言語>.json` の `errors`) の検査 (`src/lib/messageCodes.test.ts`) が読む。
fn typescript_message_codes() -> Result<String, serde_json::Error> {
    let codes = message_codes(
        message_samples()
            .into_iter()
            .chain(error::message_samples()),
    );
    let mut out = GENERATED_HEADER.to_owned();
    out.push_str(
        "// バックエンドが画面に出すエラー・警告のコード → 引数の名前 (docs/editor.md#エラーと警告のコード)。\n\n\
         export const messageCodes: Record<string, readonly string[]> = ",
    );
    out.push_str(&serde_json::to_string_pretty(&codes)?);
    out.push_str(";\n");
    Ok(out)
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
