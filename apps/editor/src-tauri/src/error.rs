//! IPC コマンドのエラー。コマンドはエラーを [`Message`] (コード・引数・英語の文。ADR 0029) で返す。
//! editor-core のエラーは `Coded` を実装しているので `?` でそのまま変わる。バックエンドだけで起きるエラーは
//! [`EditorError`] にまとめる。コードの一覧は `docs/editor.md#エラーと警告のコード`。
//! **コードは公開の約束なので名前を変えない。** 英語の文は前と同じにする (ログ・MCP で使う)。

use std::borrow::Cow;
use std::path::Path;

use ugoe_ai::AiError;
use ugoe_ai::rig::RigTemplate;
use ugoe_editor_core::{Coded, Message, MessageArgs, message_args};
use ugoe_psd::PsdError;

/// IPC コマンドの戻り値。
pub type IpcResult<T> = Result<T, Message>;

/// エディタのバックエンドだけで起きるエラー。
#[derive(Debug, thiserror::Error)]
pub enum EditorError {
    /// 錠が壊れた (別のスレッドが錠を持ったまま止まった)。`what` は `document`・`project` など。
    #[error("{what} lock poisoned")]
    LockPoisoned { what: &'static str },
    /// ファイルの読み書きに失敗した。
    #[error("{path}: {detail}")]
    Io { path: String, detail: String },
    /// 設定などを JSON にできない (起きないはず)。
    #[error("{0}")]
    Internal(String),
    /// 画面のスレッドの外で動かした処理が終わらなかった。
    #[error("{0}")]
    Task(String),
    /// 時間のかかる編集 (PSD の取り込み・法線マップを作る) をロックの外で作る間に、モデルが変わった
    /// (つないだエージェントの編集・別のプロジェクトを開いたなど)。作った編集は適用しない。
    #[error("the model changed while the edit was being made; try again")]
    ModelChanged,
    #[error("path is empty")]
    EmptyPath,
    #[error("path must be absolute: {0}")]
    RelativePath(String),
    #[error("the project has not been saved yet")]
    NotSaved,
    #[error("the project has not been saved yet; use Save As")]
    NotSavedUseSaveAs,
    #[error(
        "the project was changed outside the editor after it was opened; reload it, or save again to overwrite"
    )]
    ExternalChange,
    #[error("this is not a module project")]
    NotAModule,
    #[error("expected the bytes of a .ugoemod file")]
    ExpectedModuleBytes,
    #[error("expected the bytes of a PNG image")]
    ExpectedPngBytes,
    #[error("not a .ugoemod file: {0}")]
    NotModuleFile(String),
    #[error("not a PNG file: {0}")]
    NotPngFile(String),
    #[error("the data is not a PNG image")]
    NotPngData,
    #[error("not a JSON file: {0}")]
    NotJsonFile(String),
    #[error("missing the {0} header")]
    MissingHeader(&'static str),
    #[error("{header}: {detail}")]
    InvalidHeader {
        header: &'static str,
        detail: String,
    },
    #[error("invalid percent-encoding: {0}")]
    InvalidPercentEncoding(String),
    #[error("creating {path}: {detail}")]
    CreateFolder { path: String, detail: String },
    #[error("too many sample folders in {0}; remove old ones")]
    TooManySampleFolders(String),
    /// PSD のファイルを読めない。`error` は原因 (`psd_read.*`) に載せる。
    #[error("{path}: {error}")]
    PsdRead { path: String, error: PsdError },
    // ---- ライティング ----
    #[error("normal maps are proposed only in model projects")]
    NormalMapsModelOnly,
    #[error("choose at least one group")]
    NoGroupChosen,
    #[error("unknown group `{0}`")]
    UnknownGroup(String),
    #[error("no texture is in the group `{0}`")]
    EmptyGroup(String),
    #[error("image `{0}` not found")]
    ImageNotFound(String),
    // ---- 設定 ----
    #[error("the undo limit must be {min} to {max} steps")]
    InvalidUndoLimit { min: u32, max: u32 },
    // ---- AI ----
    #[error("choose a Claude model")]
    ClaudeModelMissing,
    #[error("the base URL must start with https:// or http://")]
    InvalidBaseUrl,
    #[error("choose a model")]
    ModelMissing,
    #[error("the Claude API needs an API key")]
    ApiKeyRequired,
    #[error("refusing to send the API key over plain http to a remote server; use https")]
    PlainHttpApiKey,
    #[error("the API key is empty")]
    EmptyApiKey,
    /// 資格情報の保存先のエラー (`crate::ai` の `describe` の文。キーの断片は含めない)。
    #[error("{0}")]
    CredentialStore(String),
    #[error("the requests go to different services")]
    MixedDestinations,
    #[error("this project is set not to send data to external AI services (see Settings)")]
    SendDisabled,
    #[error("the time limit must be {min} to {max} minutes")]
    InvalidTimeLimit { min: u32, max: u32 },
    #[error(transparent)]
    Ai(#[from] AiError),
    // ---- 提案 ----
    #[error("the model changed while the proposal was being made; propose again")]
    ProposalModelChanged,
    #[error("no PSD to classify: choose the PSD this model was imported from")]
    NoPsd,
    #[error("classify the PSD layers first")]
    NotClassified,
    #[error("proposal {0} was not made in this panel, so it cannot be regenerated")]
    NotRegenerable(u32),
    #[error("the proposal disappeared")]
    ProposalDisappeared,
    #[error("no proposal is being previewed")]
    NoPreview,
    // ---- エージェントの接続 (MCP のブリッジ) ----
    #[error("choose a port between 1 and 65535, or leave it empty")]
    InvalidPort,
    /// ブリッジを始められない (ポートが使われているなど)。
    #[error("{0}")]
    BridgeStart(String),
    #[error(
        "no ugoe-mcp was found next to the editor; build it with `cargo build --release -p ugoe-mcp` \
         (installers include it)"
    )]
    NoMcpBinary,
    #[error("Claude Desktop extensions need macOS, Windows or Linux")]
    UnsupportedPlatform,
    /// `.mcpb` を書けない。
    #[error("{0}")]
    BundleWrite(String),
    #[error("could not open {path}: {detail}")]
    OpenFailed { path: String, detail: String },
    #[error("the file name must end with .mcpb")]
    NotMcpbFile,
    // ---- ファイルの読み書きの許可 (T1616。`crate::access`) ----
    /// 読もうとしたパスを、利用者が選んでいない (ダイアログで選んだ・ドロップした・開いているプロジェクトの中や
    /// プロジェクトが指すもののどれでもない)。ビューア・スタジオと同じコード。
    #[error("{0}: the file was not chosen by the user")]
    NotChosen(String),
    /// 書こうとしたパスを、保存のダイアログで選んでいない (選んだパスにも 1 回しか書けない)。ビューア・スタジオと同じコード。
    #[error("{0}: the path was not chosen in the save dialog")]
    NotChosenForSave(String),
    /// OS のファイルのダイアログが失敗した。ビューア・スタジオと同じコード。
    #[error("the dialog failed: {0}")]
    DialogFailed(String),
}

