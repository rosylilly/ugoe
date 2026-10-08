//! 外部 API の承認を待つクライアントを、操作ウィンドウが隠れていても OS の側で知らせる (T0856。docs/studio.md#外部-api)。
//!
//! Tauri の `request_user_attention` で操作ウィンドウの注意を引く (Windows はタスクバーの点滅、macOS は Dock のアイコンの跳ね、
//! Linux はウィンドウマネージャの「注意を要する」の印)。ここはいつ引き・いつ止めるかだけを決める (ウィンドウには触らない)。

use std::collections::BTreeSet;

/// 注意を引く・止める。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request {
    Start,
    Stop,
}

/// 承認の待ちと、操作ウィンドウが前面にあるか・注意を引いているか。
///
/// 前面にあるかはウィンドウの `Focused` の出来事で覚える (承認の待ちが変わるたびにウィンドウに尋ねると、メインのスレッドを
/// 待つことになり、ロックを握った呼び手と行き違いうるため)。最小化や裏に回したときも、フォーカスを失うので前面に無いとみなす。
#[derive(Debug, Default)]
pub struct ApprovalAttention {
    /// 前に見た、承認を待つクライアントの ID。
    known: BTreeSet<String>,
    /// 操作ウィンドウがフォーカスを持っている。
    focused: bool,
    /// 注意を引いている。
    requested: bool,
}

impl ApprovalAttention {
    /// 承認を待つクライアントの一覧が変わったかもしれない (操作 API の様子を送るたびに呼ぶ)。
    ///
    /// 新しいクライアントが現れ、操作ウィンドウが前面に無ければ注意を引く。待ちが無くなったら
    /// (承認・拒否・切断・操作 API を切った) 止める。前面にあるときは、操作ウィンドウの上の知らせで足りるので引かない。
    pub fn pending<'a>(&mut self, ids: impl IntoIterator<Item = &'a str>) -> Option<Request> {
        let ids: BTreeSet<String> = ids.into_iter().map(str::to_owned).collect();
        let arrived = ids.iter().any(|id| !self.known.contains(id));
        self.known = ids;
        if self.known.is_empty() {
            return self.stop();
        }
        if arrived && !self.focused {
            self.requested = true;
            return Some(Request::Start);
        }
        None
    }

    /// 操作ウィンドウがフォーカスを得た・失った。得たら、利用者は上の知らせを見られるので止める
    /// (「後で」も操作ウィンドウを前に出してから押すので、ここで止まる)。
    pub fn focus(&mut self, focused: bool) -> Option<Request> {
        self.focused = focused;
        if focused { self.stop() } else { None }
    }

    /// 注意を引いているか (テスト)。
    #[cfg(test)]
    pub fn requested(&self) -> bool {
        self.requested
    }

    fn stop(&mut self) -> Option<Request> {
        std::mem::take(&mut self.requested).then_some(Request::Stop)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_client_draws_attention_only_when_the_window_is_not_in_front() {
        let mut attention = ApprovalAttention::default();
        attention.focus(true);
        assert_eq!(attention.pending(["a"]), None);
        // 最小化した・ほかのアプリを前に出した。
        assert_eq!(attention.focus(false), None);
        assert_eq!(attention.pending(["a", "b"]), Some(Request::Start));

        // 起動したばかりでフォーカスの出来事がまだ無い: 前面に無いとみなす。
        let mut attention = ApprovalAttention::default();
        assert_eq!(attention.pending(["a"]), Some(Request::Start));
    }

    #[test]
    fn the_same_clients_do_not_draw_attention_again() {
        let mut attention = ApprovalAttention::default();
        assert_eq!(attention.pending(["a"]), Some(Request::Start));
        // 接続の数などが変わって様子を送り直しただけ。
        assert_eq!(attention.pending(["a"]), None);
        // 前面で見て「後で」を押し、また隠した: 前からのクライアントでは引き直さない。
        assert_eq!(attention.focus(true), Some(Request::Stop));
        assert_eq!(attention.focus(false), None);
        assert_eq!(attention.pending(["a"]), None);
        // 別のクライアントが来たら引く。
        assert_eq!(attention.pending(["a", "b"]), Some(Request::Start));
    }

    #[test]
    fn attention_stops_when_nothing_waits_any_more() {
        let mut attention = ApprovalAttention::default();
        assert_eq!(attention.pending(["a", "b"]), Some(Request::Start));
        // 1 つが切断しても、まだ待っているものがある。
        assert_eq!(attention.pending(["b"]), None);
        // 承認・拒否・切断・操作 API を切ったことで、最後の待ちが無くなった。
        assert_eq!(attention.pending([]), Some(Request::Stop));
        // 引いていなければ止めない。
        assert_eq!(attention.pending([]), None);
    }

    #[test]
    fn attention_stops_when_the_window_comes_to_the_front() {
        let mut attention = ApprovalAttention::default();
        assert_eq!(attention.focus(true), None);
        assert_eq!(attention.focus(false), None);
        assert_eq!(attention.pending(["a"]), Some(Request::Start));
        assert_eq!(attention.focus(true), Some(Request::Stop));
        assert_eq!(attention.focus(true), None);
        // 止めた後で待ちが無くなっても、止め直さない。
        assert_eq!(attention.pending([]), None);
    }
}
