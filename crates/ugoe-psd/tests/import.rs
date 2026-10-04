//! `ugoe_psd::import` の結合テスト。入力は `common` のライターで組み立てる。
//!
//! 取り込みのエラー処理を試す壊れた入力・未対応の中身は、ファイルにせずここで組み立てる。

mod common;

use common::fixtures::{LAYERS_HEIGHT, LAYERS_WIDTH, fixture_path, layers_bytes};
use common::{
    Layer as W, Mask, levels_identity, pattern, write_flat_psd, write_psd, write_psd_unchecked,
    write_psd16, zlib,
};
use ugoe_psd::{
    DEFAULT_MAX_CHANNEL_DECOMPRESSED, DEFAULT_MAX_TOTAL_DECOMPRESSED, ImportLimits, ImportWarning,
    Layer, LayerNode, PsdError, WarningKind, import, import_with,
};

const WIDTH: u32 = LAYERS_WIDTH;
const HEIGHT: u32 = LAYERS_HEIGHT;

fn fixture_bytes() -> Vec<u8> {
    layers_bytes()
}

fn group(node: &LayerNode) -> (&str, &[LayerNode]) {
    match node {
        LayerNode::Group { name, children, .. } => (name, children),
        LayerNode::Layer(l) => panic!("expected a group, found layer {}", l.name),
    }
}

fn layer(node: &LayerNode) -> &Layer {
    match node {
        LayerNode::Layer(l) => l,
        LayerNode::Group { name, .. } => panic!("expected a layer, found group {name}"),
    }
}

#[test]
fn imports_fixture_tree() {
    let tree = import(&std::fs::read(fixture_path("layers.psd")).unwrap()).unwrap();
    assert_eq!((tree.width, tree.height), (WIDTH, HEIGHT));

    // 下から: 背景, 体, メモ (調整レイヤーは取り込まない)。
    assert_eq!(tree.root.len(), 3);
    let bg = layer(&tree.root[0]);
    assert_eq!(bg.name, "背景");
    assert_eq!((bg.width, bg.height), (WIDTH, HEIGHT));
    assert_eq!(bg.rgba, pattern(WIDTH, HEIGHT, "背景".len() as u8));
    assert_eq!(bg.id, Some(1));

    let LayerNode::Group {
        name,
        visible,
        children,
        id,
        opacity,
        blend_mode,
        clipping,
    } = &tree.root[1]
    else {
        panic!("expected a group");
    };
    assert_eq!(name, "体");
    assert!(visible);
    assert_eq!(*id, Some(9));
    assert!((opacity - 230.0 / 255.0).abs() < 1e-6);
    assert_eq!(blend_mode, "norm");
    assert!(!clipping);
    assert_eq!(children.len(), 2);

    let body = layer(&children[0]);
    assert_eq!(body.name, "胴");
    assert_eq!(body.offset, ugoe_core::Vec2::new(10.0, 20.0));
    assert_eq!((body.width, body.height), (20, 24));
    assert_eq!(body.rgba, pattern(20, 24, "胴".len() as u8));

    let (face_name, face) = group(&children[1]);
    assert_eq!(face_name, "顔");
    if let LayerNode::Group { blend_mode, id, .. } = &children[1] {
        assert_eq!(blend_mode, "pass");
        assert_eq!(*id, Some(8));
    }
    let names: Vec<&str> = face.iter().map(|n| layer(n).name.as_str()).collect();
    assert_eq!(names, ["肌", "影", "目"]);

    // 無圧縮のチャンネル。
    let skin = layer(&face[0]);
    assert_eq!(skin.offset, ugoe_core::Vec2::new(16.0, 4.0));
    assert_eq!(skin.rgba, pattern(32, 20, "肌".len() as u8));
    assert!(!skin.clipping);

    let shade = layer(&face[1]);
    assert!(shade.clipping);
    assert_eq!(shade.blend_mode, "mul");
    assert!((shade.fill_opacity - 0.8).abs() < 1e-6);
    assert_eq!(shade.opacity, 1.0);

    let eye = layer(&face[2]);
    assert!(!eye.visible);
    assert!(skin.visible && shade.visible);

    let memo = layer(&tree.root[2]);
    assert_eq!(memo.name, "メモ");
    assert_eq!(memo.offset, ugoe_core::Vec2::new(-4.0, -2.0));
    assert!((memo.opacity - 128.0 / 255.0).abs() < 1e-6);
    assert_eq!(memo.fill_opacity, 1.0);
    assert_eq!(memo.rgba.len(), 12 * 8 * 4);

    assert_eq!(
        tree.warnings,
        [ImportWarning {
            path: vec!["レベル補正".into()],
            kind: WarningKind::AdjustmentLayer { key: "levl".into() },
        }]
    );
    assert_eq!(
        tree.warnings[0].to_string(),
        "レベル補正: adjustment or fill layer (levl) skipped"
    );
}

