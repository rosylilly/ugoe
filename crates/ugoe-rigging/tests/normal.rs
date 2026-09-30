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

/// `∇²u = −4` (形の外と画像の外で `u = 0`) を SOR で十分に反復して解いた参照解。
fn reference_inflate(mask: &[bool], w: usize, h: usize, iterations: usize) -> Vec<f32> {
    let mut u = vec![0.0f32; w * h];
    for _ in 0..iterations {
        for i in 0..w * h {
            if !mask[i] {
                continue;
            }
            let (x, y) = (i % w, i / w);
            let mut sum = 0.0;
            if x > 0 {
                sum += u[i - 1];
            }
            if x + 1 < w {
                sum += u[i + 1];
            }
            if y > 0 {
                sum += u[i - w];
            }
            if y + 1 < h {
                sum += u[i + w];
            }
            u[i] += 1.9 * ((sum + 4.0) / 4.0 - u[i]);
        }
    }
    u
}

/// 髪の房のような、細い切れ込みの多い形でも、ふくらみの高さが参照解と合う (マルチグリッドが発散・停滞しない)。
/// T0707 で、実際の PSD の前髪・後ろ髪で高さが大きく外れていた (粗い格子の形が細かい格子の形より広がっていたため)。
#[test]
fn ragged_shapes_match_the_reference_solution() {
    let (w, h) = (160usize, 128usize);
    type Inside = Box<dyn Fn(f32, f32) -> bool>;
    let shapes: [(&str, Inside); 3] = [
        // 上が塊で、下に幅 2 の房が並ぶ櫛。
        (
            "comb",
            Box::new(|x, y| {
                (8.0..152.0).contains(&x)
                    && (8.0..120.0).contains(&y)
                    && (y < 48.0 || (x as u32 / 3).is_multiple_of(2))
            }),
        ),
        // 斜めの細い帯が交わる格子。
        (
            "lattice",
            Box::new(|x, y| ((x + y) as u32 % 9 < 3) || ((x + 2.0 * y) as u32 % 11 < 2)),
        ),
        // 円に、放射状の細い切れ込みを入れたもの。
        (
            "slits",
            Box::new(|x, y| {
                let (dx, dy) = (x - 80.0, y - 64.0);
                let r = dx.hypot(dy);
                let a = dy.atan2(dx) / std::f32::consts::TAU * 48.0;
                r < 56.0 && (r < 20.0 || a.rem_euclid(1.0) > 0.3)
            }),
        ),
    ];
    for (name, inside) in shapes {
        let mask: Vec<bool> = (0..w * h)
            .map(|i| inside((i % w) as f32 + 0.5, (i / w) as f32 + 0.5))
            .collect();
        let rgba: Vec<u8> = mask
            .iter()
            .flat_map(|&m| [0, 0, 0, if m { 255 } else { 0 }])
            .collect();
        let options = BulgeOptions {
            strength: 1.0,
            ..BulgeOptions::default()
        };
        let field = bulge_height(w as u32, h as u32, &rgba, &options).unwrap();
        let reference = reference_inflate(&mask, w, h, 1500);
        let peak = reference.iter().fold(0.0f32, |a, &b| a.max(b)).sqrt();
        let error = field
            .values
            .iter()
            .zip(&reference)
            .map(|(v, r)| (v - r.max(0.0).sqrt()).abs())
            .fold(0.0f32, f32::max);
        assert!(
            error < 0.02 * peak,
            "{name}: height differs by {error} (peak {peak})"
        );
    }
}