impl EditorError {
    /// `path` の読み書きの失敗。
    pub fn io(path: &Path, error: impl ToString) -> Self {
        Self::Io {
            path: path.display().to_string(),
            detail: error.to_string(),
        }
    }

    /// `path` を利用者が選んでいない (`file.not_chosen`)。
    pub fn not_chosen(path: &Path) -> Self {
        Self::NotChosen(path.display().to_string())
    }

    /// `path` を保存のダイアログで選んでいない (`file.not_chosen_for_save`)。
    pub fn not_chosen_for_save(path: &Path) -> Self {
        Self::NotChosenForSave(path.display().to_string())
    }

    /// PSD の `path` を読めない。
    pub fn psd_read(path: &Path, error: PsdError) -> Self {
        Self::PsdRead {
            path: path.display().to_string(),
            error,
        }
    }
}

impl EditorError {
    /// [`Coded::code`] の値。どれも固定の文字列。
    fn fixed_code(&self) -> &'static str {
        match self {
            Self::LockPoisoned { .. } => "editor.lock_poisoned",
            Self::Io { .. } => "editor.io",
            Self::Internal(_) => "editor.internal",
            Self::Task(_) => "editor.task_failed",
            Self::ModelChanged => "editor.model_changed",
            Self::EmptyPath => "editor.empty_path",
            Self::RelativePath(_) => "editor.relative_path",
            Self::NotSaved => "editor.not_saved",
            Self::NotSavedUseSaveAs => "editor.not_saved_use_save_as",
            Self::ExternalChange => "editor.external_change",
            Self::NotAModule => "editor.not_a_module",
            Self::ExpectedModuleBytes => "editor.expected_module_bytes",
            Self::ExpectedPngBytes => "editor.expected_png_bytes",
            Self::NotModuleFile(_) => "editor.not_module_file",
            Self::NotPngFile(_) => "editor.not_png_file",
            Self::NotPngData => "editor.not_png_data",
            Self::NotJsonFile(_) => "editor.not_json_file",
            Self::MissingHeader(_) => "editor.missing_header",
            Self::InvalidHeader { .. } => "editor.invalid_header",
            Self::InvalidPercentEncoding(_) => "editor.invalid_percent_encoding",
            Self::CreateFolder { .. } => "editor.create_folder",
            Self::TooManySampleFolders(_) => "editor.too_many_sample_folders",
            Self::PsdRead { .. } => "psd.read_file_failed",
            Self::NormalMapsModelOnly => "lighting.normal_maps_model_only",
            Self::NoGroupChosen => "lighting.no_group_chosen",
            Self::UnknownGroup(_) => "lighting.unknown_group",
            Self::EmptyGroup(_) => "lighting.empty_group",
            Self::ImageNotFound(_) => "lighting.image_not_found",
            Self::InvalidUndoLimit { .. } => "editor.invalid_undo_limit",
            Self::ClaudeModelMissing => "ai.claude_model_missing",
            Self::InvalidBaseUrl => "ai.invalid_base_url",
            Self::ModelMissing => "ai.model_missing",
            Self::ApiKeyRequired => "ai.api_key_required",
            Self::PlainHttpApiKey => "ai.plain_http_api_key",
            Self::EmptyApiKey => "ai.empty_api_key",
            Self::CredentialStore(_) => "ai.credential_store",
            Self::MixedDestinations => "ai.mixed_destinations",
            Self::SendDisabled => "ai.send_disabled",
            Self::InvalidTimeLimit { .. } => "ai.invalid_time_limit",
            Self::Ai(error) => ai_code(error),
            Self::ProposalModelChanged => "proposal.model_changed",
            Self::NoPsd => "proposal.no_psd",
            Self::NotClassified => "proposal.not_classified",
            Self::NotRegenerable(_) => "proposal.not_regenerable",
            Self::ProposalDisappeared => "proposal.disappeared",
            Self::NoPreview => "proposal.no_preview",
            Self::InvalidPort => "mcp.invalid_port",
            Self::BridgeStart(_) => "mcp.bridge_start_failed",
            Self::NoMcpBinary => "mcp.no_binary",
            Self::UnsupportedPlatform => "mcp.unsupported_platform",
            Self::BundleWrite(_) => "mcp.bundle_write_failed",
            Self::OpenFailed { .. } => "mcp.open_failed",
            Self::NotMcpbFile => "mcp.not_mcpb_file",
            Self::NotChosen(_) => "file.not_chosen",
            Self::NotChosenForSave(_) => "file.not_chosen_for_save",
            Self::DialogFailed(_) => "dialog.failed",
        }
    }
}