#[test]
fn warnings_carry_the_group_path() {
    let layers = vec![
        W::group_end(),
        W::group_end(),
        W::raster("plain", 1, 1, 4, 4),
        W {
            // グループのマスクは取り込めない。
            mask: Some(Mask::window(0, 0, 4, 4)),
            ..W::group("inner")
        },
        W {
            // 合成済みの画素を持たないスマートオブジェクト。
            extra: vec![(*b"SoLd", vec![0; 4])],
            ..W::new("smart")
        },
        W::group("outer"),
    ];
    let tree = import(&write_psd(8, 8, &layers)).unwrap();
    let (_, outer) = group(&tree.root[0]);
    assert_eq!(outer.len(), 1, "the smart object is skipped");
    let (_, inner) = group(&outer[0]);
    assert_eq!(layer(&inner[0]).name, "plain");
    assert_eq!(
        tree.warnings,
        [
            ImportWarning {
                path: vec!["outer".into(), "inner".into()],
                kind: WarningKind::LayerMaskIgnored,
            },
            ImportWarning {
                path: vec!["outer".into(), "smart".into()],
                kind: WarningKind::SmartObjectWithoutPixels,
            },
        ]
    );
    assert_eq!(
        tree.warnings[0].to_string(),
        "outer / inner: layer mask ignored"
    );
}

/// レイヤー効果は、追加情報のキーがあれば警告する。中身は読まないので、ここでは中身を省いた
/// (仕様どおりでない) 追加情報で試す。
#[test]
fn layer_effects_are_ignored_with_a_warning() {
    for key in [*b"lrFX", *b"lfx2"] {
        let layers = vec![W {
            opacity: 128,
            extra: vec![(key, Vec::new())],
            ..W::raster("fx", -1, -1, 3, 3)
        }];
        let tree = import(&write_psd(4, 4, &layers)).unwrap();
        assert_eq!(layer(&tree.root[0]).rgba, pattern(3, 3, 2));
        assert_eq!(
            tree.warnings,
            [ImportWarning {
                path: vec!["fx".into()],
                kind: WarningKind::LayerEffectsIgnored,
            }]
        );
        assert_eq!(tree.warnings[0].to_string(), "fx: layer effects ignored");
    }
}

#[test]
fn adjustment_layers_are_skipped_with_a_warning() {
    // 仕様どおりのレベル補正と、中身を省いた (壊れた) 追加情報のどちらでも、画素を持たないので飛ばす。
    for (key, data) in [(*b"levl", levels_identity()), (*b"curv", vec![0, 1])] {
        let layers = vec![
            W::raster("kept", 0, 0, 2, 2),
            W {
                extra: vec![(key, data)],
                ..W::new("adjust")
            },
        ];
        let tree = import(&write_psd(4, 4, &layers)).unwrap();
        assert_eq!(tree.root.len(), 1);
        let key = String::from_utf8_lossy(&key).into_owned();
        assert_eq!(
            tree.warnings,
            [ImportWarning {
                path: vec!["adjust".into()],
                kind: WarningKind::AdjustmentLayer { key },
            }]
        );
    }
}

