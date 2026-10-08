//! GPU の優先度を当てる先と、当てられなかった理由 (T0861。docs/studio.md#gpu-の優先度)。
//!
//! Windows でだけ使う。当てる先はスタジオのプロセス (仮想カメラ) と WebView2 の GPU のプロセス (出力ウィンドウと操作ウィンドウ)。
//! OS を呼ぶのは `ugoe-studio-gpu-priority` (unsafe の例外のクレート。ADR 0034) で、ここは「どこに当てるか」と「当てられなかったことを
//! どのコードで出すか」だけを持つ (OS を呼ぶ口を引数で受けるので、Windows の外でもテストできる)。

use ugoe_studio_gpu_priority::{Error, GpuPriority};

use crate::error::CommandError;

/// スタジオのプロセスの名前 (エラーの `process` に載せる)。
const STUDIO_PROCESS: &str = "ugoe-studio";

/// 優先度を当てる口 (Windows では `ugoe-studio-gpu-priority`、テストでは偽物)。
pub trait ApplyPriority: FnMut(Target, GpuPriority) -> Result<(), Error> {}

impl<F: FnMut(Target, GpuPriority) -> Result<(), Error>> ApplyPriority for F {}

/// 当てる先。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// スタジオのプロセス (仮想カメラのネイティブの wgpu)。
    Studio,
    /// WebView2 の GPU のプロセス (番号)。
    WebView(u32),
}

impl Target {
    /// エラーに載せるプロセスの名前。
    fn process(self) -> String {
        match self {
            Self::Studio => STUDIO_PROCESS.to_owned(),
            Self::WebView(pid) => format!("msedgewebview2 (GPU, {pid})"),
        }
    }
}

/// WebView2 の GPU のプロセスの知り方の様子。
#[derive(Debug, Clone, PartialEq)]
enum WebView {
    /// まだ調べていない (ウィンドウを作った直後)。
    Unknown,
    /// 知れた番号 (空なら GPU のプロセスが無い)。
    Known(Vec<u32>),
    /// 知れなかった (古い WebView2 Runtime など)。
    Failed(CommandError),
}

/// 優先度を当てた様子。
#[derive(Debug)]
pub struct PriorityState {
    /// 選んでいる優先度。
    priority: GpuPriority,
    webview: WebView,
    /// 当てた先と、当てた優先度 (無い先は作ったときの「通常」のまま)。
    applied: Vec<(Target, GpuPriority)>,
    /// スタジオのプロセスに当てられなかった理由。
    studio_error: Option<CommandError>,
    /// WebView2 の GPU のプロセスに当てられなかった理由。
    webview_error: Option<CommandError>,
}

impl Default for PriorityState {
    fn default() -> Self {
        Self {
            priority: GpuPriority::Normal,
            webview: WebView::Unknown,
            applied: Vec::new(),
            studio_error: None,
            webview_error: None,
        }
    }
}

impl PriorityState {
    /// 当てられなかった理由 (スタジオのプロセス・WebView2 の順。無ければ空)。
    pub fn errors(&self) -> Vec<CommandError> {
        self.studio_error
            .iter()
            .chain(&self.webview_error)
            .cloned()
            .collect()
    }

    /// 優先度を選び、すべての先に当てる (起動したときと、選び直したとき)。
    pub fn set(&mut self, priority: GpuPriority, apply: &mut impl ApplyPriority) {
        self.priority = priority;
        self.studio_error = self.apply(Target::Studio, apply);
        self.apply_webview(apply);
    }

    /// WebView2 の GPU のプロセスが分かった・変わった (作り直された) ので、新しいプロセスに当てる。
    pub fn webview_changed(
        &mut self,
        processes: Result<Vec<u32>, Error>,
        apply: &mut impl ApplyPriority,
    ) {
        self.webview = match processes {
            Ok(pids) => {
                // 無くなったプロセスの記録は捨てる (同じ番号が別のプロセスに使われても「通常」から始める)
                self.applied
                    .retain(|(t, _)| !matches!(t, Target::WebView(pid) if !pids.contains(pid)));
                WebView::Known(pids)
            }
            Err(e) => WebView::Failed(webview_error(e)),
        };
        self.apply_webview(apply);
    }

