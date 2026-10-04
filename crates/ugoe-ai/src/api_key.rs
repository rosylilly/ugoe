//! プロバイダと深度推定器で共通の API キー。
//!
//! キーはログ・エラー・録画に出さない (`Debug` は伏せ字。エラーの文言から取り除くのは送る側の役目)。
//! 設計: `docs/ai-integration.md#セキュリティとプライバシー`。

use std::fmt;

use crate::AiError;

/// API キー。値は `Debug` やエラーに出ない。
///
/// Claude ([`crate::claude::ClaudeProvider`])・OpenAI 互換 ([`crate::openai::OpenAiProvider`])・
/// 深度推定 ([`crate::depth::HttpDepthEstimator`]) で共通。OS の資格情報の保存先 (キーチェーンなど)
/// から読むのはホスト (エディタ) の役目で、読んだ値を [`ApiKey::new`] に渡す。
#[derive(Clone)]
pub struct ApiKey(String);

impl ApiKey {
    /// 前後の空白を除いた値を持つ。空なら [`AiError::MissingApiKey`]。
    pub fn new(key: impl Into<String>) -> Result<Self, AiError> {
        let key = key.into().trim().to_owned();
        if key.is_empty() {
            return Err(AiError::MissingApiKey("the API key is empty".into()));
        }
        Ok(Self(key))
    }

    /// 環境変数 `name` から読む ([`crate::claude::API_KEY_ENV`] の `ANTHROPIC_API_KEY`、
    /// [`crate::openai::API_KEY_ENV`] の `OPENAI_API_KEY` など)。
    pub fn from_env(name: &str) -> Result<Self, AiError> {
        match std::env::var(name) {
            Ok(v) => Self::new(v).map_err(|_| AiError::MissingApiKey(format!("{name} is empty"))),
            Err(_) => Err(AiError::MissingApiKey(format!("{name} is not set"))),
        }
    }

    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiKey(<redacted>)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_and_hides_value() {
        assert!(matches!(ApiKey::new("  "), Err(AiError::MissingApiKey(_))));
        let key = ApiKey::new(" sk-ant-hidden\n").unwrap();
        assert_eq!(key.expose(), "sk-ant-hidden");
        assert_eq!(format!("{key:?}"), "ApiKey(<redacted>)");
    }

    #[test]
    fn missing_env_var_names_the_variable() {
        let e = ApiKey::from_env("UGOE_AI_TEST_SURELY_UNSET_KEY").unwrap_err();
        assert!(
            matches!(&e, AiError::MissingApiKey(m) if m == "UGOE_AI_TEST_SURELY_UNSET_KEY is not set"),
            "{e:?}"
        );
    }
}
