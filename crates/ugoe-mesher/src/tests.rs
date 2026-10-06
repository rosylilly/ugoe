use super::*;

/// `inside(x, y)` が真の画素を不透明 (アルファ 255)、それ以外を透明にした画像。
fn image(width: u32, height: u32, inside: impl Fn(f32, f32) -> bool) -> Vec<u8> {
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    for y in 0..height {
        for x in 0..width {
            if inside(x as f32 + 0.5, y as f32 + 0.5) {
                let i = ((y * width + x) * 4) as usize;
                rgba[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
    }
    rgba
}

fn signed_area2(a: Vec2, b: Vec2, c: Vec2) -> f64 {
    let (a, b, c) = (a.as_dvec2(), b.as_dvec2(), c.as_dvec2());
    (b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y)
}

/// メッシュの健全性と被覆を検証し、メッシュを返す。
fn check(rgba: &[u8], width: u32, height: u32, options: &MeshOptions) -> GeneratedMesh {
    let mesh = generate(rgba, width, height, options).expect("mesh");
    assert!(!mesh.indices.is_empty());
    assert_eq!(mesh.indices.len() % 3, 0);
    assert!(mesh.vertices.len() <= MAX_VERTICES);
    let mut used = vec![false; mesh.vertices.len()];
    let tris: Vec<[Vec2; 3]> = mesh
        .indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|t| {
            for &i in t {
                assert!((i as usize) < mesh.vertices.len(), "index out of range");
                used[i as usize] = true;
            }
            [0, 1, 2].map(|k| mesh.vertices[t[k] as usize])
        })
        .collect();
    assert!(used.iter().all(|&u| u), "unreferenced vertex");
    for t in &tris {
        let area = signed_area2(t[0], t[1], t[2]);
        // 画面上で反時計回り (Y 下向きで符号付き面積が負) かつ退化していない
        assert!(
            area < -1e-9,
            "degenerate or back-facing triangle {t:?}: {area}"
        );
    }
    // 不透明な画素の中心は、いずれかの三角形に含まれる (三角形ごとに外接矩形内の画素を塗る)
    let (w, h) = (width as i64, height as i64);
    let mut covered = vec![false; (w * h) as usize];
    for t in &tris {
        let lo = t[0].min(t[1]).min(t[2]);
        let hi = t[0].max(t[1]).max(t[2]);
        for y in ((lo.y - 0.5).floor() as i64).max(0)..=((hi.y - 0.5).ceil() as i64).min(h - 1) {
            for x in ((lo.x - 0.5).floor() as i64).max(0)..=((hi.x - 0.5).ceil() as i64).min(w - 1)
            {
                let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                if [(0, 1), (1, 2), (2, 0)]
                    .iter()
                    .all(|&(i, j)| signed_area2(t[i], t[j], p) <= 1e-9)
                {
                    covered[(y * w + x) as usize] = true;
                }
            }
        }
    }
    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) as usize;
            if rgba[i * 4 + 3] >= options.alpha_threshold {
                assert!(covered[i], "pixel ({x}, {y}) is not covered");
            }
        }
    }
    mesh
}

fn total_area(mesh: &GeneratedMesh) -> f64 {
    mesh.indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|t| {
            -signed_area2(
                mesh.vertices[t[0] as usize],
                mesh.vertices[t[1] as usize],
                mesh.vertices[t[2] as usize],
            ) / 2.0
        })
        .sum()
}

fn covers(mesh: &GeneratedMesh, p: Vec2) -> bool {
    mesh.indices.as_chunks::<3>().0.iter().any(|t| {
        let v = [0, 1, 2].map(|k| mesh.vertices[t[k] as usize]);
        [(0, 1), (1, 2), (2, 0)]
            .iter()
            .all(|&(i, j)| signed_area2(v[i], v[j], p) <= 0.0)
    })
}

#[test]
fn fully_transparent_is_empty() {
    let rgba = vec![0u8; 16 * 16 * 4];
    assert!(matches!(
        generate(&rgba, 16, 16, &MeshOptions::default()),
        Err(MeshError::Empty)
    ));
}

