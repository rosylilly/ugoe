---
id: T0150
title: .ugoe / .ugoemod の読み込みで展開の合計に上限を設ける
status: todo
milestone: M9
crates: [ugoe-format, docs, spec]
depends_on: [T0138]
assignee:
---

# T0150: .ugoe / .ugoemod の読み込みで展開の合計に上限を設ける

## 目的

T0138 (ugoe-format のレビュー) で見つけた、展開の爆発でメモリを使い切る問題を直す。

圧縮したチャンクの展開後の上限は 1 チャンクあたり 256 MiB (`MAX_DECOMPRESSED_CHUNK_LEN`) だが、ファイル全体の合計には上限がない。
zstd は同じバイトの並びを数バイトのブロックで表せるので、256 MiB に展開されるチャンクは数 KiB で作れる。
`MOTN` (名前が 256 MiB の文字列のモーション) や、zstd のフラグを立てた `TEXR` / `NRML` (読み込みは画像のチャンクも展開する) を
数百個並べると、1 MiB に満たないファイルで、読み込みの途中で数十 GiB を確保して止まる
(テクスチャの数の検査は、すべての `TEXR` を読んだ後に行う)。外から来るファイルでクラッシュするので重大度は高。

公開仕様 (`spec/bundle.md` の Compression) は 1 チャンクあたりの上限だけを定め、読み込み器は少なくとも 256 MiB に対応しなければならない (MUST) としている。
ファイル全体の上限を入れると、仕様に適合するファイルを拒否しうるので、仕様の判断が要る。

## 受け入れ条件

- [ ] 次のどれか (または組み合わせ) を選び、`spec/bundle.md` と `docs/spec/file-formats.md` に書いた
  - ファイル全体の展開後の合計に上限を設け、仕様で読み込み器に許す (MAY。下限も決める)
  - 画像 (`TEXR` `NRML`) の圧縮を禁止する (MUST NOT) か、展開しない
  - `TEXR` / `NRML` の数を、ペイロードを読む前に (`MODL` / `MODU` をデコードした直後に) 検査する
- [ ] 小さなファイルが合計の上限を超えて展開されることを拒否するテストがある (上限は試験で小さくして、大きな確保をしない)
- [ ] `just ci` が通る

## 参考

- docs/spec/file-formats.md#ペイロードのエンコード、spec/bundle.md#compression
- `crates/ugoe-format/src/bundle.rs` の `Entry::payload` と `read_with_known`
- docs/tasks/T0138-review-format.md のメモ

## メモ
