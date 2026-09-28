use serde::{Deserialize, Serialize};
use ugoe_model::{ParamId, Parameter};

/// 編集操作。JSON で表現でき、AI エージェントのツール呼び出しにそのまま対応する。
///
/// 追加時のルール (`docs/editor.md#コマンド`):
/// - 1 コマンド = ユーザーから見た 1 操作 (Undo の単位)。
/// - フィールドは ID で対象を指定する (インデックスは使わない)。
/// - 適用前検証を [`crate::Document::apply`] 内で行い、失敗時はモデルを一切変更しない。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Command {
    AddParameter {
        parameter: Parameter,
    },
    RemoveParameter {
        id: ParamId,
    },
    SetParameterRange {
        id: ParamId,
        min: f32,
        default: f32,
        max: f32,
    },
    /// 複数コマンドを 1 つの Undo 単位にまとめる (AI の一括提案など)。
    Batch {
        label: String,
        commands: Vec<Command>,
    },
}
