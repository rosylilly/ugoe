---
id: T0220
title: 常駐の描画: 計算シェーダの無いアダプタでの YUV の読み戻しをテストする
status: done
milestone: M9
crates: [ugoe-headless, docs]
depends_on: [T0212]
assignee: claude-worker-T0220
---

# T0220: 常駐の描画: 計算シェーダの無いアダプタでの YUV の読み戻しをテストする

## 目的

T0212 (ugoe-headless のコードレビュー) の所見。`Resident` は計算シェーダの無いアダプタでは RGBA を読み戻して
CPU で YUV に変換する (`converter` が `None` の道)。この道は `gpu.supports_compute()` が偽のときにしか通らず、
計算シェーダを持つアダプタ (lavapipe を含む) で動く `tests/resident_gpu.rs` では一度も通らない。

テストからこの道を選べるようにし (例: `ResidentOptions` に GPU の変換を使わない指定を足す。公開 API の追加になる)、
GPU の変換と同じ結果 (±1) になることを確かめる。

## 受け入れ条件

- [x] CPU で変換する道をテストから選べる (公開 API に足すなら doc コメントと README に書く)
- [x] `tests/resident_gpu.rs` で、NV12・YUY2 の CPU の道が GPU の道と ±1 で一致することを確かめる
- [x] `just ci` が通る

## 参考

- crates/ugoe-headless/src/resident.rs (`Resident::new`・`read`)
- crates/ugoe-headless/README.md (常駐の描画)

## メモ

- やったこと:
  - `ResidentOptions` に `cpu_yuv: bool` を足した (公開 API の追加)。`true` なら計算シェーダがあっても RGBA を読み戻して `yuv::convert` で変換する。
    doc コメントと crates/ugoe-headless/README.md (常駐の描画・テスト) に書いた。
  - `tests/resident_gpu.rs` に `yuv_on_the_cpu_matches_the_gpu_path` を足した: NV12・YUY2 で、CPU の道 (`converts_on_gpu()` が偽) の結果が
    1 枚の口の RGBA を参照実装で変換したものと完全に一致し、GPU の道と ±1 で一致する。アダプタに計算シェーダが無ければ GPU との比較だけ飛ばす。
    ±1 の比較は既存のテストと共通の `assert_close` にまとめた。
  - lavapipe (Vulkan)・llvmpipe (GL) の両方で `cargo test -p ugoe-headless --features gpu-tests` が通ることを確かめた。
- 範囲の外の変更 (小さなもの): 構造体のフィールドが増えたので、apps/studio/src-tauri/src/virtual_camera.rs の `ResidentOptions` に `cpu_yuv: false` を足した。
- `just ci` は通った。
- 起票の候補: なし。