#[test]
fn below_threshold_is_empty() {
    let mut rgba = vec![0u8; 8 * 8 * 4];
    rgba[3] = 7;
    assert!(matches!(
        generate(&rgba, 8, 8, &MeshOptions::default()),
        Err(MeshError::Empty)
    ));
}

#[test]
fn error_codes_are_the_variant_names() {
    let codes = [
        MeshError::Empty,
        MeshError::BadImage {
            width: 2,
            height: 2,
            len: 12,
        },
        MeshError::InvalidOptions {
            option: MeshOption::Margin,
            value: -1.0,
        },
        MeshError::TooComplex,
        MeshError::TooLarge {
            width: 1,
            height: 1,
        },
    ]
    .map(|e| e.code());
    assert_eq!(
        codes,
        [
            "empty",
            "bad_image",
            "invalid_options",
            "too_complex",
            "too_large"
        ]
    );
}

#[test]
fn bad_image_size_is_rejected() {
    assert!(matches!(
        generate(&[0; 12], 2, 2, &MeshOptions::default()),
        Err(MeshError::BadImage { .. })
    ));
}

#[test]
fn invalid_options_are_rejected() {
    let rgba = image(4, 4, |_, _| true);
    for (options, option, value) in [
        (
            MeshOptions {
                margin: -1.0,
                ..MeshOptions::default()
            },
            MeshOption::Margin,
            -1.0,
        ),
        (
            MeshOptions {
                edge_spacing: 0.0,
                ..MeshOptions::default()
            },
            MeshOption::EdgeSpacing,
            0.0,
        ),
        (
            MeshOptions {
                interior_spacing: -0.5,
                ..MeshOptions::default()
            },
            MeshOption::InteriorSpacing,
            -0.5,
        ),
    ] {
        assert_eq!(
            generate(&rgba, 4, 4, &options),
            Err(MeshError::InvalidOptions { option, value })
        );
    }
}

/// 理由はどのフィールドがどの値で外れたかを持つ (NaN は値を比べられないので取り出して確かめる)。
#[test]
fn invalid_options_tell_the_field_and_the_value() {
    let options = MeshOptions {
        interior_spacing: f32::NAN,
        ..MeshOptions::default()
    };
    let Err(MeshError::InvalidOptions { option, value }) = options.validate() else {
        panic!("NaN must be rejected");
    };
    assert_eq!(option, MeshOption::InteriorSpacing);
    assert!(value.is_nan());
    // 複数のフィールドが外れていれば、最初 (`margin`・`edge_spacing`・`interior_spacing` の順) のもの。
    let options = MeshOptions {
        edge_spacing: f32::INFINITY,
        interior_spacing: -1.0,
        ..MeshOptions::default()
    };
    assert_eq!(
        options.validate(),
        Err(MeshError::InvalidOptions {
            option: MeshOption::EdgeSpacing,
            value: f32::INFINITY,
        })
    );
}

#[test]
fn option_bounds_and_names() {
    use std::ops::Bound;
    assert_eq!(
        MeshOption::Margin.bounds(),
        (Bound::Included(0.0), Bound::Included(1024.0))
    );
    assert_eq!(
        MeshOption::EdgeSpacing.bounds(),
        (Bound::Excluded(0.0), Bound::Unbounded)
    );
    assert_eq!(
        MeshOption::InteriorSpacing.bounds(),
        (Bound::Included(0.0), Bound::Unbounded)
    );
    let names = [
        MeshOption::Margin,
        MeshOption::EdgeSpacing,
        MeshOption::InteriorSpacing,
    ]
    .map(|o| (o.as_str(), o.range_text()));
    assert_eq!(
        names,
        [
            ("margin", "[0, 1024]".to_owned()),
            ("edge_spacing", "(0, inf)".to_owned()),
            ("interior_spacing", "[0, inf)".to_owned()),
        ]
    );
    let error = MeshError::InvalidOptions {
        option: MeshOption::Margin,
        value: 1025.0,
    };
    assert_eq!(
        error.to_string(),
        "invalid mesh option: margin is 1025, but must be finite and in [0, 1024]"
    );
}

