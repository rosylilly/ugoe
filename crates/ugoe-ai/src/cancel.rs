//! 提案の取り消しと、時間の合計の上限 (`docs/ai-integration.md#取り消しと時間の上限`)。
//!
//! [`CancelToken`] を呼び出し側 (エディタ) が作ってプロバイダに渡し ([`crate::claude::ClaudeProvider::with_cancel`]
//! など)、別のスレッドから [`CancelToken::cancel`] で止める。プロバイダは送る前・再試行の待ちの間・通信の間
//! (応答を待つのをやめる) に印を確かめ、[`AiError::Cancelled`] を返す。時間の上限を持たせると
//! ([`CancelToken::with_time_limit`])、作ってからの時間が上限を超えたところで [`AiError::TimeLimitExceeded`] を返す。
//! 1 回の提案 (再試行・問い直しを含む) に 1 つの印を使う。

use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::{Duration, Instant};

use crate::AiError;

/// 取り消しの印と、時間の合計の上限。複製は同じ印を共有する (どれかで取り消せば、すべてが取り消される)。
///
/// 既定 ([`CancelToken::new`]) は上限を持たず、取り消さない限り止まらない。
#[derive(Debug, Clone, Default)]
pub struct CancelToken {
    inner: Arc<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    cancelled: Mutex<bool>,
    wake: Condvar,
    /// 期限と、上限そのもの (エラーに示す)。
    deadline: Option<(Instant, Duration)>,
}

impl CancelToken {
    /// 上限のない印。
    pub fn new() -> Self {
        Self::default()
    }

    /// 今から `limit` までを上限とする印。`limit` が `Instant` に収まらないほど長ければ上限なし。
    pub fn with_time_limit(limit: Duration) -> Self {
        let deadline = Instant::now().checked_add(limit).map(|at| (at, limit));
        Self {
            inner: Arc::new(Inner {
                deadline,
                ..Inner::default()
            }),
        }
    }

    /// 取り消す。待っているプロバイダはすぐに起きて [`AiError::Cancelled`] を返す。
    pub fn cancel(&self) {
        *self.lock() = true;
        self.inner.wake.notify_all();
    }

    pub fn is_cancelled(&self) -> bool {
        *self.lock()
    }

    /// 時間の上限。なければ `None`。
    pub fn time_limit(&self) -> Option<Duration> {
        self.inner.deadline.map(|(_, limit)| limit)
    }

    /// 上限までの残りの時間。上限がなければ `None`、過ぎていれば 0。
    pub fn remaining(&self) -> Option<Duration> {
        self.inner
            .deadline
            .map(|(at, _)| at.saturating_duration_since(Instant::now()))
    }

    /// 続けてよいか。取り消されていれば [`AiError::Cancelled`]、上限を過ぎていれば
    /// [`AiError::TimeLimitExceeded`]。
    pub fn check(&self) -> Result<(), AiError> {
        if self.is_cancelled() {
            return Err(AiError::Cancelled);
        }
        match self.inner.deadline {
            Some((at, limit)) if Instant::now() >= at => Err(AiError::TimeLimitExceeded { limit }),
            _ => Ok(()),
        }
    }

    /// 上限を過ぎたことにするエラー (待ち時間が残りを超えるので待たずに諦めるとき)。
    pub(crate) fn exceeded(&self) -> AiError {
        AiError::TimeLimitExceeded {
            limit: self.time_limit().unwrap_or_default(),
        }
    }

    /// `duration` だけ待つ。取り消されたらすぐに起きる。待った後に [`Self::check`] を返す。
    pub(crate) fn wait(&self, duration: Duration) -> Result<(), AiError> {
        let guard = self.lock();
        let (guard, _) = self
            .inner
            .wake
            .wait_timeout_while(guard, duration, |cancelled| !*cancelled)
            .unwrap_or_else(PoisonError::into_inner);
        drop(guard);
        self.check()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, bool> {
        self.inner
            .cancelled
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clones_share_the_flag() {
        let token = CancelToken::new();
        let other = token.clone();
        assert!(token.check().is_ok());
        other.cancel();
        assert!(token.is_cancelled());
        assert!(matches!(token.check(), Err(AiError::Cancelled)));
    }

    #[test]
    fn a_zero_time_limit_is_already_exceeded() {
        let token = CancelToken::with_time_limit(Duration::ZERO);
        assert_eq!(token.remaining(), Some(Duration::ZERO));
        assert!(matches!(
            token.check(),
            Err(AiError::TimeLimitExceeded { limit }) if limit == Duration::ZERO
        ));
        // 取り消しは上限より先に示す
        token.cancel();
        assert!(matches!(token.check(), Err(AiError::Cancelled)));
    }

    #[test]
    fn a_huge_time_limit_means_no_limit() {
        let token = CancelToken::with_time_limit(Duration::MAX);
        assert_eq!(token.time_limit(), None);
        assert!(token.check().is_ok());
    }

    #[test]
    fn cancel_wakes_a_waiting_thread() {
        let token = CancelToken::new();
        let waiter = {
            let token = token.clone();
            std::thread::spawn(move || token.wait(Duration::from_secs(600)))
        };
        token.cancel();
        // 600 秒を待たずに起きる (起きなければテストが止まる)
        assert!(matches!(waiter.join().unwrap(), Err(AiError::Cancelled)));
    }
}
