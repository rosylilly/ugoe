//! ブラウザソースの画の確かめ (T0808、ADR 0028 の 10. の「画の一致」)。ヘッドレスの Chrome で開くページのための待ち受けで、
//! 手で実行する (`apps/studio/README.md#ブラウザソースの画を確かめる`)。
//!
//! 見本のモデル (`avatar-lit.ugoe`。法線マップと物理付き) のシーンに、光源・配置 (回転を含む)・背景の画像を付けてブラウザソースを出す。
//! ページが 2 つ (ブラウザソースのページと、出力ウィンドウのページ) つないだら、頭を振るフレームを決まった `dt` の列で配って止め、
//! 同じフレームの列を仮想カメラの受け手 (`Feed`) にも適用して読み戻した画を `vcam.png` に書く。止めた後の 3 つの画を比べる。
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use ugoe_headless::image::{self, Rgba8};
use ugoe_headless::yuv::PixelFormat;
use ugoe_headless::{Gpu, Wait};

use super::store;
use crate::profile::{Background, Transform};
use crate::virtual_camera::{Command, Feed, Settings};
use crate::{CameraThread, Studio, lock};

/// 出力の大きさ (ページの表示の大きさと同じにする)。
const SIZE: (u32, u32) = (480, 270);
/// 透過を埋める色 (背景の画像の透過のところ)。
const FILL: [u8; 3] = [40, 44, 60];
/// 配るフレームの数。
const FRAMES: usize = 150;

/// 背景の画像 (横長のグラデーションで、右下に透過のところがある)。
fn background() -> Rgba8 {
    let (w, h) = (640u32, 240u32);
    let mut pixels = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let a = if x > 480 && y > 140 { 0 } else { 255 };
            pixels.extend_from_slice(&[(x * 2 / 5) as u8, y as u8, 200 - (x / 5) as u8, a]);
        }
    }
    image::premultiply(&mut pixels);
    Rgba8 {
        width: w,
        height: h,
        pixels,
    }
}

#[test]
#[ignore = "manual: serves the browser source for a headless Chrome check (UGOE_STUDIO_CHECK_DIR)"]
fn serves_a_frozen_scene_for_a_browser_check() {
    let Ok(out) = std::env::var("UGOE_STUDIO_CHECK_DIR").map(PathBuf::from) else {
        return;
    };
    std::fs::create_dir_all(&out).unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let dist = root.join("apps/studio/dist");
    let studio = Arc::new(Studio::default());
    studio
        .sources
        .set_assets(move |path| std::fs::read(dist.join(path)).ok());
    let background_path = out.join("background.png");
    std::fs::write(
        &background_path,
        image::encode_png_unpremultiplied(&background()).unwrap(),
    )
    .unwrap();
    studio.with_stage(|stage, conductor| {
        let mut scene = stage.profile().scenes[0].clone();
        scene.model = Some(root.join("assets/samples/avatar-lit.ugoe"));
        scene.lighting = Some(serde_json::json!({
            // 光源を切って比べる (違いが光源から来ているかを分ける)。
            "enabled": std::env::var_os("UGOE_STUDIO_CHECK_NO_LIGHT").is_none(),
            "ambient": { "color": "#5a6aa0", "intensity": 0.45 },
            "directional": { "enabled": true, "color": "#ffe0c0", "intensity": 0.9, "azimuth": 30, "elevation": 25 },
            "point": { "enabled": true, "color": "#a0d0ff", "intensity": 1.2, "x": 0.8, "y": 0.2, "z": 0.3, "range": 1.2 }
        }));
        scene.transform = Transform {
            x: 0.08,
            y: 0.05,
            scale: 1.3,
            // 出力ウィンドウとページは CSS で回すので、回すと縁の画素が仮想カメラと少し違う。
            rotation: std::env::var("UGOE_STUDIO_CHECK_ROTATION")
                .ok()
                .and_then(|r| r.parse().ok())
                .unwrap_or(8.0),
        };
        scene.background = Background::Image {
            path: background_path.clone(),
        };
        let id = scene.id.clone();
        stage.update_scene(scene, conductor).unwrap();
        stage.activate(&id, conductor).unwrap();
    });
    // 仮想カメラの受け手へのチャネル (スレッドは作らず、このテストで受ける)。
    let (tx, rx) = mpsc::channel();
    let files = {
        let stage = lock(&studio.stage);
        let conductor = lock(&studio.conductor);
        Studio::camera_scene(&stage, &conductor).unwrap()
    };
    *lock(&studio.camera) = Some(CameraThread {
        commands: tx,
        thread: std::thread::spawn(|| {}),
    });
    lock(&studio.api.0).settings.source.token = store::generate_secret().unwrap();
    let port = std::env::var("UGOE_STUDIO_CHECK_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(28461);
    studio.set_api_port(port).unwrap();
    studio.set_source_enabled(true);
    assert!(
        studio.api_view().source.listening,
        "{:?}",
        studio.api_view()
    );
    std::fs::write(out.join("url.txt"), studio.source_url()).unwrap();
    eprintln!("serving {}", studio.source_url());

    // ページが 2 つつなぐのを待ち、モデルを読む時間を置く。
    let deadline = Instant::now() + Duration::from_secs(120);
    while studio.sources.len() < 2 {
        assert!(Instant::now() < deadline, "the pages did not connect");
        std::thread::sleep(Duration::from_millis(50));
    }
    std::thread::sleep(Duration::from_secs(5));

    let gpu = Gpu::new(None).unwrap();
    let mut feed = Feed::new(
        &Settings {
            width: SIZE.0,
            height: SIZE.1,
            fps: 30,
            background: FILL,
        },
        PixelFormat::Rgba8,
    );
    feed.set_scene(&gpu, &files).unwrap();
    let mut last = 0;
    for i in 0..FRAMES {
        let value = 28.0 * (i as f32 * 0.17).sin();
        assert!(lock(&studio.conductor).set_parameter("AngleX", value));
        let dt = if i % 7 == 0 { 0.023 } else { 1.0 / 60.0 };
        studio.tick(dt, false);
        // ページが追いつけるよう、刻みの速さで配る。
        std::thread::sleep(Duration::from_millis(16));
    }
    for command in rx.try_iter() {
        if let Command::Frame(frame) = command {
            feed.receive(&frame).unwrap();
            last = frame.seq;
        }
    }
    feed.render(&gpu).unwrap();
    let rgba = feed.poll(&gpu, Wait::All).unwrap().unwrap();
    let png = image::encode_png_unpremultiplied(&Rgba8 {
        width: SIZE.0,
        height: SIZE.1,
        pixels: rgba,
    })
    .unwrap();
    std::fs::write(out.join("vcam.png"), png).unwrap();
    std::fs::write(out.join("done.txt"), last.to_string()).unwrap();
    eprintln!("frozen at seq {last}");
    let secs = std::env::var("UGOE_STUDIO_CHECK_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);
    std::thread::sleep(Duration::from_secs(secs));
    gpu.leak();
}