#[test]
fn smart_object_with_pixels_is_imported() {
    let layers = vec![W {
        extra: vec![(*b"PlLd", vec![0; 4])],
        ..W::raster("placed", 0, 0, 2, 2)
    }];
    let tree = import(&write_psd(4, 4, &layers)).unwrap();
    assert_eq!(layer(&tree.root[0]).rgba, pattern(2, 2, 6));
    assert!(tree.warnings.is_empty());
}

#[test]
fn empty_layers_are_kept() {
    let tree = import(&write_psd(4, 4, &[W::new("empty")])).unwrap();
    let empty = layer(&tree.root[0]);
    assert_eq!((empty.width, empty.height), (0, 0));
    assert!(empty.rgba.is_empty());
    assert!(tree.warnings.is_empty());
}

#[test]
fn zip_compressed_layers_are_decoded() {
    // ZIP・予測付き ZIP。
    for compression in [2, 3] {
        let layers = vec![W {
            compression,
            ..W::raster("zip", -1, 1, 5, 3)
        }];
        let tree = import(&write_psd(4, 4, &layers)).unwrap();
        assert_eq!(layer(&tree.root[0]).rgba, pattern(5, 3, 3), "{compression}");
        assert!(tree.warnings.is_empty());
    }
}

#[test]
fn broken_zip_data_is_invalid() {
    let bytes = write_psd(
        4,
        4,
        &[W {
            compression: 2,
            ..W::raster("zip", 0, 0, 4, 4)
        }],
    );
    // 最初のチャンネルの zlib ストリームの中ほどを壊す。
    let start = bytes
        .windows(4)
        .position(|w| w == [0, 2, 0x78, 0x9c])
        .unwrap();
    for at in [start + 2, start + 5] {
        let mut broken = bytes.clone();
        broken[at] ^= 0xff;
        let err = import(&broken).unwrap_err();
        assert!(matches!(err, PsdError::Invalid(_)), "{err:?}");
    }
}

/// ZIP で圧縮したチャンネルが 0 の画素 `width` × `height` に展開されるレイヤー。画素は用意しない。
fn zip_bomb(name: &str, width: u32, height: u32, stream: &[u8]) -> W {
    let mut data = vec![0, 2];
    data.extend_from_slice(stream);
    W {
        width,
        height,
        compression: 2,
        channel_data: Some(data),
        ..W::new(name)
    }
}

fn assert_over_limit(result: Result<ugoe_psd::LayerTree, PsdError>) {
    let Err(err) = result else {
        panic!("expected the import limit to be exceeded");
    };
    assert!(
        matches!(&err, PsdError::Unsupported(m) if m.contains("import limit")),
        "{err:?}"
    );
}

#[test]
fn zip_bomb_channel_is_rejected_before_inflating() {
    // 1 チャンネルが 40000 × 40000 = 1.6 GB に展開されると宣言したレイヤー。既定の上限 (1 GiB) を超えるので、
    // ストリームを展開する前 (画素を確保する前) に止まる。
    let bytes = write_psd(4, 4, &[zip_bomb("bomb", 40_000, 40_000, &zlib(&[0; 1024]))]);
    assert_over_limit(import(&bytes));
    // 上限を上げると展開に進み、ストリームが宣言より短いので壊れた入力になる (止めていたのは上限)。
    let limits = ImportLimits::default()
        .with_max_channel_decompressed(u64::MAX)
        .with_max_total_decompressed(u64::MAX);
    let err = import_with(&bytes, limits).unwrap_err();
    assert!(matches!(err, PsdError::Invalid(_)), "{err:?}");
}