    fn apply_webview(&mut self, apply: &mut impl ApplyPriority) {
        self.webview_error = match &self.webview {
            // 調べ終えていなければ、分かったときに当てる
            WebView::Unknown => None,
            // 下げないなら、知れなくても困らない
            _ if self.priority == GpuPriority::Normal && !self.lowered_webview() => None,
            WebView::Failed(error) => Some(error.clone()),
            WebView::Known(pids) if pids.is_empty() => {
                Some(CommandError::gpu_priority_no_gpu_process())
            }
            WebView::Known(pids) => {
                // どれかで失敗しても、残りのプロセスにも当てる (知らせるのは最初の失敗)
                let pids = pids.clone();
                let errors: Vec<_> = pids
                    .into_iter()
                    .filter_map(|pid| self.apply(Target::WebView(pid), apply))
                    .collect();
                errors.into_iter().next()
            }
        };
    }

    /// WebView2 のプロセスに「通常」でない優先度を当てたことがあるか。
    fn lowered_webview(&self) -> bool {
        self.applied
            .iter()
            .any(|(t, p)| matches!(t, Target::WebView(_)) && *p != GpuPriority::Normal)
    }

    /// 1 つの先に、今の優先度を当てる (当ててあれば何もしない)。当てられなければ理由を返す。
    fn apply(&mut self, target: Target, apply: &mut impl ApplyPriority) -> Option<CommandError> {
        let current = self
            .applied
            .iter()
            .find(|(t, _)| *t == target)
            .map_or(GpuPriority::Normal, |(_, p)| *p);
        if current == self.priority {
            return None;
        }
        match apply(target, self.priority) {
            Ok(()) => {
                self.applied.retain(|(t, _)| *t != target);
                self.applied.push((target, self.priority));
                None
            }
            Err(e) => Some(target_error(target, e)),
        }
    }
}

/// 当てる先で起きた失敗のコード。
fn target_error(target: Target, error: Error) -> CommandError {
    let process = target.process();
    match error {
        Error::AccessDenied => CommandError::gpu_priority_access_denied(&process),
        Error::ProcessNotFound => CommandError::gpu_priority_process_not_found(&process),
        Error::WebView2TooOld => CommandError::gpu_priority_webview_too_old(),
        Error::Other(reason) => CommandError::gpu_priority_failed(&process, reason),
    }
}

