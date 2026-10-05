//! AI のプロバイダの選択 (Claude / OpenAI 互換) と、その保存。ADR 0022・`docs/editor.md#ai-の設定と送信の同意`。
//!
//! - 選んだプロバイダ・モデル・ベースの URL はエディタ全体の設定で、アプリの設定のディレクトリの
//!   `ai-provider.json` に置く (プロジェクトには書かない)。API キーは含めない。
//! - 既定は Claude (`claude-opus-5-5`)。ファイルがなければ既定のまま (既存の利用者の設定は変わらない)。
//! - API キーの置き場 ([`KeySlot`]) はプロバイダごと。OpenAI 互換の API は送り先のホストごとに分ける
//!   (OpenAI のキーを OpenRouter などの別の送り先に送らないため)。
//! - 1 回の提案にかける時間の合計の上限 ([`ProviderSettings::time_limit_minutes`]。再試行と問い直しを含む) も
//!   ここに置く (`docs/ai-integration.md#取り消しと時間の上限`)。

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use ugoe_ai::claude::{self, ClaudeConfig, ClaudeProvider, Effort};
use ugoe_ai::openai::{self, OpenAiConfig, OpenAiProvider, StructuredOutput};
use ugoe_ai::transport::{is_loopback_url, url_host};
use ugoe_ai::{ApiKey, CancelToken, Provider};

use crate::error::EditorError;

/// 設定のファイルの名前 (アプリの設定のディレクトリの中)。
const SETTINGS_FILE: &str = "ai-provider.json";
/// OpenAI 互換の API の既定のモデル。
pub const DEFAULT_OPENAI_MODEL: &str = "gpt-6-sol";
/// 1 回の提案にかける時間の合計の上限の既定 (分)。1 回のリクエストの上限 (10 分) に再試行の待ちを足した程度。
pub const DEFAULT_TIME_LIMIT_MINUTES: u32 = 15;
/// 時間の上限に選べる範囲 (分)。
pub const TIME_LIMIT_RANGE_MINUTES: std::ops::RangeInclusive<u32> = 1..=120;

/// 使うプロバイダ。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    /// Anthropic の Claude API。
    #[default]
    Claude,
    /// OpenAI 互換の API (OpenAI・OpenRouter・Gemini・Ollama・LM Studio・vLLM など)。
    OpenaiCompatible,
}

/// OpenAI 互換の API で、答えを JSON にさせる方法 (`ugoe_ai::openai::StructuredOutput`)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum OutputMode {
    /// 構造化出力 (`response_format` の JSON Schema。`strict`)。
    #[default]
    JsonSchema,
    /// JSON モード。スキーマはプロンプトに書き、答えを検証して通らなければ問い直す。
    JsonObject,
    /// `response_format` を送らない。スキーマはプロンプトに書き、答えを検証して問い直す。
    Prompt,
}

impl From<OutputMode> for StructuredOutput {
    fn from(mode: OutputMode) -> Self {
        match mode {
            OutputMode::JsonSchema => Self::JsonSchema,
            OutputMode::JsonObject => Self::JsonObject,
            OutputMode::Prompt => Self::Prompt,
        }
    }
}

/// Claude の設定。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default)]
pub struct ClaudeSettings {
    /// モデルの ID。
    pub model: String,
}

impl Default for ClaudeSettings {
    fn default() -> Self {
        Self {
            model: claude::DEFAULT_MODEL.to_owned(),
        }
    }
}

/// OpenAI 互換の API の設定。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default)]
pub struct OpenAiSettings {
    /// `/chat/completions` の手前までの URL (例: `https://api.openai.com/v1`・`http://localhost:11434/v1`)。
    pub base_url: String,
    /// モデルの ID。
    pub model: String,
    pub output: OutputMode,
}

impl Default for OpenAiSettings {
    fn default() -> Self {
        Self {
            base_url: openai::OPENAI_BASE_URL.to_owned(),
            model: DEFAULT_OPENAI_MODEL.to_owned(),
            output: OutputMode::default(),
        }
    }
}

