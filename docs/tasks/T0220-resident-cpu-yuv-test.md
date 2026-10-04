---
id: T0220
title: 常駐の描画: 計算シェーダの無いアダプタでの YUV の読み戻しをテストする
status: todo
milestone: M9
crates: [ugoe-headless, docs]
depends_on: [T0212]
assignee:
---

# T0220: 常駐の描画: 計算シェーダの無いアダプタでの YUV の読み戻しをテストする

## 目的

T0212 (ugoe-headless のコードレビュー) の所見。`Resident` は計算シェーダの無いアダプタでは RGBA を読み戻して
CPU で YUV に変換する (`converter` が `None` の道)。この道は `gpu.supports_compute()` が偽のときにしか通らず、
計算シェーダを持つアダプタ (lavapipe を含む) で動く `tests/resident_gpu.rs` では一度も通らない。

テストからこの道を選べるようにし (例: `ResidentOptions` に GPU の変換を使わない指定を足す。公開 API の追加になる)、
GPU の変換と同じ結果 (±1) になることを確かめる。

## 受け入れ条件

- [ ] CPU で変換する道をテストから選べる (公開 API に足すなら doc コメントと README に書く)
- [ ] `tests/resident_gpu.rs` で、NV12・YUY2 の CPU の道が GPU の道と ±1 で一致することを確かめる
- [ ] `just ci` が通る

## 参考

- crates/ugoe-headless/src/resident.rs (`Resident::new`・`read`)
- crates/ugoe-headless/README.md (常駐の描画)

## メモ