#[test]
fn zip_bomb_layers_stop_at_the_total_limit() {
    // 本物の ZIP 爆弾: 1 MiB の 0 が約 1 KiB に縮む。64 レイヤー × 4 チャンネルで 256 MiB に展開される。
    let stream = zlib(&vec![0; 1024 * 1024]);
    assert!(stream.len() < 2048, "{}", stream.len());
    let layers: Vec<W> = (0..64)
        .map(|i| zip_bomb(&format!("bomb {i}"), 1024, 1024, &stream))
        .collect();
    let bytes = write_psd(4, 4, &layers);
    assert!(bytes.len() < 1024 * 1024, "{}", bytes.len());
    let limit = 16 * 1024 * 1024;
    assert_over_limit(import_with(
        &bytes,
        ImportLimits::default().with_max_total_decompressed(limit),
    ));
    // 4 レイヤー分 (16 MiB) ちょうどなら読める。
    let tree = import_with(
        &write_psd(4, 4, &layers[..4]),
        ImportLimits::default().with_max_total_decompressed(limit),
    )
    .unwrap();
    assert_eq!(tree.root.len(), 4);
    assert_eq!(layer(&tree.root[3]).rgba, vec![0; 1024 * 1024 * 4]);
}

#[test]
fn import_limits_count_decompressed_bytes_of_every_compression() {
    let limits = |channel: u64, total: u64| {
        ImportLimits::default()
            .with_max_channel_decompressed(channel)
            .with_max_total_decompressed(total)
    };
    // 16 × 16 のレイヤーは 1 チャンネル 256 バイト、4 チャンネルで 1024 バイト。
    for compression in 0..=3 {
        let layers = [W {
            compression,
            ..W::raster("a", 0, 0, 16, 16)
        }];
        let bytes = write_psd(16, 16, &layers);
        assert!(
            import_with(&bytes, limits(256, 1024)).is_ok(),
            "{compression}"
        );
        assert_over_limit(import_with(&bytes, limits(255, 1024)));
        assert_over_limit(import_with(&bytes, limits(256, 1023)));
        // 16 ビットは標本が 2 バイト。
        let wide = write_psd16(16, 16, &layers);
        assert!(
            import_with(&wide, limits(512, 2048)).is_ok(),
            "{compression}"
        );
        assert_over_limit(import_with(&wide, limits(511, 2048)));
    }
    // マスクのチャンネルも数える (4 × 4 のマスクで 16 バイト)。
    let masked = write_psd(
        16,
        16,
        &[W {
            mask: Some(Mask::window(0, 0, 4, 4)),
            ..W::raster("a", 0, 0, 16, 16)
        }],
    );
    assert!(import_with(&masked, limits(256, 1024 + 16)).is_ok());
    assert_over_limit(import_with(&masked, limits(256, 1024 + 15)));
    // 合成画像 (レイヤーのない PSD) は 3 チャンネル。
    let flat = write_flat_psd(16, 16, &[0; 16 * 16 * 3], 1);
    assert!(import_with(&flat, limits(256, 768)).is_ok());
    assert_over_limit(import_with(&flat, limits(256, 767)));
}

#[test]
fn default_import_limits() {
    let limits = ImportLimits::default();
    assert_eq!(
        limits.max_channel_decompressed,
        DEFAULT_MAX_CHANNEL_DECOMPRESSED
    );
    assert_eq!(
        limits.max_total_decompressed,
        DEFAULT_MAX_TOTAL_DECOMPRESSED
    );
    // PSD の最大のキャンバスを覆う 8 ビットのチャンネルは既定で読める。
    const { assert!(30_000 * 30_000 <= DEFAULT_MAX_CHANNEL_DECOMPRESSED) };
}

#[test]
fn unknown_compression_is_skipped_with_a_warning() {
    let mut bytes = write_psd(
        4,
        4,
        &[W {
            compression: 0,
            ..W::raster("odd", 0, 0, 2, 2)
        }],
    );
    // 最初のチャンネル (アルファ) の圧縮方式を、仕様にない 7 に書き換える。
    let end = layer_section_end(&bytes);
    let at = bytes[..end]
        .windows(6)
        .position(|w| w == [0, 0, 0, 255, 0, 252])
        .unwrap();
    bytes[at + 1] = 7;
    let tree = import(&bytes).unwrap();
    assert!(tree.root.is_empty());
    assert_eq!(
        tree.warnings[0].kind,
        WarningKind::UnsupportedCompression { compression: 7 }
    );
}

