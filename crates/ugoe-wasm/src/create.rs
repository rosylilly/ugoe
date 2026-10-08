//! `WebPuppet.create` の失敗のうち、コードを持つもの (T0475) と、引数 `powerPreference` の読み取り (T0499)。
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

/// `WebPuppet.create` の `powerPreference` (GPU の電力の希望。T0499) を読む。省くと `"default"` (希望なし)。
///
/// 知らない文字列は組み込みの誤りなので、[`CreateError`] ではなくコードを持たない `Error` にする (知らない `backend` と同じ)。
/// どれも希望で、効くかはブラウザと OS が決める (README の「GPU の電力の希望」)。
pub fn power_preference(name: Option<&str>) -> Result<wgpu::PowerPreference, String> {
    match name.unwrap_or("default") {
        "default" => Ok(wgpu::PowerPreference::None),
        "low-power" => Ok(wgpu::PowerPreference::LowPower),
        "high-performance" => Ok(wgpu::PowerPreference::HighPerformance),
        other => Err(format!(
            "unknown powerPreference `{other}` (expected default, low-power or high-performance)"
        )),
    }
}

/// WebGL の `getContext` の属性 `powerPreference` の値 (WebGPU の `GPUPowerPreference` と違い、希望なしは `"default"`)。
pub fn webgl_power_preference(preference: wgpu::PowerPreference) -> &'static str {
    match preference {
        wgpu::PowerPreference::None => "default",
        wgpu::PowerPreference::LowPower => "low-power",
        wgpu::PowerPreference::HighPerformance => "high-performance",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 3 つの名前を読み、省けば希望なし、ほかは誤り。WebGL の属性には同じ名前で渡す (T0499)。
    #[test]
    fn power_preference_names() {
        for (name, preference) in [
            (None, wgpu::PowerPreference::None),
            (Some("default"), wgpu::PowerPreference::None),
            (Some("low-power"), wgpu::PowerPreference::LowPower),
            (
                Some("high-performance"),
                wgpu::PowerPreference::HighPerformance,
            ),
        ] {
            assert_eq!(power_preference(name), Ok(preference), "{name:?}");
            assert_eq!(
                webgl_power_preference(preference),
                name.unwrap_or("default")
            );
        }
        for name in ["", "high_performance", "High-Performance", "none"] {
            let error = power_preference(Some(name)).unwrap_err();
            assert!(error.contains("unknown powerPreference"), "{error}");
        }
    }

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
