//! 実際のペイントツールが書き出した PSD (`assets/fixtures/psd-real/*.psd`) の取り込みのテスト。
//!
//! ファイルの追加手順とライセンスの記録方法は `assets/fixtures/psd-real/README.md`。
//! ファイルが無ければ何もしない。
//!
//! - 対応する形式 (8/16 ビット RGB の PSD) なら、取り込みが成功すること (警告はあってよい)
//! - 対応しない形式 (PSB・8/16 ビット以外・RGB 以外) なら、`PsdError::Unsupported` になること
//! - ImageMagick (`identify`) があれば、画素を持つレイヤーの数が ImageMagick の見方と一致すること

// テストの補助関数でも unwrap を使う (失敗はテストの失敗)。
#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::process::Command;

use ugoe_psd::{LayerNode, PsdError, WarningKind, import};

fn corpus() -> Vec<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/fixtures/psd-real");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("psd")))
        .collect();
    files.sort();
    files
}

/// ヘッダが、取り込みが対応する形式 (版 1・8/16 ビット・RGB) か。
fn is_supported_format(bytes: &[u8]) -> bool {
    bytes.len() >= 26
        && bytes[..4] == *b"8BPS"
        && bytes[4..6] == [0, 1]
        && (bytes[22..24] == [0, 8] || bytes[22..24] == [0, 16])
        && bytes[24..26] == [0, 3]
}

/// 画素を持つ (大きさが 0 でない) ラスタレイヤーの数。
fn raster_count(nodes: &[LayerNode]) -> usize {
    nodes
        .iter()
        .map(|n| match n {
            LayerNode::Group { children, .. } => raster_count(children),
            LayerNode::Layer(l) => usize::from(l.width > 0 && l.height > 0),
        })
        .sum()
}

/// ImageMagick が読むレイヤーの数 (合成画像を除く)。ImageMagick が無ければ `None`。
fn imagemagick_layer_count(path: &Path) -> Option<usize> {
    let output = Command::new("identify")
        .args(["-format", "%p\n"])
        .arg(path)
        .output()
        .ok()?;
    if !output.status.success() {
        panic!(
            "identify failed for {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let frames = String::from_utf8_lossy(&output.stdout).lines().count();
    // 1 枚だけなら合成画像だけ (レイヤー情報がない)。
    Some(frames.saturating_sub(1))
}

#[test]
fn real_world_psds_import() {
    let files = corpus();
    if files.is_empty() {
        eprintln!("no files in assets/fixtures/psd-real; nothing to test");
        return;
    }
    for path in files {
        let bytes = std::fs::read(&path).unwrap();
        let name = path.display();
        let result = import(&bytes);
        if !is_supported_format(&bytes) {
            assert!(
                matches!(result, Err(PsdError::Unsupported(_))),
                "{name}: expected unsupported, got {result:?}"
            );
            continue;
        }
        let tree = result.unwrap_or_else(|e| panic!("{name}: {e}"));
        for warning in &tree.warnings {
            eprintln!("{name}: warning: {warning}");
        }
        let flattened = tree
            .warnings
            .iter()
            .any(|w| w.kind == WarningKind::FlattenedImage);
        // 画素があるのに取り込まなかったレイヤーは、ImageMagick の数に含まれる。
        let skipped = tree
            .warnings
            .iter()
            .filter(|w| {
                matches!(
                    w.kind,
                    WarningKind::UnsupportedCompression { .. } | WarningKind::MissingPixels
                )
            })
            .count();
        if let Some(expected) = imagemagick_layer_count(&path).filter(|_| !flattened) {
            assert_eq!(
                raster_count(&tree.root) + skipped,
                expected,
                "{name}: layer count differs from ImageMagick"
            );
        }
    }
}
