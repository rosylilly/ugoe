//! GPU を待つ時間の上限 (`Gpu::wait_timeout`) を GPU で確かめる。`just test-gpu` で実行する。
//!
//! 本当に止まるドライバは用意できないので、上限を 0 にして「終わっていなければ上限のエラーで戻る
//! (止まり続けない)」ことと、上限を戻せば同じ `Gpu` で描き続けられることを確かめる。上限を超える
//! 決まった道は `src/gpu.rs` の単体テスト (`wait_with`) が確かめる。
#![cfg(feature = "gpu-tests")]
#![allow(clippy::unwrap_used)]

use std::time::Duration;

use ugoe_headless::{
    DEFAULT_WAIT_TIMEOUT, Fit, Gpu, RenderOptions, Resident, ResidentOptions, Scene, Wait,
    fit_view, prepare, yuv::PixelFormat,
};
use ugoe_model::fixtures;

/// 0 秒では終わりにくいように大きめにする。
const SIZE: (u32, u32) = (2048, 2048);

#[test]
fn zero_timeout_returns_instead_of_waiting() {
    let mut gpu = Gpu::new(None).unwrap();
    assert_eq!(gpu.wait_timeout(), DEFAULT_WAIT_TIMEOUT);
    let prepared = prepare(
        Scene::from_model(fixtures::single_quad()),
        &RenderOptions {
            size: Some(SIZE),
            ..Default::default()
        },
    )
    .unwrap();

    // 1 枚の描画: 終わっていれば描け、終わっていなければ上限のエラー。ほかのエラーにはならない。
    gpu.set_wait_timeout(Duration::ZERO);
    let mut timed_out = 0;
    match prepared.render(&gpu) {
        Ok((image, _)) => assert_eq!((image.width, image.height), SIZE),
        Err(e) => {
            assert!(e.is_gpu_wait_timeout(), "{e:?}");
            timed_out += 1;
        }
    }

    // 常駐の描画の `Wait::All` も同じ。
    let canvas = prepared.puppet.instance().compiled().model().meta.canvas;
    let mut resident = Resident::new(
        &gpu,
        &prepared.textures,
        &prepared.shading,
        ResidentOptions {
            size: SIZE,
            format: PixelFormat::Rgba8,
            background: [0, 0, 0, 255],
            view: fit_view((canvas.width, canvas.height), SIZE, Fit::Contain),
            slots: 1,
            cpu_yuv: false,
        },
    )
    .unwrap();
    assert!(
        resident
            .submit(&gpu, prepared.puppet.instance(), 7)
            .unwrap()
    );
    let mut received = None;
    match resident.poll(&gpu, Wait::All) {
        Ok(readback) => received = readback,
        Err(e) => {
            assert!(e.is_gpu_wait_timeout(), "{e:?}");
            timed_out += 1;
        }
    }
    eprintln!("wait timeout gpu test: {timed_out} of 2 waits timed out with a zero limit");

    // 上限を戻せば (実際には止まっていないので) 描き続けられる。
    gpu.set_wait_timeout(DEFAULT_WAIT_TIMEOUT);
    if received.is_none() {
        received = resident.poll(&gpu, Wait::All).unwrap();
    }
    assert_eq!(received.unwrap().tag, 7);
    let (image, _) = prepared.render(&gpu).unwrap();
    assert_eq!((image.width, image.height), SIZE);

    drop(resident);
    // 破棄しない (crates/ugoe-headless/README.md の「GPU」)。
    gpu.leak();
}
