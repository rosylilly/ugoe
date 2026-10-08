//! macOS の App Nap を止める (T0855。docs/studio.md#刻みの速さと正確さ)。
//!
//! App Nap は、ウィンドウがすべて隠れたアプリのタイマーとスレッドの起床を遅らせる。指揮役の刻みはバックエンドの
//! スレッドのスリープで回すので、配信中に両方のウィンドウが隠れると刻みが遅れうる。そこで起動から終わりまで
//! `NSProcessInfo` の活動 (`beginActivityWithOptions:reason:`) を始めたままにする。macOS の外では何もしない。

/// 始めた活動。持っている間 App Nap しない。終える `endActivity` は `unsafe` なので呼ばず、アプリの終わりまで持つ。
pub struct Activity {
    #[cfg(target_os = "macos")]
    _token:
        objc2::rc::Retained<objc2::runtime::ProtocolObject<dyn objc2_foundation::NSObjectProtocol>>,
}

/// App Nap を止める活動を始める。画面と計算機のスリープは妨げない
/// (`NSActivityUserInitiatedAllowingIdleSystemSleep`)。
#[cfg(target_os = "macos")]
pub fn prevent() -> Activity {
    use objc2_foundation::{NSActivityOptions, NSProcessInfo, NSString};
    let reason = NSString::from_str("ugoe studio drives the stream output at a fixed tick rate");
    let token = NSProcessInfo::processInfo().beginActivityWithOptions_reason(
        NSActivityOptions::UserInitiatedAllowingIdleSystemSleep,
        &reason,
    );
    Activity { _token: token }
}

/// macOS の外では何もしない (App Nap が無い)。
#[cfg(not(target_os = "macos"))]
pub fn prevent() -> Activity {
    Activity {}
}

#[cfg(all(test, not(target_os = "macos")))]
mod tests {
    use super::*;

    #[test]
    fn prevent_holds_nothing_outside_macos() {
        let _activity = prevent();
        assert_eq!(size_of::<Activity>(), 0);
    }
}