/// AI のプロバイダの設定 (`ai-provider.json`)。API キーは含めない。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default)]
pub struct ProviderSettings {
    pub provider: ProviderKind,
    pub claude: ClaudeSettings,
    pub openai: OpenAiSettings,
    /// 1 回の提案 (分類・リグの提案・接続の確認) にかける時間の合計の上限 (分)。再試行の待ちと問い直しを含む。
    /// 超えたら止めて `ai.time_limit` のエラーにする。範囲は [`TIME_LIMIT_RANGE_MINUTES`]。
    pub time_limit_minutes: u32,
}

impl Default for ProviderSettings {
    fn default() -> Self {
        Self {
            provider: ProviderKind::default(),
            claude: ClaudeSettings::default(),
            openai: OpenAiSettings::default(),
            time_limit_minutes: DEFAULT_TIME_LIMIT_MINUTES,
        }
    }
}

/// プロバイダを作る目的。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// 接続の確認 (小さなリクエスト。考える量を減らす)。
    ConnectionTest,
    /// 分類・リギングの提案。
    Proposal,
}

/// API キーの置き場。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeySlot {
    /// 資格情報の保存先でのアカウント名。
    pub account: String,
    /// 保存先にキーがないときに読む環境変数。
    pub env: Option<&'static str>,
    /// キーがないと送れないか (手元のサーバなどはキーが要らない)。
    pub required: bool,
}

impl ProviderSettings {
    /// 保存する前の検査。
    pub fn check(&self) -> Result<(), EditorError> {
        if !TIME_LIMIT_RANGE_MINUTES.contains(&self.time_limit_minutes) {
            return Err(EditorError::InvalidTimeLimit {
                min: *TIME_LIMIT_RANGE_MINUTES.start(),
                max: *TIME_LIMIT_RANGE_MINUTES.end(),
            });
        }
        match self.provider {
            ProviderKind::Claude if self.claude.model.trim().is_empty() => {
                Err(EditorError::ClaudeModelMissing)
            }
            ProviderKind::Claude => Ok(()),
            ProviderKind::OpenaiCompatible => {
                let url = self.openai.base_url.trim();
                if !(url.starts_with("https://") || url.starts_with("http://"))
                    || url_host(url).is_empty()
                {
                    return Err(EditorError::InvalidBaseUrl);
                }
                if self.openai.model.trim().is_empty() {
                    return Err(EditorError::ModelMissing);
                }
                Ok(())
            }
        }
    }

    /// 送り先がこの計算機 (ループバック) か。外部に出ないので同意を求めない。
    pub fn is_local(&self) -> bool {
        match self.provider {
            ProviderKind::Claude => false,
            ProviderKind::OpenaiCompatible => is_loopback_url(&self.openai.base_url),
        }
    }

    /// 利用者に示すサービスの名前。
    pub fn service(&self) -> String {
        match self.provider {
            ProviderKind::Claude => "Anthropic Claude API".into(),
            ProviderKind::OpenaiCompatible => openai::service_name(&self.openai.base_url),
        }
    }

    /// 使うモデル。
    pub fn model(&self) -> &str {
        match self.provider {
            ProviderKind::Claude => &self.claude.model,
            ProviderKind::OpenaiCompatible => &self.openai.model,
        }
    }

    /// API キーの置き場。Claude は前からの名前 (`anthropic-api-key`) のまま。
    pub fn key_slot(&self) -> KeySlot {
        match self.provider {
            ProviderKind::Claude => KeySlot {
                account: "anthropic-api-key".into(),
                env: Some(claude::API_KEY_ENV),
                required: true,
            },
            ProviderKind::OpenaiCompatible => {
                let host = url_host(&self.openai.base_url);
                KeySlot {
                    // 環境変数 `OPENAI_API_KEY` は OpenAI に送るときだけ使う
                    env: (host == "api.openai.com").then_some(openai::API_KEY_ENV),
                    account: format!("openai-compatible:{host}"),
                    required: false,
                }
            }
        }
    }

