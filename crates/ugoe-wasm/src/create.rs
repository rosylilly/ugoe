//! `WebPuppet.create` の失敗のうち、コードを持つもの (T0475)。
//!
//! JS では `name` が `"CreateError"` で `code` を持つ `Error` になる (読み込みの `LoadError`・描画の `RenderError` と同じ形)。
//! どれも GPU やブラウザの都合で作れなかったもので、新しい canvas で (時間を置いて) 作り直せば作れることがある。
//! 引数の誤り (知らない `backend`) と、起きないはずの失敗 (canvas のサーフェスが形式を 1 つも持たないなど) は、
//! 組み込みの誤りなのでコードを持たない `Error` のまま。

use crate::load::ErrorInfo;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CreateError {
    /// 求めたバックエンド (`"webgpu"`) がこのブラウザに無い。
    #[error("WebGPU is not available in this browser")]
    BackendUnavailable,
    /// canvas から GPU のコンテキストを取れない (`getContext` が `null` を返した・取れたコンテキストが既に失われている。
    /// WebGL2 のコンテキストの数の上限・GPU プロセスの不調・canvas を別の種類のコンテキストで使っているなど)。
    #[error("the canvas could not provide a GPU context: {0}")]
    ContextUnavailable(String),
    /// GPU のアダプタを取れない。
    #[error("no GPU adapter is available: {0}")]
    AdapterUnavailable(String),
    /// GPU のデバイスを作れない。
    #[error("the GPU device could not be created: {0}")]
    DeviceUnavailable(String),
}

impl CreateError {
    /// JS の `Error` の `name` (種類は `code`。[`CreateError::info`])。
    pub const NAME: &'static str = "CreateError";

    /// JS に渡す `code` と補足の値。
    pub fn info(&self) -> ErrorInfo {
        let code = match self {
            Self::BackendUnavailable => "backend_unavailable",
            Self::ContextUnavailable(_) => "context_unavailable",
            Self::AdapterUnavailable(_) => "adapter_unavailable",
            Self::DeviceUnavailable(_) => "device_unavailable",
        };
        ErrorInfo {
            code,
            fields: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// コンテキストを取れないときは `CreateError` で `code` が `context_unavailable` (T0475)。
    #[test]
    fn context_unavailable_is_a_coded_create_error() {
        assert_eq!(CreateError::NAME, "CreateError");
        let error = CreateError::ContextUnavailable("the WebGL2 context is lost".into());
        assert_eq!(
            error.info(),
            ErrorInfo {
                code: "context_unavailable",
                fields: vec![],
            }
        );
        assert_eq!(
            error.to_string(),
            "the canvas could not provide a GPU context: the WebGL2 context is lost"
        );
    }
}
