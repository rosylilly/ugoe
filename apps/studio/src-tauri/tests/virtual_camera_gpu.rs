//! 仮想カメラの受け手の画 (ADR 0028 の 10.)。`just test-gpu` で実行する。
//!
//! 指揮役が配ったフレームの列を仮想カメラの受け手 (`Feed`: ネイティブの受け手 + 常駐の描画) に適用して読み戻した画が、
//! 物理を有効にした 1 つの `Puppet` を同じ入力で進めて 1 枚の口 (`Gpu::render`) で描き、CPU で同じ形に変換した画と一致する。
//! 動的物理で揺れる髪のモデルで、頭を振ってから比べる (物理が効いた形で比べる)。
#![cfg(feature = "gpu-tests")]
#![allow(clippy::unwrap_used)]

use std::sync::{Arc, OnceLock};

use ugoe::format::{Bundle, TextureData, TextureFormat};
use ugoe::model::fixtures;
use ugoe::{Puppet, Toggles};
use ugoe_headless::yuv::{self, PixelFormat};
use ugoe_headless::{Fit, Gpu, Rgba8, Shading, Wait, fit_view, image, texture_count};
use ugoe_studio_lib::conductor::Conductor;
use ugoe_studio_lib::virtual_camera::{Feed, SceneFiles, Settings};

const SIZE: (u32, u32) = (96, 64);
const BACKGROUND: [u8; 3] = [30, 160, 90];
/// 頂点の小さな差 (受け手と 1 つの Puppet の差は 1e-3 以内) とラスタライズの境目の差を吸う。
const TOLERANCE: i32 = 3;
/// 許容誤差を超えてよい画素の割合 (縁の画素だけ)。
const OUTLIER_SHARE: f64 = 0.01;

fn gpu() -> &'static Gpu {
    static GPU: OnceLock<Gpu> = OnceLock::new();
    GPU.get_or_init(|| Gpu::new(None).unwrap())
}

fn texture() -> Rgba8 {
    Rgba8 {
        width: 2,
        height: 2,
        pixels: vec![
            250, 40, 40, 255, 40, 40, 250, 255, //
            240, 240, 60, 255, 250, 250, 250, 255,
        ],
    }
}

/// 物理付きのバンドルのバイト列 (テクスチャは PNG)。
fn bundle() -> (Bundle, Vec<u8>) {
    let model = fixtures::dynamic_hair();
    let png = image::encode_png_unpremultiplied(&texture()).unwrap();
    let mut bundle = Bundle::new(model);
    bundle.physics = Some(fixtures::dynamic_hair_settings());
    bundle.textures = vec![
        TextureData {
            format: TextureFormat::Png,
            bytes: png,
        };
        texture_count(&bundle.model)
    ];
    let bytes = ugoe::format::write_bundle(&bundle).unwrap();
    (bundle, bytes)
}

fn settings() -> Settings {
    Settings {
        width: SIZE.0,
        height: SIZE.1,
        fps: 30,
        background: BACKGROUND,
    }
}

fn compare(actual: &[u8], expected: &[u8], what: &str) {
    assert_eq!(actual.len(), expected.len(), "{what}");
    let outliers = actual
        .iter()
        .zip(expected)
        .filter(|(a, e)| (i32::from(**a) - i32::from(**e)).abs() > TOLERANCE)
        .count();
    assert!(
        (outliers as f64) <= actual.len() as f64 * OUTLIER_SHARE,
        "{what}: {outliers} of {} bytes differ by more than {TOLERANCE}",
        actual.len()
    );
}

/// 物理を有効にした 1 つの `Puppet`。`physics` が偽なら物理の段を切る。
fn puppet(source: &Bundle, physics: bool) -> Puppet {
    let mut puppet = Puppet::new(source.model.clone())
        .unwrap()
        .with_physics(source.physics.clone().unwrap());
    puppet.set_toggles(Toggles {
        physics,
        ..Toggles::default()
    });
    puppet
}

fn set_angle(puppet: &mut Puppet, value: f32) {
    let params = puppet.instance_mut().parameters_mut();
    let index = params.index_of("AngleX").unwrap();
    params.set(index, value);
}

#[test]
fn the_camera_feed_matches_a_single_puppet() {
    let (source, bytes) = bundle();
    let [r, g, b] = BACKGROUND;
    for format in [PixelFormat::Nv12, PixelFormat::Yuy2] {
        let mut conductor = Conductor::new();
        let scene = conductor.load("hair.ugoe", source.clone()).unwrap().clone();
        let mut reference = puppet(&source, true);
        let mut still = puppet(&source, false);
        let mut feed = Feed::new(&settings(), format);
        // モデルが無い間は背景の単色。
        let fill = feed.poll(gpu(), Wait::All).unwrap().unwrap();
        let solid = Rgba8 {
            width: SIZE.0,
            height: SIZE.1,
            pixels: [r, g, b, 255].repeat((SIZE.0 * SIZE.1) as usize),
        };
        assert_eq!(
            fill,
            yuv::convert(&solid, format).unwrap(),
            "{format:?} fill"
        );
        let files = SceneFiles {
            scene,
            model: Some(Arc::new(bytes.clone())),
            modules: Vec::new(),
        };
        feed.set_scene(gpu(), &files).unwrap();
        for i in 0..90 {
            let value = 30.0 * (i as f32 * 0.2).sin();
            assert!(conductor.set_parameter("AngleX", value));
            set_angle(&mut reference, value);
            set_angle(&mut still, value);
            let dt = if i % 5 == 0 { 0.021 } else { 1.0 / 60.0 };
            let frame = conductor.tick(dt).unwrap();
            reference.update(dt);
            still.update(dt);
            feed.receive(&frame).unwrap();
            // 描画は 2 刻みに 1 回 (仮想カメラのフレームレートは刻みより遅い)。結果の形には効かない。
            if i % 2 == 0 {
                feed.render(gpu()).unwrap();
                feed.poll(gpu(), Wait::No).unwrap();
            }
        }
        feed.render(gpu()).unwrap();
        let actual = feed.poll(gpu(), Wait::All).unwrap().unwrap();

        let model = reference.instance().compiled().model();
        let canvas = (model.meta.canvas.width, model.meta.canvas.height);
        let png = image::encode_png_unpremultiplied(&texture()).unwrap();
        let textures = vec![image::decode_png_premultiplied(&png).unwrap(); texture_count(model)];
        let (rgba, _) = gpu()
            .render(
                reference.instance(),
                &textures,
                &Shading::default(),
                SIZE,
                &fit_view(canvas, SIZE, Fit::Contain),
                [r, g, b, 255],
            )
            .unwrap();
        compare(
            &actual,
            &yuv::convert(&rgba, format).unwrap(),
            &format!("{format:?}"),
        );
        // モデルが描かれている (背景だけではない)。
        assert_ne!(actual, fill, "{format:?}: only the background was drawn");
        // 物理が効いている (比べる相手が、物理を切った形と違う)。上の比べ合わせが空振りしていないことの確認。
        let moved = reference
            .instance()
            .drawables()
            .iter()
            .zip(still.instance().drawables())
            .any(|(a, b)| {
                a.positions
                    .iter()
                    .zip(&b.positions)
                    .any(|(p, q)| (p.x - q.x).abs() + (p.y - q.y).abs() > 1e-2)
            });
        assert!(moved, "{format:?}: physics did not move the hair");
    }
}