    /// 答えが検証を通らなかったときに、プロバイダが同じ内容を送り直す回数の上限 (1 リクエストあたり)。
    pub fn json_retries(&self) -> usize {
        match self.provider {
            ProviderKind::Claude => 0,
            ProviderKind::OpenaiCompatible => {
                StructuredOutput::from(self.openai.output).default_json_retries() as usize
            }
        }
    }

    /// 1 回の提案にかける時間の合計の上限。
    pub fn time_limit(&self) -> Duration {
        Duration::from_secs(u64::from(self.time_limit_minutes) * 60)
    }

    fn claude_config(&self, purpose: Purpose) -> ClaudeConfig {
        let config = ClaudeConfig {
            model: self.claude.model.trim().to_owned(),
            ..ClaudeConfig::default()
        };
        match purpose {
            Purpose::ConnectionTest => ClaudeConfig {
                max_tokens: 1024,
                effort: Some(Effort::Low),
                ..config
            },
            Purpose::Proposal => config,
        }
    }

    fn openai_config(&self) -> OpenAiConfig {
        OpenAiConfig::new(self.openai.base_url.trim(), self.openai.model.trim())
            .with_structured_output(self.openai.output.into())
    }

    /// プロバイダを作る。`key` は [`Self::key_slot`] の置き場から読んだもの。`cancel` は取り消しの印と時間の上限
    /// ([`crate::ai::AiJobs::start`] が作る)。
    pub fn build(
        &self,
        key: Option<ApiKey>,
        purpose: Purpose,
        cancel: CancelToken,
    ) -> Result<Box<dyn Provider>, EditorError> {
        self.check()?;
        match self.provider {
            ProviderKind::Claude => {
                let key = key.ok_or(EditorError::ApiKeyRequired)?;
                Ok(Box::new(
                    ClaudeProvider::new(self.claude_config(purpose), key).with_cancel(cancel),
                ))
            }
            ProviderKind::OpenaiCompatible => {
                let config = self.openai_config();
                // 暗号化されない通信で、キーを外部に送らない
                if key.is_some() && !config.is_local() && !config.base_url.starts_with("https://") {
                    return Err(EditorError::PlainHttpApiKey);
                }
                Ok(Box::new(
                    OpenAiProvider::new(config, key).with_cancel(cancel),
                ))
            }
        }
    }
}

/// プロバイダの設定の置き場。起動時に [`ProviderStore::load`] でファイルを読み、変えるたびに書く。
#[derive(Debug, Default)]
pub struct ProviderStore {
    path: OnceLock<PathBuf>,
    value: Mutex<ProviderSettings>,
}

impl ProviderStore {
    /// `dir` (アプリの設定のディレクトリ) のファイルを読む。なければ既定のまま。
    /// 読めなければ既定のままエラーを返す (起動は止めない)。
    pub fn load(&self, dir: PathBuf) -> Result<(), EditorError> {
        let path = dir.join(SETTINGS_FILE);
        let _ = self.path.set(path.clone());
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(EditorError::io(&path, e)),
        };
        let settings: ProviderSettings =
            serde_json::from_str(&text).map_err(|e| EditorError::io(&path, e))?;
        *self.lock() = settings;
        Ok(())
    }

    pub fn get(&self) -> ProviderSettings {
        self.lock().clone()
    }

    /// 検査してから置き換え、ファイルに書く (ファイルの場所が決まっていれば)。
    pub fn set(&self, settings: ProviderSettings) -> Result<(), EditorError> {
        settings.check()?;
        if let Some(path) = self.path.get() {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).map_err(|e| EditorError::io(dir, e))?;
            }
            let mut text = serde_json::to_string_pretty(&settings)
                .map_err(|e| EditorError::Internal(e.to_string()))?;
            text.push('\n');
            ugoe_fs::write_atomic(path, text.as_bytes()).map_err(|e| EditorError::io(path, e))?;
        }
        *self.lock() = settings;
        Ok(())
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, ProviderSettings> {
        self.value
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}