/// 期待するアルファ: 模様のアルファに (濃度を掛けた) マスクの値を掛けて丸める。
fn masked_alpha(rgba: &[u8], left: i32, top: i32, width: u32, mask: &Mask) -> Vec<u8> {
    rgba.as_chunks::<4>()
        .0
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let x = left + (i as u32 % width) as i32;
            let y = top + (i as u32 / width) as i32;
            let hide = 255 - u32::from(mask.value(x, y));
            let m = 255 - (hide * u32::from(mask.density.unwrap_or(255)) + 127) / 255;
            ((u32::from(p[3]) * m + 127) / 255) as u8
        })
        .collect()
}

fn alpha(rgba: &[u8]) -> Vec<u8> {
    rgba.as_chunks::<4>().0.iter().map(|p| p[3]).collect()
}

#[test]
fn layer_masks_are_applied_to_alpha() {
    // レイヤー (1, 1)-(7, 6) に対して、左上にはみ出し、中の値が変化するマスク。
    let gradient = Mask {
        pixels: (0..5 * 4).map(|i| (i * 13) as u8).collect(),
        default_color: 255,
        ..Mask::window(-1, 0, 5, 4)
    };
    // レイヤーの中の 2x2 だけを見せるマスク。
    let window = Mask::window(3, 2, 2, 2);
    for mask in [gradient, window] {
        for compression in [0, 1, 2, 3] {
            let raw = W {
                compression,
                mask: Some(mask.clone()),
                ..W::raster("masked", 1, 1, 6, 5)
            };
            let tree = import(&write_psd(8, 8, std::slice::from_ref(&raw))).unwrap();
            assert!(tree.warnings.is_empty(), "{:?}", tree.warnings);
            let got = layer(&tree.root[0]);
            let written = raw.rgba.as_ref().unwrap();
            assert_eq!(alpha(&got.rgba), masked_alpha(written, 1, 1, 6, &mask));
            // 色はそのまま。
            for (g, w) in got
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .zip(written.as_chunks::<4>().0)
            {
                assert_eq!(g[..3], w[..3]);
            }
        }
    }
}

#[test]
fn mask_density_weakens_the_mask() {
    let mask = Mask {
        density: Some(100),
        ..Mask::window(2, 2, 2, 2)
    };
    let raw = W {
        mask: Some(mask.clone()),
        ..W::raster("dense", 0, 0, 5, 5)
    };
    let tree = import(&write_psd(6, 6, std::slice::from_ref(&raw))).unwrap();
    let got = alpha(&layer(&tree.root[0]).rgba);
    assert_eq!(
        got,
        masked_alpha(raw.rgba.as_ref().unwrap(), 0, 0, 5, &mask)
    );
    // 隠す量が 100/255 になる: マスクの外の不透明な画素 (1, 0) は 255 - 100 = 155。
    assert_eq!(got[1], 155);
}

/// ベクトルマスクと併用したレイヤーは、両方を合成した -3 だけを掛ける。
#[test]
fn real_user_mask_replaces_the_user_mask() {
    let real = Mask {
        pixels: (0..4 * 3).map(|i| (i * 20) as u8).collect(),
        default_color: 255,
        ..Mask::window(1, 1, 4, 3)
    };
    for compression in [1, 2] {
        let raw = W {
            compression,
            // -2 は何も見せないマスク。-3 を使えばこれは効かない。
            mask: Some(Mask::window(0, 0, 1, 1)),
            real_mask: Some(real.clone()),
            ..W::raster("both", 0, 0, 6, 5)
        };
        let tree = import(&write_psd(6, 6, std::slice::from_ref(&raw))).unwrap();
        assert!(tree.warnings.is_empty());
        assert_eq!(
            alpha(&layer(&tree.root[0]).rgba),
            masked_alpha(raw.rgba.as_ref().unwrap(), 0, 0, 6, &real)
        );
    }
}

#[test]
fn disabled_and_empty_masks_are_ignored() {
    for mask in [
        Mask {
            disabled: true,
            ..Mask::window(0, 0, 2, 2)
        },
        Mask::window(0, 0, 0, 0),
    ] {
        let layers = vec![W {
            mask: Some(mask),
            ..W::raster("m", 0, 0, 3, 3)
        }];
        let tree = import(&write_psd(4, 4, &layers)).unwrap();
        assert_eq!(layer(&tree.root[0]).rgba, pattern(3, 3, 1));
        assert!(tree.warnings.is_empty());
    }
}

