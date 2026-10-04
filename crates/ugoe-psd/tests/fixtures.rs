//! `assets/fixtures/` の PSD (`layers.psd`・`character.psd`) のテスト。
//!
//! - コミットしたファイルが `common::fixtures` の定義と一致すること
//! - 取り込みの結果
//! - 独立した実装の ImageMagick (`identify` / `convert`) でも読め、レイヤーと合成画像が期待どおりであること
//!   (ImageMagick が無い環境では何もしない)

// テストの補助関数でも unwrap を使う (失敗はテストの失敗)。
#![allow(clippy::unwrap_used)]

mod common;

use std::path::Path;
use std::process::Command;

use common::compose;
use common::fixtures::*;
use ugoe_psd::{Layer, LayerNode, import};

/// コミットしたフィクスチャがライターの出力と一致すること。
/// 作り直すときは `UGOE_UPDATE_FIXTURES=1 cargo test -p ugoe-psd --test fixtures`。
fn check_committed(name: &str, bytes: &[u8]) {
    let path = fixture_path(name);
    if std::env::var_os("UGOE_UPDATE_FIXTURES").is_some() {
        // 並行して読むテストが途中の中身を見ないよう、書いてから置き換える。
        let tmp = path.with_extension("psd.tmp");
        std::fs::write(&tmp, bytes).unwrap();
        std::fs::rename(&tmp, &path).unwrap();
    }
    let committed = std::fs::read(&path).unwrap();
    assert!(committed.len() < 1024 * 1024);
    assert!(committed == bytes, "{} is stale", path.display());
}

#[test]
fn committed_layers_psd_is_up_to_date() {
    check_committed("layers.psd", &layers_bytes());
}