impl Coded for EditorError {
    fn code(&self) -> Cow<'static, str> {
        Cow::Borrowed(self.fixed_code())
    }

    fn args(&self) -> MessageArgs {
        match self {
            Self::LockPoisoned { what } => message_args! { "what" => *what },
            Self::Io { path, detail }
            | Self::CreateFolder { path, detail }
            | Self::OpenFailed { path, detail } => {
                message_args! { "path" => path, "detail" => detail }
            }
            Self::PsdRead { path, error } => {
                message_args! { "path" => path, "detail" => error.to_string() }
            }
            Self::Internal(detail)
            | Self::Task(detail)
            | Self::InvalidPercentEncoding(detail)
            | Self::CredentialStore(detail)
            | Self::BridgeStart(detail)
            | Self::BundleWrite(detail) => message_args! { "detail" => detail },
            Self::RelativePath(path)
            | Self::NotModuleFile(path)
            | Self::NotPngFile(path)
            | Self::NotJsonFile(path)
            | Self::TooManySampleFolders(path)
            | Self::NotChosen(path)
            | Self::NotChosenForSave(path) => message_args! { "path" => path },
            Self::DialogFailed(reason) => message_args! { "reason" => reason },
            Self::MissingHeader(header) => message_args! { "header" => *header },
            Self::InvalidHeader { header, detail } => {
                message_args! { "header" => *header, "detail" => detail }
            }
            Self::UnknownGroup(group) | Self::EmptyGroup(group) => {
                message_args! { "group" => group }
            }
            Self::ImageNotFound(name) => message_args! { "name" => name },
            Self::NotRegenerable(id) => message_args! { "id" => *id },
            Self::InvalidTimeLimit { min, max } | Self::InvalidUndoLimit { min, max } => {
                message_args! { "min" => *min, "max" => *max }
            }
            Self::Ai(error) => ai_args(error),
            Self::ModelChanged
            | Self::EmptyPath
            | Self::NotSaved
            | Self::NotSavedUseSaveAs
            | Self::ExternalChange
            | Self::NotAModule
            | Self::ExpectedModuleBytes
            | Self::ExpectedPngBytes
            | Self::NotPngData
            | Self::NormalMapsModelOnly
            | Self::NoGroupChosen
            | Self::ClaudeModelMissing
            | Self::InvalidBaseUrl
            | Self::ModelMissing
            | Self::ApiKeyRequired
            | Self::PlainHttpApiKey
            | Self::EmptyApiKey
            | Self::MixedDestinations
            | Self::SendDisabled
            | Self::ProposalModelChanged
            | Self::NoPsd
            | Self::NotClassified
            | Self::ProposalDisappeared
            | Self::NoPreview
            | Self::InvalidPort
            | Self::NoMcpBinary
            | Self::UnsupportedPlatform
            | Self::NotMcpbFile => message_args! {},
        }
    }

    fn cause(&self) -> Option<Message> {
        match self {
            // 画面は原因の訳を `detail` に差し込む (T0571)。
            Self::PsdRead { error, .. } => Some(error.to_message()),
            _ => None,
        }
    }
}