#[test]
fn legacy_names_are_decoded() {
    let sjis = |s: &str| encoding_rs::SHIFT_JIS.encode(s).0.into_owned();
    for (bytes, expected) in [
        // SAI などの日本語の環境のツール: Shift-JIS (CP932 の丸数字も)。
        (sjis("前髪①"), "前髪①"),
        (sjis("ｱﾎ毛"), "ｱﾎ毛"),
        // UTF-8 で書くツール。
        ("後ろ髪".as_bytes().to_vec(), "後ろ髪"),
        (b"face".to_vec(), "face"),
        // どちらでもないバイト列は Windows-1252。
        (vec![b'c', 0xe9, b'!'], "c\u{e9}!"),
    ] {
        let layers = vec![W {
            unicode_name: false,
            legacy_name: Some(bytes),
            ..W::raster("ignored", 0, 0, 1, 1)
        }];
        let tree = import(&write_psd(2, 2, &layers)).unwrap();
        assert_eq!(layer(&tree.root[0]).name, expected);
    }
    // `luni` があればそちらを使う。
    let layers = vec![W {
        legacy_name: Some(sjis("古い名前")),
        ..W::raster("新しい名前", 0, 0, 1, 1)
    }];
    let tree = import(&write_psd(2, 2, &layers)).unwrap();
    assert_eq!(layer(&tree.root[0]).name, "新しい名前");
}

/// 16 ビットの PSD (`Lr16`) は、同じ中身の 8 ビットの PSD と同じに取り込める。
#[test]
fn sixteen_bit_documents_match_eight_bit() {
    let mut layers = common::fixtures::layers();
    for (i, l) in layers.iter_mut().enumerate() {
        l.compression = [0, 1, 2, 3][i % 4];
    }
    layers.push(W {
        mask: Some(Mask {
            pixels: (0..16).map(|i| (i * 17) as u8).collect(),
            ..Mask::window(2, 2, 4, 4)
        }),
        compression: 3,
        ..W::raster("masked", 0, 0, 8, 8)
    });
    let a = import(&write_psd(WIDTH, HEIGHT, &layers)).unwrap();
    let b = import(&write_psd16(WIDTH, HEIGHT, &layers)).unwrap();
    assert_eq!(format!("{:?}", a.root), format!("{:?}", b.root));
    assert_eq!(a.warnings, b.warnings);
    assert_eq!(b.root.len(), 4);
}

#[test]
fn flat_psd_becomes_one_layer() {
    let rgb: Vec<u8> = (0..3 * 3 * 2).map(|i| i as u8).collect();
    let expected: Vec<u8> = rgb
        .as_chunks::<3>()
        .0
        .iter()
        .flat_map(|p| [p[0], p[1], p[2], 255])
        .collect();
    // 無圧縮と PackBits。
    for compression in [0, 1] {
        let tree = import(&write_flat_psd(3, 2, &rgb, compression)).unwrap();
        let bg = layer(&tree.root[0]);
        assert_eq!((bg.width, bg.height), (3, 2));
        assert_eq!(bg.rgba, expected);
        assert_eq!(tree.warnings[0].kind, WarningKind::FlattenedImage);
    }
}

#[test]
fn unbalanced_groups_are_invalid() {
    let layers = vec![W::group_end(), W::raster("a", 0, 0, 1, 1)];
    let err = import(&write_psd_unchecked(2, 2, &layers, &[0; 12])).unwrap_err();
    assert!(matches!(err, PsdError::Invalid(_)), "{err:?}");
}

fn header_with(offset: usize, value: &[u8]) -> Vec<u8> {
    let mut bytes = fixture_bytes();
    bytes[offset..offset + value.len()].copy_from_slice(value);
    bytes
}