#[test]
fn validate_accepts_margin_up_to_the_limit() {
    assert!(MeshOptions::default().validate().is_ok());
    for (margin, ok) in [
        (0.0, true),
        (1024.0, true),
        (1024.5, false),
        (f32::INFINITY, false),
    ] {
        let options = MeshOptions {
            margin,
            ..MeshOptions::default()
        };
        assert_eq!(options.validate().is_ok(), ok, "margin {margin}");
    }
}

/// 外 (JSON など) から来た極端なオプションで、パニック・確保の爆発・終わらない処理にならないこと。
#[test]
fn extreme_options_do_not_blow_up() {
    let rgba = image(8, 8, |x, y| {
        (2.0..6.0).contains(&x) && (2.0..6.0).contains(&y)
    });
    for margin in [1e6, 1e30, f32::MAX] {
        let options = MeshOptions {
            margin,
            ..MeshOptions::default()
        };
        assert!(matches!(
            generate(&rgba, 8, 8, &options),
            Err(MeshError::InvalidOptions {
                option: MeshOption::Margin,
                ..
            })
        ));
    }
    // 間隔が小さすぎれば、広げても上限に収まらない。点を作り切る前に諦める。
    for (edge_spacing, interior_spacing) in [(1e-30, 48.0), (24.0, 1e-30), (f32::MIN_POSITIVE, 0.0)]
    {
        let options = MeshOptions {
            edge_spacing,
            interior_spacing,
            ..MeshOptions::default()
        };
        assert!(matches!(
            generate(&rgba, 8, 8, &options),
            Err(MeshError::TooComplex)
        ));
    }
    // 広げれば収まる程度に小さい間隔は、広げて作る。
    let options = MeshOptions {
        edge_spacing: 1e-3,
        interior_spacing: 1e-3,
        ..MeshOptions::default()
    };
    check(&rgba, 8, 8, &options);
}

#[test]
fn square_is_covered_with_margin() {
    let rgba = image(64, 64, |x, y| {
        (16.0..48.0).contains(&x) && (16.0..48.0).contains(&y)
    });
    let options = MeshOptions::default();
    let mesh = check(&rgba, 64, 64, &options);
    // 外周は margin の範囲に収まる
    for v in &mesh.vertices {
        assert!(
            (10.0..=54.0).contains(&v.x) && (10.0..=54.0).contains(&v.y),
            "{v}"
        );
    }
    // 画素の縁 (16..48) まで覆う
    let area = total_area(&mesh);
    assert!(area >= 32.0 * 32.0, "area {area}");
    // 内部点が置かれている
    assert!(
        mesh.vertices
            .iter()
            .any(|v| (20.0..44.0).contains(&v.x) && (20.0..44.0).contains(&v.y))
    );
}

#[test]
fn square_without_margin_hugs_the_pixels() {
    let rgba = image(40, 40, |x, y| {
        (8.0..32.0).contains(&x) && (8.0..32.0).contains(&y)
    });
    let options = MeshOptions {
        margin: 0.0,
        ..MeshOptions::default()
    };
    let mesh = check(&rgba, 40, 40, &options);
    let area = total_area(&mesh);
    assert!((area - 24.0 * 24.0).abs() < 4.0, "area {area}");
}

#[test]
fn circle_is_covered() {
    let rgba = image(128, 128, |x, y| (x - 64.0).hypot(y - 64.0) < 50.0);
    for margin in [0.0, 1.0, 2.5, 4.0, 10.0] {
        let options = MeshOptions {
            margin,
            ..MeshOptions::default()
        };
        check(&rgba, 128, 128, &options);
    }
}

