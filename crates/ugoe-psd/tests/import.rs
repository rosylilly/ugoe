//! `ugoe_psd::import` の結合テスト。入力は `common` のライターで組み立てる。
//!
//! 取り込みのエラー処理を試す壊れた入力・未対応の中身は、ファイルにせずここで組み立てる。

mod common;

use common::fixtures::{LAYERS_HEIGHT, LAYERS_WIDTH, fixture_path, layers_bytes};
use common::{
    Layer as W, levels_identity, pattern, write_flat_psd, write_psd, write_psd_unchecked,
};
use ugoe_psd::{ImportWarning, Layer, LayerNode, PsdError, WarningKind, import};

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
        W {
            mask: true,
            ..W::raster("masked", 1, 1, 4, 4)
        },
        W::group("inner"),
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
    assert_eq!(layer(&inner[0]).name, "masked");
    assert_eq!(
        tree.warnings,
        [
            ImportWarning {
                path: vec!["outer".into(), "inner".into(), "masked".into()],
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
        "outer / inner / masked: layer mask ignored"
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
fn zip_compressed_layers_are_skipped_with_a_warning() {
    // 中身は正しい zlib ストリーム (ZIP・予測付き ZIP)。取り込みは ZIP に対応していない。
    for compression in [2, 3] {
        let layers = vec![
            W::raster("kept", 0, 0, 2, 2),
            W {
                compression,
                ..W::raster("zip", 0, 0, 2, 2)
            },
        ];
        let tree = import(&write_psd(4, 4, &layers)).unwrap();
        assert_eq!(tree.root.len(), 1);
        assert_eq!(
            tree.warnings[0].kind,
            WarningKind::UnsupportedCompression { compression }
        );
    }
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
    unsupported(&header_with(22, &[0, 16]), "16-bit");
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

/// 決定的な擬似乱数でバイトを書き換えても、パニックしないこと。
#[test]
fn corrupted_input_does_not_panic() {
    let original = fixture_bytes();
    let end = layer_section_end(&original);
    let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for _ in 0..3000 {
        let mut bytes = original.clone();
        for _ in 0..1 + next() % 4 {
            let at = (next() as usize) % end;
            bytes[at] = next() as u8;
        }
        let _ = import(&bytes);
    }
}
