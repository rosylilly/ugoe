//! `ugoe_psd::import` の結合テスト。入力は `common` のライターで組み立てる。

mod common;

use std::path::PathBuf;

use common::{Layer as W, pattern, write_psd};
use ugoe_psd::{ImportWarning, Layer, LayerNode, PsdError, WarningKind, import};

const WIDTH: u32 = 64;
const HEIGHT: u32 = 48;

/// `assets/fixtures/layers.psd` の中身 (下から上)。
fn fixture_layers() -> Vec<W> {
    vec![
        W {
            id: Some(1),
            ..W::raster("背景", 0, 0, WIDTH, HEIGHT)
        },
        W::group_end(),
        W {
            id: Some(3),
            ..W::raster("胴", 10, 20, 20, 24)
        },
        W::group_end(),
        W {
            id: Some(5),
            compression: 0,
            ..W::raster("肌", 16, 4, 32, 20)
        },
        W {
            id: Some(6),
            clipping: true,
            fill_opacity: Some(204),
            blend: *b"mul ",
            ..W::raster("影", 20, 8, 8, 8)
        },
        W {
            id: Some(7),
            hidden: true,
            ..W::raster("目", 22, 10, 6, 4)
        },
        W {
            id: Some(8),
            ..W::group("顔")
        },
        W {
            id: Some(9),
            opacity: 230,
            // 閉じたフォルダで、`lsct` にブレンドモードを持たない。
            section: Some((2, None)),
            ..W::group("体")
        },
        W {
            id: Some(10),
            opacity: 128,
            extra: vec![(*b"lrFX", vec![0, 0, 0, 0])],
            ..W::raster("メモ", -4, -2, 12, 8)
        },
        W {
            id: Some(11),
            extra: vec![(*b"levl", vec![0, 2])],
            ..W::new("レベル補正")
        },
    ]
}

fn fixture_bytes() -> Vec<u8> {
    // 合成画像は背景レイヤーをそのまま使う (取り込みでは読まない)。
    let composite: Vec<u8> = pattern(WIDTH, HEIGHT, "背景".len() as u8)
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| [p[0], p[1], p[2]])
        .collect();
    write_psd(WIDTH, HEIGHT, &fixture_layers(), &composite)
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/fixtures/layers.psd")
}

/// コミットしたフィクスチャが、ライターの出力と一致すること。
/// 作り直すときは `UGOE_UPDATE_FIXTURES=1 cargo test -p ugoe-psd`。
#[test]
fn committed_fixture_is_up_to_date() {
    let bytes = fixture_bytes();
    let path = fixture_path();
    if std::env::var_os("UGOE_UPDATE_FIXTURES").is_some() {
        // 並行して読むテストが途中の中身を見ないよう、書いてから置き換える。
        let tmp = path.with_extension("psd.tmp");
        std::fs::write(&tmp, &bytes).unwrap();
        std::fs::rename(&tmp, &path).unwrap();
    }
    let committed = std::fs::read(&path).unwrap();
    assert!(committed.len() < 1024 * 1024);
    assert!(committed == bytes, "{} is stale", path.display());
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
    let tree = import(&std::fs::read(fixture_path()).unwrap()).unwrap();
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
        [
            ImportWarning {
                path: vec!["メモ".into()],
                kind: WarningKind::LayerEffectsIgnored,
            },
            ImportWarning {
                path: vec!["レベル補正".into()],
                kind: WarningKind::AdjustmentLayer { key: "levl".into() },
            },
        ]
    );
    assert_eq!(
        tree.warnings[1].to_string(),
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
    let tree = import(&write_psd(8, 8, &layers, &[0; 8 * 8 * 3])).unwrap();
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

#[test]
fn smart_object_with_pixels_is_imported() {
    let layers = vec![W {
        extra: vec![(*b"PlLd", vec![0; 4])],
        ..W::raster("placed", 0, 0, 2, 2)
    }];
    let tree = import(&write_psd(4, 4, &layers, &[0; 4 * 4 * 3])).unwrap();
    assert_eq!(layer(&tree.root[0]).rgba, pattern(2, 2, 6));
    assert!(tree.warnings.is_empty());
}

#[test]
fn empty_layers_are_kept() {
    let tree = import(&write_psd(4, 4, &[W::new("empty")], &[0; 4 * 4 * 3])).unwrap();
    let empty = layer(&tree.root[0]);
    assert_eq!((empty.width, empty.height), (0, 0));
    assert!(empty.rgba.is_empty());
    assert!(tree.warnings.is_empty());
}

#[test]
fn zip_compressed_layers_are_skipped_with_a_warning() {
    let layers = vec![
        W::raster("kept", 0, 0, 2, 2),
        W {
            compression: 2,
            ..W::raster("zip", 0, 0, 2, 2)
        },
    ];
    let tree = import(&write_psd(4, 4, &layers, &[0; 4 * 4 * 3])).unwrap();
    assert_eq!(tree.root.len(), 1);
    assert_eq!(
        tree.warnings[0].kind,
        WarningKind::UnsupportedCompression { compression: 2 }
    );
}

#[test]
fn flat_psd_becomes_one_layer() {
    let rgb: Vec<u8> = (0..3 * 3 * 2).map(|i| i as u8).collect();
    let tree = import(&write_psd(3, 2, &[], &rgb)).unwrap();
    let bg = layer(&tree.root[0]);
    assert_eq!((bg.width, bg.height), (3, 2));
    let expected: Vec<u8> = rgb
        .as_chunks::<3>()
        .0
        .iter()
        .flat_map(|p| [p[0], p[1], p[2], 255])
        .collect();
    assert_eq!(bg.rgba, expected);
    assert_eq!(tree.warnings[0].kind, WarningKind::FlattenedImage);
}

#[test]
fn unbalanced_groups_are_invalid() {
    let layers = vec![W::group_end(), W::raster("a", 0, 0, 1, 1)];
    let err = import(&write_psd(2, 2, &layers, &[0; 12])).unwrap_err();
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
    let len = u32::from_be_bytes([bytes[34], bytes[35], bytes[36], bytes[37]]) as usize;
    38 + len
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