#[test]
fn ring_keeps_its_hole() {
    let rgba = image(128, 128, |x, y| {
        let r = (x - 64.0).hypot(y - 64.0);
        (30.0..56.0).contains(&r)
    });
    let options = MeshOptions {
        margin: 2.0,
        edge_spacing: 12.0,
        interior_spacing: 16.0,
        ..MeshOptions::default()
    };
    let mesh = check(&rgba, 128, 128, &options);
    assert!(!covers(&mesh, Vec2::new(64.0, 64.0)), "hole is filled");
    assert!(!covers(&mesh, Vec2::new(64.0, 44.0)), "hole is filled");
}

#[test]
fn small_hole_closes_with_large_margin() {
    let rgba = image(64, 64, |x, y| {
        (8.0..56.0).contains(&x) && (8.0..56.0).contains(&y) && (x - 32.0).hypot(y - 32.0) > 3.0
    });
    let mesh = check(&rgba, 64, 64, &MeshOptions::default());
    assert!(covers(&mesh, Vec2::new(32.0, 32.0)));
}

#[test]
fn two_islands_are_one_mesh_with_a_gap() {
    let rgba = image(128, 64, |x, y| {
        (x - 30.0).hypot(y - 32.0) < 20.0 || (x - 98.0).hypot(y - 32.0) < 20.0
    });
    let mesh = check(&rgba, 128, 64, &MeshOptions::default());
    assert!(
        !covers(&mesh, Vec2::new(64.0, 32.0)),
        "gap between islands is filled"
    );
}

#[test]
fn thin_shapes_are_covered() {
    // 1 画素幅の横線・斜め線・縦線
    let rgba = image(96, 96, |x, y| {
        (y - 10.5).abs() < 0.5 && (5.0..90.0).contains(&x)
            || (x - y).abs() < 0.6 && (20.0..80.0).contains(&x)
            || (x - 70.5).abs() < 0.5 && (30.0..90.0).contains(&y)
    });
    for margin in [0.0, 0.5, 1.0, 4.0] {
        let options = MeshOptions {
            margin,
            ..MeshOptions::default()
        };
        check(&rgba, 96, 96, &options);
    }
}

#[test]
fn single_pixel_and_image_border() {
    let rgba = image(5, 5, |x, y| x < 1.0 && y < 1.0);
    for margin in [0.0, 4.0] {
        check(
            &rgba,
            5,
            5,
            &MeshOptions {
                margin,
                ..MeshOptions::default()
            },
        );
    }
    let full = image(7, 3, |_, _| true);
    check(&full, 7, 3, &MeshOptions::default());
}

#[test]
fn noisy_image_is_covered() {
    // 決定的な擬似乱数による散らばった画素 (鞍点や小さな穴を多く含む)
    let mut state = 0x2545_f491_u32;
    let mut rgba = vec![0u8; 48 * 48 * 4];
    for px in rgba.as_chunks_mut::<4>().0 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        px[3] = if state.is_multiple_of(3) { 255 } else { 0 };
    }
    for margin in [0.0, 0.7, 3.0] {
        check(
            &rgba,
            48,
            48,
            &MeshOptions {
                margin,
                edge_spacing: 6.0,
                interior_spacing: 0.0,
                ..MeshOptions::default()
            },
        );
    }
}

#[test]
fn tiny_islands_can_be_dropped() {
    let rgba = image(64, 64, |x, y| {
        (x - 20.0).hypot(y - 32.0) < 10.0 || (x - 55.5).abs() < 1.0 && (y - 32.5).abs() < 1.0
    });
    let options = MeshOptions {
        min_island_area: 16,
        ..MeshOptions::default()
    };
    let mesh = generate(&rgba, 64, 64, &options).expect("mesh");
    assert!(!covers(&mesh, Vec2::new(55.5, 32.5)));
    assert!(covers(&mesh, Vec2::new(20.0, 32.0)));
    // 捨てない設定なら覆う
    let mesh = check(&rgba, 64, 64, &MeshOptions::default());
    assert!(covers(&mesh, Vec2::new(55.5, 32.5)));
    // すべて捨てたら空
    let options = MeshOptions {
        min_island_area: 100_000,
        ..MeshOptions::default()
    };
    assert!(matches!(
        generate(&rgba, 64, 64, &options),
        Err(MeshError::Empty)
    ));
}