#[test]
fn rejects_unsupported_documents() {
    let unsupported = |bytes: &[u8], needle: &str| match import(bytes) {
        Err(PsdError::Unsupported(msg)) => assert!(msg.contains(needle), "{msg}"),
        other => panic!("expected unsupported, got {other:?}"),
    };
    unsupported(&header_with(4, &[0, 2]), "psb");
    unsupported(&header_with(22, &[0, 32]), "32-bit");
    unsupported(&header_with(22, &[0, 1]), "1-bit");
    unsupported(&header_with(24, &[0, 4]), "cmyk");
    unsupported(&header_with(24, &[0, 1]), "grayscale");
}

#[test]
fn rejects_malformed_headers() {
    for bytes in [
        b"".to_vec(),
        b"GIF89a".to_vec(),
        header_with(0, b"8BPX"),
        header_with(4, &[0, 3]),
        header_with(14, &[0, 0, 0, 0]),
        header_with(18, &[0xff, 0xff, 0xff, 0xff]),
    ] {
        assert!(
            matches!(import(&bytes), Err(PsdError::Invalid(_))),
            "{:?}",
            &bytes[..bytes.len().min(26)]
        );
    }
}

/// レイヤーとマスクの情報の終わりの位置 (この後は取り込みで読まない合成画像)。
fn layer_section_end(bytes: &[u8]) -> usize {
    let u32_at = |at: usize| {
        u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]) as usize
    };
    // ヘッダ 26 バイトの後に、長さ付きのカラーモードデータ・画像リソース・レイヤーとマスクの情報が続く。
    let resources = 26 + 4 + u32_at(26);
    let layers = resources + 4 + u32_at(resources);
    layers + 4 + u32_at(layers)
}

#[test]
fn truncated_input_is_an_error() {
    let bytes = fixture_bytes();
    let end = layer_section_end(&bytes);
    for len in 0..bytes.len() {
        let result = import(&bytes[..len]);
        if len < end {
            assert!(result.is_err(), "prefix {len} was accepted");
        }
    }
}

/// マスク・ZIP・予測付き ZIP を含む 16 ビットの PSD。
fn feature_bytes() -> Vec<u8> {
    let layers: Vec<W> = (0..4)
        .map(|compression| W {
            compression,
            mask: Some(Mask {
                density: Some(200),
                ..Mask::window(1, 1, 3, 3)
            }),
            real_mask: (compression == 3).then(|| Mask::window(0, 0, 2, 2)),
            ..W::raster("f", 0, 0, 5, 4)
        })
        .collect();
    write_psd16(6, 6, &layers)
}

/// 決定的な擬似乱数でバイトを書き換えても、パニックしないこと。
#[test]
fn corrupted_input_does_not_panic() {
    let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for original in [fixture_bytes(), feature_bytes()] {
        import(&original).unwrap();
        let end = layer_section_end(&original);
        for _ in 0..3000 {
            let mut bytes = original.clone();
            for _ in 0..1 + next() % 4 {
                let at = (next() as usize) % end;
                bytes[at] = next() as u8;
            }
            let _ = import(&bytes);
        }
        for len in 0..end {
            let _ = import(&original[..len]);
        }
    }
}

fn nested_groups(depth: usize) -> Vec<u8> {
    let mut layers = vec![W::group_end(); depth];
    layers.push(W::raster("leaf", 0, 0, 1, 1));
    layers.extend((0..depth).map(|i| W::group(&format!("g{i}"))));
    write_psd_unchecked(2, 2, &layers, &[0; 12])
}

/// 深く入れ子にしたグループ。木を作って捨てる (再帰する) ところでスタックをあふれさせないこと。
#[test]
fn deeply_nested_groups_are_rejected() {
    let tree = import(&nested_groups(256)).unwrap();
    let mut node = &tree.root[0];
    for _ in 0..256 {
        node = &group(node).1[0];
    }
    assert_eq!(layer(node).name, "leaf");
    // レイヤー数の上限 (32767) に近い深さ。
    for depth in [257, 16_000] {
        let err = import(&nested_groups(depth)).unwrap_err();
        assert!(matches!(err, PsdError::Unsupported(_)), "{err:?}");
    }
}