/// エディタのバックエンドが出すすべてのコードの見本 (`ugoe_editor_core::message_catalog::message_samples` の
/// 続き)。引数の値に意味はない。翻訳のカタログの検査に使う (`bindings.rs`。コードを足したらここにも足す)。
pub fn message_samples() -> Vec<Message> {
    let detail = || "detail".to_owned();
    let path = || "/work/a.png".to_owned();
    let errors = [
        EditorError::LockPoisoned { what: "document" },
        EditorError::Io {
            path: path(),
            detail: detail(),
        },
        EditorError::Internal(detail()),
        EditorError::Task(detail()),
        EditorError::ModelChanged,
        EditorError::EmptyPath,
        EditorError::RelativePath(path()),
        EditorError::NotSaved,
        EditorError::NotSavedUseSaveAs,
        EditorError::ExternalChange,
        EditorError::NotAModule,
        EditorError::ExpectedModuleBytes,
        EditorError::ExpectedPngBytes,
        EditorError::NotModuleFile(path()),
        EditorError::NotPngFile(path()),
        EditorError::NotPngData,
        EditorError::NotJsonFile(path()),
        EditorError::MissingHeader("x-path"),
        EditorError::InvalidHeader {
            header: "x-path",
            detail: detail(),
        },
        EditorError::InvalidPercentEncoding(detail()),
        EditorError::CreateFolder {
            path: path(),
            detail: detail(),
        },
        EditorError::TooManySampleFolders(path()),
        EditorError::PsdRead {
            path: path(),
            error: PsdError::NotPsd,
        },
        EditorError::NormalMapsModelOnly,
        EditorError::NoGroupChosen,
        EditorError::UnknownGroup("Body".to_owned()),
        EditorError::EmptyGroup("Body".to_owned()),
        EditorError::ImageNotFound("body.png".to_owned()),
        EditorError::ClaudeModelMissing,
        EditorError::InvalidBaseUrl,
        EditorError::ModelMissing,
        EditorError::ApiKeyRequired,
        EditorError::PlainHttpApiKey,
        EditorError::EmptyApiKey,
        EditorError::CredentialStore(detail()),
        EditorError::MixedDestinations,
        EditorError::SendDisabled,
        EditorError::InvalidUndoLimit { min: 1, max: 1000 },
        EditorError::InvalidTimeLimit { min: 1, max: 120 },
        EditorError::Ai(AiError::Provider(detail())),
        EditorError::Ai(AiError::InvalidProposal(detail())),
        EditorError::Ai(AiError::MissingApiKey(detail())),
        EditorError::Ai(AiError::InvalidRequest(detail())),
        EditorError::Ai(AiError::Template {
            template: RigTemplate::Character,
            reason: detail(),
        }),
        EditorError::Ai(AiError::TextureTooLarge {
            texture: "body".to_owned(),
            width: 20000,
            height: 1024,
            max: 16384,
        }),
        EditorError::Ai(AiError::Api {
            status: 400,
            kind: "invalid_request_error".to_owned(),
            message: detail(),
        }),
        EditorError::Ai(AiError::RateLimited { retry_after: None }),
        EditorError::Ai(AiError::RateLimited {
            retry_after: Some(std::time::Duration::from_secs(30)),
        }),
        EditorError::Ai(AiError::Network(detail())),
        EditorError::Ai(AiError::Refused { category: None }),
        EditorError::Ai(AiError::Refused {
            category: Some("cyber".to_owned()),
        }),
        EditorError::Ai(AiError::Truncated),
        EditorError::Ai(AiError::InvalidResponse(detail())),
        EditorError::Ai(AiError::Cancelled),
        EditorError::Ai(AiError::TimeLimitExceeded {
            limit: std::time::Duration::from_secs(900),
        }),
        EditorError::ProposalModelChanged,
        EditorError::NoPsd,
        EditorError::NotClassified,
        EditorError::NotRegenerable(3),
        EditorError::ProposalDisappeared,
        EditorError::NoPreview,
        EditorError::InvalidPort,
        EditorError::BridgeStart(detail()),
        EditorError::NoMcpBinary,
        EditorError::UnsupportedPlatform,
        EditorError::BundleWrite(detail()),
        EditorError::OpenFailed {
            path: path(),
            detail: detail(),
        },
        EditorError::NotMcpbFile,
        EditorError::NotChosen(path()),
        EditorError::NotChosenForSave(path()),
        EditorError::DialogFailed(detail()),
    ];
    errors.iter().map(Coded::to_message).collect()
}