#[test]
fn output_is_deterministic() {
    let rgba = image(200, 150, |x, y| {
        let r = (x - 100.0).hypot(y - 75.0);
        (20.0..60.0).contains(&r) || (x - 180.0).hypot(y - 20.0) < 12.0
    });
    let options = MeshOptions::default();
    let a = generate(&rgba, 200, 150, &options).expect("mesh");
    let b = generate(&rgba, 200, 150, &options).expect("mesh");
    assert_eq!(a, b);
}

#[test]
fn vertex_count_is_kept_within_u16() {
    // 間隔 1px では頂点が上限を超えるので、自動で間隔が広がる
    let rgba = image(400, 400, |x, y| (x - 200.0).hypot(y - 200.0) < 190.0);
    let options = MeshOptions {
        margin: 1.0,
        edge_spacing: 1.0,
        interior_spacing: 1.0,
        ..MeshOptions::default()
    };
    let mesh = check(&rgba, 400, 400, &options);
    assert!(mesh.vertices.len() > 1000, "{}", mesh.vertices.len());
}

#[test]
fn alpha_threshold_is_respected() {
    let mut rgba = image(32, 32, |x, y| {
        (8.0..24.0).contains(&x) && (8.0..24.0).contains(&y)
    });
    // 右半分をアルファ 100 にする
    for y in 8..24 {
        for x in 16..24 {
            rgba[(y * 32 + x) * 4 + 3] = 100;
        }
    }
    let options = MeshOptions {
        alpha_threshold: 128,
        margin: 0.0,
        ..MeshOptions::default()
    };
    let mesh = check(&rgba, 32, 32, &options);
    assert!(!covers(&mesh, Vec2::new(20.0, 16.0)));
}

#[test]
fn content_rect_is_tight() {
    let mut rgba = vec![0u8; 10 * 6 * 4];
    for &(x, y) in &[(3usize, 1usize), (7, 2), (5, 4)] {
        rgba[(y * 10 + x) * 4 + 3] = 255;
    }
    // しきい値未満の画素は数えない
    rgba[(5 * 10 + 9) * 4 + 3] = 7;
    assert_eq!(
        Rect::of_content(&rgba, 10, 6, 8),
        Some(Rect {
            x: 3,
            y: 1,
            width: 5,
            height: 4
        })
    );
    assert_eq!(Rect::of_content(&rgba, 10, 6, 0).map(|r| r.width), Some(10));
    assert_eq!(Rect::of_content(&vec![0; 10 * 6 * 4], 10, 6, 1), None);
}

/// 中身の矩形だけの格子で作っても、画像全体の格子で作ったときと同じ出力になること。
#[test]
fn cropping_to_content_keeps_the_output() {
    let (w, h) = (300u32, 220u32);
    let shapes: [Box<dyn Fn(f32, f32) -> bool>; 4] = [
        // 中央の輪と離れた小島
        Box::new(|x, y| {
            let r = (x - 150.0).hypot(y - 100.0);
            (20.0..50.0).contains(&r) || (x - 230.0).hypot(y - 170.0) < 6.0
        }),
        // 隅に寄った形 (矩形の左上が 0 にならない向きと、画像の端に接する向き)
        Box::new(|x, y| x > 260.0 && y > 190.0 || (x - 270.0).hypot(y - 30.0) < 15.0),
        Box::new(|x, y| x < 40.0 && y < 25.0 && (x + y) as i32 % 7 != 0),
        // ばらついた点
        Box::new(|x, y| {
            (60.0..200.0).contains(&x)
                && (40.0..120.0).contains(&y)
                && ((x * 12.9898 + y * 78.233).sin() * 43758.545).fract().abs() > 0.8
        }),
    ];
    let options = [
        MeshOptions::default(),
        MeshOptions {
            margin: 0.0,
            ..MeshOptions::default()
        },
        MeshOptions {
            margin: 30.0,
            edge_spacing: 6.0,
            interior_spacing: 10.0,
            min_island_area: 20,
            ..MeshOptions::default()
        },
    ];
    let full = Rect {
        x: 0,
        y: 0,
        width: w as usize,
        height: h as usize,
    };
    for (i, shape) in shapes.iter().enumerate() {
        let rgba = image(w, h, shape);
        let rect = Rect::of_content(&rgba, w, h, 8).expect("content");
        assert_ne!(rect, full, "shape {i}");
        for (j, options) in options.iter().enumerate() {
            let cropped = generate(&rgba, w, h, options).expect("mesh");
            let whole = generate_in(&rgba, w, options, full).expect("mesh");
            assert_eq!(cropped, whole, "shape {i}, options {j}");
        }
    }
}

