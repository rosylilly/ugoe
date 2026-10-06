//! 描画テスト (`tests/gpu.rs`・`tests/spec_render.rs`) で共有する補助。
//! ugoe-ai の `tests/normal_render.rs` も `#[path]` で取り込む (wgpu だけに依存させておく)。

use std::sync::mpsc;
use std::time::Duration;

/// 待ち終わったのにマップのコールバックがまだ呼ばれていないとき、poll し直す回数の上限。
///
/// wgpu はコールバックを、`poll` で集めたスレッドがロックを外してから呼ぶ。テストは 1 つのデバイスを
/// 複数のスレッドで共有するので、こちらの `poll` が戻っても、こちらのコールバックはほかのスレッドが持ったまま、
/// まだ呼ばれていないことがある (T0963・T0964)。そのスレッドがすぐに呼ぶので、少し間を置いて確かめ直す。
const CALLBACK_RETRIES: u32 = 1000;

/// poll し直す前に置く間 (ほかのスレッドがコールバックを呼ぶのを待つ。空回りしないように)。
const CALLBACK_RETRY_PAUSE: Duration = Duration::from_millis(1);

/// `buffer` 全体を読むためにマップし、マップのコールバックが呼ばれるまで待つ。
///
/// マップに失敗したら、このテストのスレッドで panic する (ほかのスレッドの `poll` の中で panic しないように)。
pub fn map_read(device: &wgpu::Device, buffer: &wgpu::Buffer) {
    let (tx, rx) = mpsc::channel();
    buffer.map_async(wgpu::MapMode::Read, .., move |r| {
        let _ = tx.send(r);
    });
    for _ in 0..=CALLBACK_RETRIES {
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
        if let Ok(result) = rx.try_recv() {
            result.expect("map readback");
            return;
        }
        std::thread::sleep(CALLBACK_RETRY_PAUSE);
    }
    panic!("the readback callback was not called after {CALLBACK_RETRIES} polls");
}
