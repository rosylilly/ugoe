---
id: T0151
title: 公開仕様のファイルの例に、ほかのチャンクの中を指す長さ 0 のチャンクを足す
status: done
milestone: M9
crates: [spec, ugoe-format]
depends_on: [T0138]
assignee: claude-worker-T0151
---

# T0151: 公開仕様のファイルの例に、ほかのチャンクの中を指す長さ 0 のチャンクを足す

## 目的

T0138 で、参照実装の読み込みが、ほかのチャンクのペイロードの中を指す長さ 0 のチャンクを `OverlappingChunks` で拒否していた誤りを直した
(`spec/bundle.md` の Chunk table は「長さ 0 のペイロードは何とも重ならない」と定める)。
`spec/examples/bundles/` にはこの場合の例がなく、ほかの実装 (バインディング・別の読み込み器) が同じ誤りをしても気づけない。

## 受け入れ条件

- [x] `spec/examples/bundles/` に、ほかのチャンクのペイロードの中 (16 バイト境界) を指す長さ 0 の未知のチャンクを持つ、読めるファイルの例がある
- [x] チャンク表より前 (ヘッダの中) を指す長さ 0 のチャンクを持つ、拒否されるファイルの例がある (`error: chunk_table`)
- [x] `spec/examples/README.md` の一覧と `bundles.json` を更新し、`crates/ugoe-format/tests/spec_bundles.rs` が通る
- [x] `just ci` が通る

## 参考

- spec/bundle.md#chunk-table、spec/examples/README.md#file-examples
- docs/tasks/T0138-review-format.md のメモ

## メモ

- やったこと:
  - `spec/examples/bundles/` に `empty-chunk-inside.ugoe` (長さ 0 の `Xnul` が `MODL` のペイロードの中の offset 112 を指す。`MODL` は 96 から 671 バイト。読める) と
    `empty-chunk-header.ugoe` (長さ 0 の `Xnul` が offset 0、ヘッダの中を指す。`error: chunk_table`) を足した。`bundles.json` と `spec/examples/README.md` の一覧を更新した。
  - 目録の `chunks[]` に任意の `offset` (長さ 0 のペイロードだけ。格納された offset) を足し、README の表に書いた。「この版の例は `feature_declarations` だけを使う」を `chunk_table` を含めて直した。
  - `spec/bundle.md` の Chunk table の箇条に、長さ 0 のペイロードがほかのペイロードの中を指してよいこと、チャンク表の終わりより前を指してはならないこと
    (Reading の手順 2 にはすでにある) を明記した。日本語の `docs/spec/file-formats.md` はすでに同じことを書いているので変えていない。
  - `crates/ugoe-format/tests/spec_bundles.rs` で `offset` を読み、格納された (offset, length) と照合する。参照実装は 2 つの例とも期待どおり (読む / `chunk_table` で拒否)。
- `crates:` の外の変更 (小さなもの): `tools/spec-check/` (例を作る独立の書き出し) で、長さ 0 の知らないチャンクに明示の offset を書けるようにし
  (`write.ts`・`bundles.ts`、`writer.test.ts` にテスト)、目録の照合で offset を比べ、拒否されるファイルの表も読めるよう検査なしで表を読む `storedTable` を足した。
  独立性の規則を守るため、この部分は `crates/` と `docs/spec/` を読まない別のサブエージェントが spec/ だけから行った。spec-check の読み込み器は直す必要がなかった (2 つの例を仕様どおりに読む / 拒む)。
  既存の例のファイルは作り直しても変わらなかった。
- やらなかったこと: `tools/spec-check/src/roundtrip.ts` (参照実装の `ugoe` を使う手動の往復の試験) は流していない (読めるほうの例は `ugoe-format` のテストで参照実装が読むことを確かめた)。
- `just ci` は通った。