#[test]
fn grid_size_is_limited() {
    let rect = |width, height| Rect {
        x: 0,
        y: 0,
        width,
        height,
    };
    // 16384 x 16384 ちょうどまでは受け付ける
    assert_eq!(grid_size(&rect(16380, 16380), 2).ok(), Some((16384, 16384)));
    assert!(matches!(
        grid_size(&rect(16381, 16380), 2),
        Err(MeshError::TooLarge {
            width: 16385,
            height: 16384
        })
    ));
    // 細長くても数は同じ
    assert!(grid_size(&rect(MAX_GRID_CELLS - 4, 1), 2).is_err());
    // 余白を足しても掛けてもあふれない
    let huge = u32::MAX as usize;
    assert!(matches!(
        grid_size(&rect(huge, huge), 1026),
        Err(MeshError::TooLarge { .. })
    ));
}

/// 大きな透明のキャンバスに小さな中身があっても、格子は中身の範囲で作る
/// (画像全体の格子なら 20 バイト x 1600 万セルほどを確保する)。
#[test]
fn large_transparent_canvas_is_cheap() {
    let (w, h) = (4000u32, 4000u32);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for y in 2000..2010 {
        for x in 3000..3020 {
            rgba[(y * w as usize + x) * 4 + 3] = 255;
        }
    }
    let mesh = generate(&rgba, w, h, &MeshOptions::default()).expect("mesh");
    assert!(mesh.vertices.iter().all(|v| v.x > 2990.0 && v.y > 1990.0));
}

/// 1024x1024 のレイヤーでの所要時間の目安: `cargo test -p ugoe-mesher --release -- --ignored --nocapture`
#[test]
#[ignore = "benchmark"]
fn bench_1024_layer() {
    let rgba = image(1024, 1024, |x, y| {
        let r = (x - 512.0).hypot(y - 512.0);
        let wobble = 40.0 * ((y - 512.0).atan2(x - 512.0) * 7.0).sin();
        r < 420.0 + wobble && (x - 600.0).hypot(y - 400.0) > 60.0
            || (x - 80.0).hypot(y - 80.0) < 40.0
    });
    let options = MeshOptions::default();
    let start = std::time::Instant::now();
    let mesh = generate(&rgba, 1024, 1024, &options).expect("mesh");
    eprintln!(
        "1024x1024: {:?}, {} vertices, {} triangles",
        start.elapsed(),
        mesh.vertices.len(),
        mesh.indices.len() / 3
    );
    // 同じ中身を 4096x4096 の透明なキャンバスに置く (格子は中身の範囲だけなので、ほぼ同じ時間)
    let (w, x0, y0) = (4096usize, 2000usize, 1500usize);
    let mut canvas = vec![0u8; w * w * 4];
    for y in 0..1024 {
        let src = &rgba[y * 1024 * 4..(y + 1) * 1024 * 4];
        let dst = ((y0 + y) * w + x0) * 4;
        canvas[dst..dst + 1024 * 4].copy_from_slice(src);
    }
    let start = std::time::Instant::now();
    let mesh = generate(&canvas, w as u32, w as u32, &options).expect("mesh");
    eprintln!(
        "1024x1024 in 4096x4096: {:?}, {} vertices",
        start.elapsed(),
        mesh.vertices.len()
    );
}