/// WebView2 の GPU のプロセスを知れなかったときのコード。
fn webview_error(error: Error) -> CommandError {
    match error {
        Error::WebView2TooOld => CommandError::gpu_priority_webview_too_old(),
        e => CommandError::gpu_priority_failed("msedgewebview2", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 当てた先と優先度を記録し、`fail` の先では `error` を返す偽の口。
    struct Fake {
        calls: Vec<(Target, GpuPriority)>,
        fail: Option<(Target, fn() -> Error)>,
    }

    impl Fake {
        fn new() -> Self {
            Self {
                calls: Vec::new(),
                fail: None,
            }
        }

        fn apply(&mut self) -> impl FnMut(Target, GpuPriority) -> Result<(), Error> + '_ {
            |target, priority| {
                self.calls.push((target, priority));
                match self.fail {
                    Some((t, error)) if t == target => Err(error()),
                    _ => Ok(()),
                }
            }
        }
    }

    const LOW: GpuPriority = GpuPriority::BelowNormal;
    const NORMAL: GpuPriority = GpuPriority::Normal;

    #[test]
    fn normal_at_start_touches_nothing() {
        let mut state = PriorityState::default();
        let mut fake = Fake::new();
        state.set(NORMAL, &mut fake.apply());
        state.webview_changed(Ok(vec![10]), &mut fake.apply());
        assert!(fake.calls.is_empty());
        assert!(state.errors().is_empty());
        // GPU のプロセスが知れなくても、下げないなら知らせない
        state.webview_changed(Err(Error::WebView2TooOld), &mut fake.apply());
        state.webview_changed(Ok(vec![]), &mut fake.apply());
        assert!(fake.calls.is_empty());
        assert!(state.errors().is_empty());
    }

    #[test]
    fn low_applies_to_the_studio_and_the_webview_gpu_process() {
        let mut state = PriorityState::default();
        let mut fake = Fake::new();
        // 起動したとき: WebView2 のプロセスは後で分かる
        state.set(LOW, &mut fake.apply());
        assert_eq!(fake.calls, [(Target::Studio, LOW)]);
        state.webview_changed(Ok(vec![10]), &mut fake.apply());
        assert_eq!(fake.calls[1..], [(Target::WebView(10), LOW)]);
        // ほかのプロセスが変わっただけなら当て直さない
        state.webview_changed(Ok(vec![10]), &mut fake.apply());
        assert_eq!(fake.calls.len(), 2);
        // GPU のプロセスが作り直されたら、新しいほうに当てる
        state.webview_changed(Ok(vec![11]), &mut fake.apply());
        assert_eq!(fake.calls[2..], [(Target::WebView(11), LOW)]);
        // 戻すと、当てた先をすべて通常にする
        state.set(NORMAL, &mut fake.apply());
        assert_eq!(
            fake.calls[3..],
            [(Target::Studio, NORMAL), (Target::WebView(11), NORMAL)]
        );
        assert!(state.errors().is_empty());
        assert_eq!(state.priority, NORMAL);
    }

    #[test]
    fn failures_are_reported_with_codes() {
        let mut state = PriorityState::default();
        let mut fake = Fake::new();
        fake.fail = Some((Target::WebView(10), || Error::AccessDenied));
        state.webview_changed(Ok(vec![10]), &mut fake.apply());
        state.set(LOW, &mut fake.apply());
        let codes: Vec<_> = state.errors().iter().map(|e| e.code).collect();
        assert_eq!(codes, ["gpu_priority.access_denied"]);
        assert_eq!(
            state.errors()[0].args["process"],
            "msedgewebview2 (GPU, 10)"
        );
        // 当てられなかった先は、次の機会に当て直す
        fake.fail = None;
        state.webview_changed(Ok(vec![10]), &mut fake.apply());
        assert!(state.errors().is_empty());
        assert_eq!(fake.calls.last(), Some(&(Target::WebView(10), LOW)));

        // 古い WebView2 Runtime と、GPU のプロセスが無いとき
        state.webview_changed(Err(Error::WebView2TooOld), &mut fake.apply());
        let codes: Vec<_> = state.errors().iter().map(|e| e.code).collect();
        assert_eq!(codes, ["gpu_priority.webview_too_old"]);
        state.webview_changed(Ok(vec![]), &mut fake.apply());
        let codes: Vec<_> = state.errors().iter().map(|e| e.code).collect();
        assert_eq!(codes, ["gpu_priority.no_gpu_process"]);

        // スタジオのプロセスに当てられない (と、そのほかの失敗)
        let mut state = PriorityState::default();
        let mut fake = Fake::new();
        fake.fail = Some((Target::Studio, || {
            Error::Other("NTSTATUS 0xc000000d".into())
        }));
        state.set(LOW, &mut fake.apply());
        let errors = state.errors();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "gpu_priority.failed");
        assert_eq!(errors[0].args["process"], "ugoe-studio");
        // 当てられなかったので、戻すときは何もしない
        fake.fail = None;
        state.set(NORMAL, &mut fake.apply());
        assert_eq!(fake.calls.len(), 1);
        assert!(state.errors().is_empty());
    }

    #[test]
    fn a_vanished_process_is_reported_and_forgotten() {
        let mut state = PriorityState::default();
        let mut fake = Fake::new();
        state.webview_changed(Ok(vec![10]), &mut fake.apply());
        state.set(LOW, &mut fake.apply());
        // 番号 10 が別のプロセスに使い回されても、当てたことにしない
        state.webview_changed(Ok(vec![12]), &mut fake.apply());
        state.webview_changed(Ok(vec![10]), &mut fake.apply());
        assert_eq!(
            fake.calls,
            [
                (Target::Studio, LOW),
                (Target::WebView(10), LOW),
                (Target::WebView(12), LOW),
                (Target::WebView(10), LOW),
            ]
        );
        fake.fail = Some((Target::WebView(13), || Error::ProcessNotFound));
        state.webview_changed(Ok(vec![13]), &mut fake.apply());
        assert_eq!(state.errors()[0].code, "gpu_priority.process_not_found");
    }
}