/// AI のプロバイダのエラーのコード。
fn ai_code(error: &AiError) -> &'static str {
    match error {
        AiError::Provider(_) => "ai.provider",
        AiError::InvalidProposal(_) => "ai.invalid_proposal",
        AiError::MissingApiKey(_) => "ai.missing_api_key",
        AiError::InvalidRequest(_) => "ai.invalid_request",
        AiError::Template { .. } => "ai.template",
        AiError::TextureTooLarge { .. } => "ai.texture_too_large",
        AiError::Api { .. } => "ai.api_error",
        AiError::RateLimited { .. } => "ai.rate_limited",
        AiError::Network(_) => "ai.network",
        AiError::Refused { .. } => "ai.refused",
        AiError::Truncated => "ai.truncated",
        AiError::InvalidResponse(_) => "ai.invalid_response",
        AiError::Cancelled => "ai.cancelled",
        AiError::TimeLimitExceeded { .. } => "ai.time_limit",
    }
}

/// AI のプロバイダのエラーの引数。
fn ai_args(error: &AiError) -> MessageArgs {
    match error {
        AiError::Provider(detail)
        | AiError::InvalidProposal(detail)
        | AiError::MissingApiKey(detail)
        | AiError::InvalidRequest(detail)
        | AiError::Network(detail)
        | AiError::InvalidResponse(detail) => message_args! { "detail" => detail },
        AiError::Api {
            status,
            kind,
            message,
        } => message_args! { "status" => *status, "kind" => kind, "detail" => message },
        AiError::Template { template, reason } => {
            message_args! { "template" => template.as_str(), "detail" => reason }
        }
        AiError::TextureTooLarge {
            texture,
            width,
            height,
            max,
        } => {
            message_args! { "texture" => texture, "width" => *width, "height" => *height, "max" => *max }
        }
        AiError::RateLimited {
            retry_after: Some(after),
        } => message_args! { "retry_after" => ugoe_ai::secs_rounded_up(*after) },
        AiError::Refused {
            category: Some(category),
        } => message_args! { "category" => category },
        // 画面は分で示す (上限は分で選ぶ)
        AiError::TimeLimitExceeded { limit } => {
            message_args! { "minutes" => ugoe_ai::secs_rounded_up(*limit).div_ceil(60) }
        }
        AiError::RateLimited { retry_after: None }
        | AiError::Refused { category: None }
        | AiError::Truncated
        | AiError::Cancelled => message_args! {},
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::collections::BTreeSet;
    use std::time::Duration;

    use super::*;

    #[test]
    fn errors_have_stable_codes_args_and_english_text() {
        let message = EditorError::io(Path::new("/work/a.png"), "disk full").to_message();
        assert_eq!(message.code, "editor.io");
        assert_eq!(
            message.args,
            message_args! { "path" => "/work/a.png", "detail" => "disk full" }
        );
        assert_eq!(message.message, "/work/a.png: disk full");

        let message = EditorError::ExternalChange.to_message();
        assert_eq!(message.code, "editor.external_change");
        assert!(message.args.is_empty());
        assert_eq!(
            message.message,
            "the project was changed outside the editor after it was opened; reload it, or save \
             again to overwrite"
        );
    }

    /// PSD を読めないときは、英語の文を `detail` に、元のエラーを原因 (`psd_read.*`) に載せる (T0571)。
    #[test]
    fn psd_read_carries_the_psd_error_as_the_cause() {
        let error = EditorError::psd_read(
            Path::new("/work/a.psd"),
            PsdError::UnsupportedDepth { depth: 32 },
        );
        let message = error.to_message();
        assert_eq!(message.code, "psd.read_file_failed");
        assert_eq!(
            message.args,
            message_args! {
                "path" => "/work/a.psd",
                "detail" => "unsupported psd feature: 32-bit channels (only 8-bit and 16-bit are supported)",
            }
        );
        let cause = message.cause.expect("the psd error as the cause");
        assert_eq!(cause.code, "psd_read.unsupported_depth");
        assert_eq!(cause.args, message_args! { "depth" => 32u16 });
        // ほかのエラーは原因を持たない。
        assert!(EditorError::EmptyPath.to_message().cause.is_none());
    }

    #[test]
    fn ai_errors_keep_their_kind() {
        let message = EditorError::Ai(AiError::RateLimited {
            retry_after: Some(Duration::from_secs(30)),
        })
        .to_message();
        assert_eq!(message.code, "ai.rate_limited");
        assert_eq!(message.args, message_args! { "retry_after" => 30u64 });
        assert_eq!(message.message, "rate limited (retry after 30s)");

        // 1 秒に満たない待ちを「0 秒」と示さない (文言と引数の両方を切り上げる)
        let message = EditorError::Ai(AiError::RateLimited {
            retry_after: Some(Duration::from_millis(500)),
        })
        .to_message();
        assert_eq!(message.args, message_args! { "retry_after" => 1u64 });
        assert_eq!(message.message, "rate limited (retry after 1s)");

        let message = EditorError::Ai(AiError::TimeLimitExceeded {
            limit: Duration::from_millis(60_500),
        })
        .to_message();
        assert_eq!(message.args, message_args! { "minutes" => 2u64 });

        let message = EditorError::Ai(AiError::Api {
            status: 400,
            kind: "invalid_request_error".into(),
            message: "bad".into(),
        })
        .to_message();
        assert_eq!(message.code, "ai.api_error");
        assert_eq!(
            message.args,
            message_args! { "status" => 400u16, "kind" => "invalid_request_error", "detail" => "bad" }
        );

        // テンプレートで組めない失敗は、どのテンプレートかと足りないものを引数に持つ (T0555)
        let message = EditorError::Ai(AiError::Template {
            template: RigTemplate::Module,
            reason: "nothing to rig".into(),
        })
        .to_message();
        assert_eq!(message.code, "ai.template");
        assert_eq!(
            message.args,
            message_args! { "template" => "module", "detail" => "nothing to rig" }
        );
        assert_eq!(
            message.message,
            "the module template cannot rig this model: nothing to rig"
        );
    }

    /// ソースに書いたコード (`"area.name"` の文字列)。
    fn codes_in_source() -> BTreeSet<&'static str> {
        include_str!("error.rs")
            .split('"')
            .skip(1)
            .step_by(2)
            .filter(|s| {
                [
                    "editor.",
                    "ai.",
                    "mcp.",
                    "psd.",
                    "lighting.",
                    "proposal.",
                    "file.",
                    "dialog.",
                ]
                .iter()
                .any(|a| s.starts_with(a) && s.len() > a.len())
                    && s.chars()
                        .all(|c| c.is_ascii_lowercase() || c == '_' || c == '.')
            })
            .collect()
    }

    /// すべてのコードが `docs/editor.md#エラーと警告のコード` の一覧にある。
    #[test]
    fn every_code_is_documented() {
        let docs = include_str!("../../../../docs/editor.md");
        let codes = codes_in_source();
        assert!(codes.len() > 40, "{}", codes.len());
        for code in codes {
            assert!(
                docs.contains(&format!("`{code}`")),
                "{code} is not in docs/editor.md"
            );
        }
    }

    /// ソースに書いたすべてのコードに見本があり (`message_samples`)、見本にソースに無いコードが無い。
    #[test]
    fn every_code_has_a_sample() {
        let samples = message_samples();
        let sampled: BTreeSet<&str> = samples.iter().map(|m| m.code.as_str()).collect();
        assert_eq!(codes_in_source(), sampled);
    }

    #[test]
    fn codes_are_lowercase_and_dotted() {
        let errors = [
            EditorError::LockPoisoned { what: "document" },
            EditorError::EmptyPath,
            EditorError::RelativePath("a".into()),
            EditorError::NotSavedUseSaveAs,
            EditorError::MissingHeader("x-path"),
            EditorError::NormalMapsModelOnly,
            EditorError::SendDisabled,
            EditorError::Ai(AiError::Truncated),
            EditorError::NotRegenerable(3),
            EditorError::InvalidPort,
        ];
        for error in errors {
            let code = error.code();
            let (area, name) = code.split_once('.').unwrap();
            for part in [area, name] {
                assert!(!part.is_empty(), "{code}");
                assert!(
                    part.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                    "{code}"
                );
            }
        }
    }
}
