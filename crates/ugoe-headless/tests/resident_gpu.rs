//! 常駐の描画の口 (`Resident`) を GPU で確かめる。`just test-gpu` で実行する。
//!
//! - RGBA で読み戻した画は、1 枚を描く口 (`Gpu::render`) と同じ。
//! - NV12・YUY2 の計算シェーダの結果は、同じ画を CPU の参照実装 (`yuv::convert`) で変換したものと ±1 で一致する。
//! - 読み戻しを待たない: 空いたバッファが無ければ描かずに `false` を返し、`poll` は最も新しい 1 枚を返す。
#![cfg(feature = "gpu-tests")]
#![allow(clippy::unwrap_used)]

use std::sync::OnceLock;

use ugoe_headless::yuv::{self, PixelFormat};
use ugoe_headless::{
    Fit, Gpu, Prepared, RenderOptions, Resident, ResidentOptions, Rgba8, Scene, Wait, fit_view,
    prepare,
};
use ugoe_model::fixtures;

const SIZE: (u32, u32) = (64, 48);
const BACKGROUND: [u8; 4] = [40, 80, 120, 255];

/// テスト全体で 1 つの GPU を使い、破棄しない (WSL2 の dzn は破棄で落ちることがある。crates/ugoe-headless/README.md)。
fn gpu() -> &'static Gpu {
    static GPU: OnceLock<Gpu> = OnceLock::new();
    GPU.get_or_init(|| {
        let gpu = Gpu::new(None).unwrap();
        eprintln!(
            "resident gpu tests: {} (compute: {})",
            gpu.adapter_description(),
            gpu.supports_compute()
        );
        gpu
    })
}

/// 4 色の 2×2 のテクスチャを貼った四角形 (色差の平均を確かめるため、色の境目を含める)。
fn prepared() -> Prepared {
    let mut prepared = prepare(
        Scene::from_model(fixtures::single_quad()),
        &RenderOptions {
            size: Some(SIZE),
            background: BACKGROUND,
            ..Default::default()
        },
    )
    .unwrap();
    prepared.textures = vec![Rgba8 {
        width: 2,
        height: 2,
        pixels: vec![
            255, 0, 0, 255, 0, 255, 0, 255, //
            0, 0, 255, 255, 255, 255, 255, 255,
        ],
    }];
    prepared
}

fn resident(prepared: &Prepared, format: PixelFormat, slots: usize) -> Resident {
    let canvas = prepared.puppet.instance().compiled().model().meta.canvas;
    Resident::new(
        gpu(),
        &prepared.textures,
        &prepared.shading,
        ResidentOptions {
            size: SIZE,
            format,
            background: BACKGROUND,
            view: fit_view((canvas.width, canvas.height), SIZE, Fit::Contain),
            slots,
        },
    )
    .unwrap()
}

fn render_once(resident: &mut Resident, prepared: &Prepared) -> Vec<u8> {
    assert!(
        resident
            .submit(gpu(), prepared.puppet.instance(), 1)
            .unwrap()
    );
    let readback = resident.poll(gpu(), Wait::All).unwrap().unwrap();
    assert_eq!(readback.tag, 1);
    assert_eq!((readback.width, readback.height), SIZE);
    assert_eq!(
        readback.bytes.len(),
        readback.format.frame_bytes(SIZE.0, SIZE.1)
    );
    readback.bytes
}

#[test]
fn rgba_matches_the_one_shot_renderer() {
    let prepared = prepared();
    let (expected, _) = prepared.render(gpu()).unwrap();
    let mut resident = resident(&prepared, PixelFormat::Rgba8, 2);
    // 2 回描いても同じ (クリアしてから描く)。
    for _ in 0..2 {
        assert_eq!(render_once(&mut resident, &prepared), expected.pixels);
    }
    // 背景は不透明のまま埋まる。
    assert_eq!(&expected.pixels[..4], &BACKGROUND);
}

#[test]
fn yuv_on_the_gpu_matches_the_cpu_reference() {
    let prepared = prepared();
    let (rgba, _) = prepared.render(gpu()).unwrap();
    for format in [PixelFormat::Nv12, PixelFormat::Yuy2] {
        let expected = yuv::convert(&rgba, format).unwrap();
        let mut resident = resident(&prepared, format, 2);
        let actual = render_once(&mut resident, &prepared);
        assert_eq!(actual.len(), expected.len(), "{format:?}");
        for (i, (a, e)) in actual.iter().zip(&expected).enumerate() {
            assert!(
                (i32::from(*a) - i32::from(*e)).abs() <= 1,
                "{format:?} byte {i}: {a} != {e}"
            );
        }
    }
}

#[test]
fn readbacks_do_not_wait_and_the_newest_wins() {
    let prepared = prepared();
    let mut resident = resident(&prepared, PixelFormat::Nv12, 3);
    let instance = prepared.puppet.instance();
    for tag in 0..3 {
        assert!(resident.submit(gpu(), instance, tag).unwrap());
    }
    // 3 枚とも読み戻しを待っている間は描かない (GPU が追いついていない)。
    assert_eq!(resident.in_flight(), 3);
    assert!(!resident.submit(gpu(), instance, 3).unwrap());
    let readback = resident.poll(gpu(), Wait::All).unwrap().unwrap();
    assert_eq!(readback.tag, 2);
    assert_eq!(resident.superseded(), 2);
    assert_eq!(resident.in_flight(), 0);
    assert!(resident.poll(gpu(), Wait::All).unwrap().is_none());
    // 背景の色を変えると次の 1 枚から効く。
    resident.set_background([255, 255, 255, 255]);
    assert!(resident.submit(gpu(), instance, 4).unwrap());
    let white = resident.poll(gpu(), Wait::All).unwrap().unwrap();
    assert_eq!(white.bytes[0], 235);
}

#[test]
fn backdrop_replaces_the_clear_color_until_removed() {
    let prepared = prepared();
    let mut resident = resident(&prepared, PixelFormat::Rgba8, 2);
    let color = [200, 100, 50, 255];
    let backdrop = Rgba8 {
        width: SIZE.0,
        height: SIZE.1,
        pixels: color.repeat(SIZE.0 as usize * SIZE.1 as usize),
    };
    resident.set_backdrop(gpu(), Some(&backdrop)).unwrap();
    // 隅はモデルの外なので、背景の画像の色になる。
    assert_eq!(&render_once(&mut resident, &prepared)[..4], &color);
    // 外すとクリアの色に戻る。
    resident.set_backdrop(gpu(), None).unwrap();
    assert_eq!(&render_once(&mut resident, &prepared)[..4], &BACKGROUND);
    // 大きさの違う画像は置かない。
    let small = Rgba8 {
        width: 1,
        height: 1,
        pixels: color.to_vec(),
    };
    let err = resident.set_backdrop(gpu(), Some(&small)).unwrap_err();
    assert!(err.to_string().contains("backdrop is 1x1"), "{err}");
}
