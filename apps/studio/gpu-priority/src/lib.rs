//! スタジオの GPU の優先度 (ADR 0034、`docs/studio.md#gpu-の優先度`、T0861)。
//!
//! Windows の GPU のスケジューラ (WDDM) の、プロセスごとの優先度のクラスを変える。スタジオの GPU の処理は 2 つのプロセスにある:
//!
//! - スタジオのプロセス (仮想カメラのネイティブの wgpu): `set_current_process_priority`。
//! - WebView2 の GPU のプロセス (出力ウィンドウと操作ウィンドウの描画。WebView2 の環境ごとに 1 つ): `webview2` で番号を知り、
//!   `set_process_priority` で変える。
//!
//! ほかの OS には、普通のアプリがプロセスの GPU の優先度を変える口が無いので、Windows の外では OS を呼ぶ関数を出さない
//! ([`GpuPriority`] と [`Error`] だけ)。
//!
//! **`unsafe` の例外 (ADR 0034):** ワークスペースで `unsafe` を許すクレートの 1 つ。`unsafe` は Windows の呼び出し
//! (`D3DKMTSetProcessSchedulingPriorityClass`・`OpenProcess`・WebView2 の COM) に閉じ込め、外へは安全な関数だけを出す。どのブロックにも
//! 成り立たせている前提を `// SAFETY:` で書く (clippy の `undocumented_unsafe_blocks` で検査する)。

#[cfg(windows)]
mod win;

#[cfg(windows)]
pub use win::{set_current_process_priority, set_process_priority, webview2};

/// GPU の優先度 (スタジオが選べるもの)。
///
/// 上げる (`ABOVE_NORMAL` 以上) には管理者の権限が要るので、選べるのは「通常」と、それより 1 つ低い `BELOW_NORMAL` だけにする。
/// `IDLE` は、ほかのプロセスが GPU を使い続ける間はほとんど描けなくなり、出力が止まって見えるので選ばせない。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GpuPriority {
    /// 通常 (`D3DKMT_SCHEDULINGPRIORITYCLASS_NORMAL`。プロセスを作ったときの値)。
    #[default]
    Normal,
    /// 通常より低い (`D3DKMT_SCHEDULINGPRIORITYCLASS_BELOW_NORMAL`)。ゲームや配信ソフトが GPU を使い切っている間は、描画が後回しになる。
    BelowNormal,
}

impl GpuPriority {
    /// `D3DKMT_SCHEDULINGPRIORITYCLASS` の値 (d3dkmthk.h の列挙の順: `IDLE` = 0、`BELOW_NORMAL` = 1、`NORMAL` = 2)。
    /// 出典: <https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/d3dkmthk/ne-d3dkmthk-_d3dkmt_schedulingpriorityclass>
    pub fn class(self) -> i32 {
        match self {
            Self::Normal => 2,
            Self::BelowNormal => 1,
        }
    }
}

/// 優先度を変えられなかった理由。スタジオは種類ごとにエラーのコードを選ぶ (ADR 0029)。
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// プロセスを開けない・優先度を変える権限が無い (`E_ACCESSDENIED`・`STATUS_ACCESS_DENIED`・`STATUS_PRIVILEGE_NOT_HELD`)。
    #[error("access to the process was denied")]
    AccessDenied,
    /// プロセスが無い (終わった)。
    #[error("the process was not found")]
    ProcessNotFound,
    /// WebView2 Runtime が古い (`ICoreWebView2Environment8`。WebView2 Runtime 1.0.1108.44 から) ので、GPU のプロセスを知れない。
    #[error("the WebView2 Runtime is too old to list its processes")]
    WebView2TooOld,
    /// そのほかの失敗 (`HRESULT`・`NTSTATUS` の英語の説明)。
    #[error("{0}")]
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classes_follow_the_d3dkmt_enum() {
        assert_eq!(GpuPriority::default(), GpuPriority::Normal);
        assert_eq!(GpuPriority::Normal.class(), 2);
        assert_eq!(GpuPriority::BelowNormal.class(), 1);
    }
}
