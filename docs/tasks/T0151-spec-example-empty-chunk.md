---
id: T0151
title: 公開仕様のファイルの例に、ほかのチャンクの中を指す長さ 0 のチャンクを足す
status: todo
milestone: M9
crates: [spec, ugoe-format]
depends_on: [T0138]
assignee:
---

# T0151: 公開仕様のファイルの例に、ほかのチャンクの中を指す長さ 0 のチャンクを足す

## 目的

T0138 で、参照実装の読み込みが、ほかのチャンクのペイロードの中を指す長さ 0 のチャンクを `OverlappingChunks` で拒否していた誤りを直した
(`spec/bundle.md` の Chunk table は「長さ 0 のペイロードは何とも重ならない」と定める)。
`spec/examples/bundles/` にはこの場合の例がなく、ほかの実装 (バインディング・別の読み込み器) が同じ誤りをしても気づけない。

## 受け入れ条件

- [ ] `spec/examples/bundles/` に、ほかのチャンクのペイロードの中 (16 バイト境界) を指す長さ 0 の未知のチャンクを持つ、読めるファイルの例がある
- [ ] チャンク表より前 (ヘッダの中) を指す長さ 0 のチャンクを持つ、拒否されるファイルの例がある (`error: chunk_table`)
- [ ] `spec/examples/README.md` の一覧と `bundles.json` を更新し、`crates/ugoe-format/tests/spec_bundles.rs` が通る
- [ ] `just ci` が通る

## 参考

- spec/bundle.md#chunk-table、spec/examples/README.md#file-examples
- docs/tasks/T0138-review-format.md のメモ

## メモ
