---
id: T0162
title: PNG のデコードの残りの写しを ugoe_format::image に寄せる (editor-core とテスト)
status: in-progress
milestone: M9
crates: [ugoe-editor-core, ugoe-format, docs]
depends_on: []
assignee: claude-worker-T0162
---

# T0162: PNG のデコードの残りの写しを ugoe_format::image に寄せる (editor-core とテスト)

## 目的

T0009 で PNG のデコードと大きさの上限を `ugoe_format::image` にまとめた (`ugoe-headless`・`ugoe-wasm`) が、写しが 2 つ残っている。

- **`ugoe-editor-core/src/texture.rs` (優先):** 大きさの上限の無い写し。エディタは外から来た PNG (取り込み・差し替え) を読むので、巨大な画像でメモリを使い切りうる。
- `ugoe-format/tests/spec_bundles.rs` のテスト用のデコーダ (dev-dependency で自分の `png` feature を有効にすれば置き換えられる)。

## 受け入れ条件

- [ ] `ugoe-editor-core` の PNG のデコードが `ugoe_format::image` を使い、上限を超える画像をエラー (コードつき。ADR 0029) で拒む。テストがある
- [ ] `spec_bundles.rs` のテスト用のデコーダを `ugoe_format::image` に置き換えた
- [ ] `crates/ugoe-editor-core/README.md` (変わるなら) を更新した
- [ ] `just ci` と `just wasm-check` が通る

## 参考

- T0009 のメモ (起票の候補)
- crates/ugoe-format/src/image.rs

## メモ

- T0019 で起票した (元の候補: T0009 の 2 つ)。
