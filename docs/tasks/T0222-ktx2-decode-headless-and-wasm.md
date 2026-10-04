---
id: T0222
title: ヘッドレスと wasm で非圧縮の KTX2 を読み、適合テストに KTX2 のケースを足す
status: in-progress
milestone: M9
crates: [ugoe-format, ugoe-headless, ugoe-wasm, tests/conformance, bindings/godot, docs]
depends_on: []
assignee: claude-worker-T0222
---

# T0222: ヘッドレスと wasm で非圧縮の KTX2 を読み、適合テストに KTX2 のケースを足す

## 目的

T0448 で Godot が KTX2 のテクスチャのアルファを仕様どおり (非乗算として読み、乗算する) に扱うようにした。確かめは Godot の
`bindings/godot/example/tests/ktx2.gd` だけで、参照の実装 (`ugoe render`・wasm) は KTX2 を読めないので、組み込み先の間で比べられない。

- `ugoe-headless` と wasm で、少なくとも圧縮していない KTX2 をデコードする (仕様の扱いで乗算する)。デコーダの置き場所は PNG (T0009 の `ugoe_format::image`) に揃える。
- 適合テスト (`tests/conformance`) に KTX2 のケースを足し、Godot の `ktx2.gd` の確かめをそちらへ移す。
- 書き出し側 (エディタ・`ugoe-format`) は今 KTX2 を書かないことを確かめる。書くなら `KHR_DF_FLAG_ALPHA_PREMULTIPLIED` を立てず非乗算で書くことを検査する。

## 受け入れ条件

- [ ] 非圧縮の KTX2 (RGBA8・sRGB) のデコードがあり、上限 (大きさ) を PNG と同じく確かめる。不正なファイルのテストがある
- [ ] `ugoe render` と wasm が KTX2 のテクスチャを持つ `.ugoe` を描ける
- [ ] 適合テストに KTX2 のケースがあり、C ABI・wasm・Godot で参照と比べる。Godot の `ktx2.gd` を外すか、残す理由を書いた
- [ ] 書き出し側の扱いを確かめてメモに書いた
- [ ] `docs/spec/rendering.md` (または file-formats.md) と公開仕様の対応する節を、必要なら更新した
- [ ] `just ci`・`just wasm-check`・`just conformance`・`just godot-check` が通る

## 参考

- T0448 のメモ (起票の候補)
- crates/ugoe-format/src/image.rs、tests/conformance/README.md

## メモ

- T0019 で起票した (元の候補: T0448 の 2 つ)。
