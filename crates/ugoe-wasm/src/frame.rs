//! 描画 (`render`) と読み戻し (`read_pixels`) の失敗のうち、コードを持つもの (T0465)。
//!
//! JS では `name` が `"RenderError"` で `code` を持つ `Error` になる (読み込みの `LoadError`・着せ替えの `DressUpError` と同じ形)。
//! ほかの描画の失敗 (canvas のテクスチャを取れない・レンダラの失敗・読み戻しの時間切れなど) は、組み込みの誤りか
//! 一時的なもので、利用者に見せる文ではないので、コードを持たない `Error` のまま。

use crate::load::ErrorInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FrameError {
    /// GPU のデバイスを失った (`WebPuppet::is_lost`)。新しい canvas で作り直す必要がある。
    #[error("the GPU device was lost")]
    DeviceLost,
}

impl FrameError {
    /// JS の `Error` の `name` (種類は `code`。[`FrameError::info`])。
    pub const NAME: &'static str = "RenderError";

    /// JS に渡す `code` と補足の値。
    pub fn info(&self) -> ErrorInfo {
        let code = match self {
            Self::DeviceLost => "device_lost",
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

    /// GPU を失った後の描画・読み戻しは `RenderError` で `code` が `device_lost` (T0465)。
    #[test]
    fn device_lost_is_a_coded_render_error() {
        assert_eq!(FrameError::NAME, "RenderError");
        assert_eq!(
            FrameError::DeviceLost.info(),
            ErrorInfo {
                code: "device_lost",
                fields: vec![],
            }
        );
        // 文は T0465 の前の例外 (コード無しの `JsError`) と同じ。
        assert_eq!(
            FrameError::DeviceLost.to_string(),
            "the GPU device was lost"
        );
    }
}