#[test]
fn committed_character_psd_is_up_to_date() {
    check_committed("character.psd", &character_bytes());
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

fn names(nodes: &[LayerNode]) -> Vec<&str> {
    nodes
        .iter()
        .map(|n| match n {
            LayerNode::Group { name, .. } => name.as_str(),
            LayerNode::Layer(l) => l.name.as_str(),
        })
        .collect()
}

#[test]
fn imports_character_tree() {
    let tree = import(&std::fs::read(fixture_path("character.psd")).unwrap()).unwrap();
    assert_eq!(
        (tree.width, tree.height),
        (CHARACTER_WIDTH, CHARACTER_HEIGHT)
    );
    assert_eq!(tree.warnings, []);
    assert_eq!(names(&tree.root), ["背景", "後ろ髪", "体", "頭"]);

    let (_, body) = group(&tree.root[2]);
    assert_eq!(names(body), ["首", "服"]);
    if let LayerNode::Group { blend_mode, .. } = &tree.root[2] {
        assert_eq!(blend_mode, "norm");
    }
    let (_, head) = group(&tree.root[3]);
    assert_eq!(
        names(head),
        ["顔", "顔の影", "頬", "目", "口", "口 (開き)", "前髪"]
    );
    if let LayerNode::Group { blend_mode, .. } = &tree.root[3] {
        assert_eq!(blend_mode, "pass");
    }
    let shade = layer(&head[1]);
    assert!(shade.clipping);
    assert_eq!(shade.blend_mode, "mul");
    assert!((layer(&head[2]).opacity - 128.0 / 255.0).abs() < 1e-6);
    assert!(!layer(&head[5]).visible);
    let (_, eyes) = group(&head[3]);
    assert_eq!(names(eyes), ["右目", "左目"]);

    // 画素とキャンバス上の位置が、描いたものと一致する。
    let right = layer(&eyes[0]);
    let (cx, cy) = RIGHT_EYE;
    assert!(right.offset.x < cx && cx < right.offset.x + right.width as f32);
    assert!(right.offset.y < cy && cy < right.offset.y + right.height as f32);
    let expected = character();
    let written = expected.iter().find(|l| l.name == "右目").unwrap();
    assert_eq!(Some(&right.rgba), written.rgba.as_ref());
}

/// 合成画像のうち、意味の分かる点の色。レイヤーの重なり・クリッピング・不透明度・非表示を確かめる。
fn check_character_composite(rgb: &[u8]) {
    let at = |(x, y): (f32, f32)| {
        let i = (y as usize * CHARACTER_WIDTH as usize + x as usize) * 3;
        [rgb[i], rgb[i + 1], rgb[i + 2]]
    };
    let rgb3 = |c: [u8; 4]| [c[0], c[1], c[2]];
    let mul = |a: [u8; 4], b: [u8; 4]| {
        [0, 1, 2].map(|i| (f32::from(a[i]) * f32::from(b[i]) / 255.0).round() as u8)
    };
    let close = |a: [u8; 3], b: [u8; 3], what: &str| {
        assert!(
            a.iter().zip(&b).all(|(&x, &y)| x.abs_diff(y) <= 1),
            "{what}: {a:?} != {b:?}"
        );
    };
    close(at((4.0, 4.0)), rgb3(BACKGROUND), "background corner");
    close(at((70.0, 250.0)), rgb3(HAIR), "back hair beside the neck");
    close(at((160.0, 360.0)), rgb3(SHIRT), "shirt");
    close(at((160.0, 250.0)), rgb3(SKIN), "neck");
    close(at((160.0, 20.0)), rgb3(BACKGROUND), "above the hair");
    close(at((160.0, 60.0)), rgb3(HAIR), "bangs");
    close(at(FACE_CENTER), rgb3(SKIN), "between the eyes");
    // 顔の影は顔でクリッピングされ、乗算で暗くなる。顔の外 (髪の上) には出ない。
    close(at((160.0, 128.0)), mul(SKIN, SHADE), "shaded forehead");
    close(
        at((80.0, 110.0)),
        rgb3(HAIR),
        "shade clipped outside the face",
    );
    // 頬は不透明度 50%。
    let cheek = at((114.0, 186.0));
    let half = [0, 1, 2].map(|i| {
        let t = 128.0 / 255.0;
        (f32::from(SKIN[i]) * (1.0 - t) + f32::from(CHEEK[i]) * t).round() as u8
    });
    close(cheek, half, "cheek");
    close(
        at((RIGHT_EYE.0, RIGHT_EYE.1 + 4.0)),
        rgb3(PUPIL),
        "right pupil",
    );
    close(
        at((LEFT_EYE.0, LEFT_EYE.1 + 4.0)),
        rgb3(PUPIL),
        "left pupil",
    );
    close(
        at((LEFT_EYE.0 + 14.0, LEFT_EYE.1)),
        rgb3(EYE_WHITE),
        "eye white",
    );
    close(
        at((MOUTH_CENTER.0, MOUTH_CENTER.1 + 8.0)),
        rgb3(MOUTH),
        "mouth",
    );
    // 非表示の「口 (開き)」は描かれない。
    close(at(MOUTH_CENTER), rgb3(SKIN), "hidden open mouth");
}

#[test]
fn character_composite_shows_the_intended_picture() {
    check_character_composite(&compose::flatten(
        CHARACTER_WIDTH,
        CHARACTER_HEIGHT,
        &character(),
    ));
}

// ---- ImageMagick ----

fn imagemagick() -> bool {
    let found = Command::new("identify")
        .arg("-version")
        .output()
        .is_ok_and(|o| o.status.success());
    if !found {
        eprintln!("ImageMagick (identify) not found; skipping");
    }
    found
}

/// ImageMagick が見るレイヤー: (幅, 高さ, 左, 上)。0 番 (合成画像) を除く。
fn identify_layers(path: &Path) -> Vec<(u32, u32, i32, i32)> {
    let output = Command::new("identify")
        .args(["-format", "%w %h %X %Y\n"])
        .arg(path)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .skip(1)
        .map(|line| {
            let v: Vec<&str> = line.split_whitespace().collect();
            (
                v[0].parse().unwrap(),
                v[1].parse().unwrap(),
                v[2].parse().unwrap(),
                v[3].parse().unwrap(),
            )
        })
        .collect()
}

/// ImageMagick が読んだ合成画像 (RGB8)。
fn convert_composite(path: &Path) -> Vec<u8> {
    let output = Command::new("convert")
        .arg(format!("{}[0]", path.display()))
        .args(["-depth", "8", "rgb:-"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    output.stdout
}

/// 画素を持つレイヤー (矩形が空でないもの) の (幅, 高さ, 左, 上)。
fn raster_geometry(layers: &[common::Layer]) -> Vec<(u32, u32, i32, i32)> {
    layers
        .iter()
        .filter(|l| l.width > 0 && l.height > 0)
        .map(|l| (l.width, l.height, l.left, l.top))
        .collect()
}

#[test]
fn imagemagick_reads_layers_psd() {
    if !imagemagick() {
        return;
    }
    let path = fixture_path("layers.psd");
    assert_eq!(identify_layers(&path), raster_geometry(&layers()));
    assert_eq!(
        convert_composite(&path),
        compose::flatten(LAYERS_WIDTH, LAYERS_HEIGHT, &layers())
    );
}

#[test]
fn imagemagick_reads_character_psd() {
    if !imagemagick() {
        return;
    }
    let path = fixture_path("character.psd");
    assert_eq!(identify_layers(&path), raster_geometry(&character()));
    let rgb = convert_composite(&path);
    assert_eq!(
        rgb,
        compose::flatten(CHARACTER_WIDTH, CHARACTER_HEIGHT, &character())
    );
    check_character_composite(&rgb);
}

/// マスク・ZIP・16 ビットを含む PSD を、取り込みと ImageMagick の両方で読み、各レイヤーの画素
/// (マスクを掛けた後) が一致すること。ImageMagick のマスクの掛け算の丸めの違いで 1 ずれてよい。
///
/// マスクの既定の色 (矩形の外の値) は 0 だけを使う。ImageMagick 6.9 は既定の色が 255 のとき、
/// 矩形の外を 255 でなく約 55 (赤だけを 255 にした色の輝度) として扱うため。255 は `tests/import.rs` で試す。
#[test]
fn imagemagick_agrees_on_masks_zip_and_16_bit() {
    if !imagemagick() {
        return;
    }
    let gradient = common::Mask {
        pixels: (0..6 * 5).map(|i| (i * 8) as u8).collect(),
        ..common::Mask::window(-2, 1, 6, 5)
    };
    let layers: Vec<common::Layer> = (0..4)
        .map(|compression| common::Layer {
            compression,
            mask: Some(if compression % 2 == 0 {
                gradient.clone()
            } else {
                common::Mask::window(4 + compression as i32, 3, 3, 4)
            }),
            ..common::Layer::raster(&format!("layer {compression}"), 1, 2, 9, 7)
        })
        .collect();
    let dir = tempfile::tempdir().unwrap();
    for (depth, bytes) in [
        (8, common::write_psd(12, 10, &layers)),
        (16, common::write_psd16(12, 10, &layers)),
    ] {
        let path = dir.path().join(format!("{depth}.psd"));
        std::fs::write(&path, &bytes).unwrap();
        let tree = import(&bytes).unwrap();
        assert!(tree.warnings.is_empty());
        assert_eq!(identify_layers(&path), raster_geometry(&layers));
        for (i, node) in tree.root.iter().enumerate() {
            let ours = &layer(node).rgba;
            let output = Command::new("convert")
                .arg(format!("{}[{}]", path.display(), i + 1))
                .args(["-depth", "8", "rgba:-"])
                .output()
                .unwrap();
            assert!(output.status.success(), "{output:?}");
            let theirs = output.stdout;
            assert_eq!(ours.len(), theirs.len());
            for (p, (a, b)) in ours
                .as_chunks::<4>()
                .0
                .iter()
                .zip(theirs.as_chunks::<4>().0)
                .enumerate()
            {
                let channels = if a[3] == 0 { 3..4 } else { 0..4 };
                for c in channels {
                    assert!(
                        a[c].abs_diff(b[c]) <= 1,
                        "{depth}-bit layer {i} pixel {p}: {a:?} != {b:?}"
                    );
                }
            }
        }
    }
}
