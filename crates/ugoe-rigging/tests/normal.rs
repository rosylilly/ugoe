//! 法線マップの生成 (決定的な方法) のテスト。`docs/ai-integration.md#法線マップの生成`。

use ugoe_rigging::{BulgeOptions, NormalMap, bulge_height, bulge_normal_map};

/// 一辺 `size` の画像に、中心 `(size/2, size/2)`・半径 `radius` の不透明な円を描く (RGBA8)。
fn disc(size: u32, radius: f32) -> Vec<u8> {
    let c = size as f32 / 2.0;
    (0..size * size)
        .flat_map(|i| {
            let (x, y) = ((i % size) as f32 + 0.5, (i / size) as f32 + 0.5);
            let inside = (x - c).hypot(y - c) <= radius;
            [200, 100, 50, if inside { 255 } else { 0 }]
        })
        .collect()
}

fn normal_at(map: &NormalMap, x: f32, y: f32) -> [f32; 3] {
    map.normal(x as u32, y as u32).expect("inside the image")
}

#[test]
fn disc_gives_radial_normals() {
    let size = 64;
    let map = bulge_normal_map(size, size, &disc(size, 24.0), &BulgeOptions::default()).unwrap();
    let c = size as f32 / 2.0;
    // 中心は正面を向く。
    let n = normal_at(&map, c, c);
    assert!(n[2] > 0.99, "center {n:?}");
    // 円の中の点の法線の xy は、中心から外へ向かう向き (法線マップの y は上向き) と揃う。
    for k in 0..16 {
        let angle = k as f32 / 16.0 * std::f32::consts::TAU;
        for r in [8.0, 14.0, 20.0] {
            let (dx, dy) = (angle.cos(), angle.sin());
            let n = normal_at(&map, c + dx * r, c + dy * r);
            let (nx, ny) = (n[0], -n[1]);
            let len = nx.hypot(ny);
            assert!(len > 0.1, "r {r} angle {angle}: {n:?}");
            let cos = (nx * dx + ny * dy) / len;
            assert!(cos > 0.97, "r {r} angle {angle}: {n:?} (cos {cos})");
        }
    }
    // 外ほど寝る (半球に近い)。
    let tilt = |r: f32| normal_at(&map, c + r, c)[0];
    assert!(tilt(8.0) < tilt(14.0) && tilt(14.0) < tilt(20.0));
    // 半球の法線 (x = r / R) に近い。
    assert!((tilt(12.0) - 0.5).abs() < 0.08, "{}", tilt(12.0));
    // 円の外は平ら。
    assert_eq!(&map.rgba[..4], &[128, 128, 255, 255]);
    // 上の側は上 (法線マップの +y) を向く。
    assert!(normal_at(&map, c, c - 16.0)[1] > 0.3);
}

#[test]
fn strength_scales_the_bulge() {
    let size = 64;
    let rgba = disc(size, 24.0);
    let tilt = |strength: f32| {
        let options = BulgeOptions {
            strength,
            ..Default::default()
        };
        bulge_normal_map(size, size, &rgba, &options)
            .unwrap()
            .normal(32 + 12, 32)
            .unwrap()[0]
    };
    assert!(tilt(0.0).abs() < 0.01, "flat");
    assert!(tilt(0.3) < tilt(1.0) && tilt(1.0) < tilt(2.0));
    // 高さは strength に比例する。
    let height = |strength: f32| {
        let options = BulgeOptions {
            strength,
            ..Default::default()
        };
        bulge_height(size, size, &rgba, &options).unwrap().values[32 * 64 + 32]
    };
    assert!((height(0.5) * 2.0 - height(1.0)).abs() < 1e-4);
}

#[test]
fn radius_limits_the_rounded_rim() {
    // 横長の帯: 幅を狭くすると、中央は平らで縁だけが丸まる。
    let (w, h) = (96u32, 32u32);
    let rgba: Vec<u8> = (0..w * h)
        .flat_map(|i| {
            let y = i / w;
            [255, 255, 255, if (4..28).contains(&y) { 255 } else { 0 }]
        })
        .collect();
    let options = BulgeOptions {
        radius: Some(4.0),
        ..Default::default()
    };
    let map = bulge_normal_map(w, h, &rgba, &options).unwrap();
    let center = map.normal(48, 16).unwrap();
    assert!(center[2] > 0.999, "{center:?}");
    let rim = map.normal(48, 5).unwrap();
    assert!(rim[1] > 0.3, "upper rim faces up: {rim:?}");
    let rim = map.normal(48, 26).unwrap();
    assert!(rim[1] < -0.3, "lower rim faces down: {rim:?}");
    // 自動の幅では帯全体が丸い (中央の少し上も上を向く)。
    let map = bulge_normal_map(w, h, &rgba, &BulgeOptions::default()).unwrap();
    assert!(map.normal(48, 12).unwrap()[1] > 0.2);
}

#[test]
fn result_is_deterministic_and_opaque() {
    let rgba = disc(40, 15.0);
    let a = bulge_normal_map(40, 40, &rgba, &BulgeOptions::default()).unwrap();
    let b = bulge_normal_map(40, 40, &rgba, &BulgeOptions::default()).unwrap();
    assert_eq!(a, b);
    assert!(a.rgba.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
}
