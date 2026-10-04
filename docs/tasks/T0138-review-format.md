---
id: T0138
title: コードレビュー: ugoe-format
status: done
milestone: M9
crates: [ugoe-format, docs]
depends_on: []
assignee: claude-worker-T0138
---

# T0138: コードレビュー: ugoe-format

## 目的

ugoe-format のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `crates/ugoe-format` のすべて

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - 壊れた・巨大な `.ugoe`・`.ugoemod` (展開の爆発・長さの偽装・パスのたどり) で止まらないか、確保の上限
  - `docs/spec/file-formats.md` と読み書きの一致、未知の値の保持 (前方互換)、版の読み分け
  - 書いたものを読み直すと同じになるか (往復のテスト)
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0150〜T0154 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)

### 所見

`cargo clippy -p ugoe-format -- -W clippy::pedantic` も流した (`#[must_use]`・`# Errors` などの体裁だけで、直すべきものはなかった)。

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 高 | `src/bundle.rs` (`Entry::payload`・`read_with_known`) | 展開後の上限は 1 チャンクあたり 256 MiB だけで、ファイル全体の合計に上限がない。数 KiB で 256 MiB に展開される `MOTN` や zstd のフラグ付きの `TEXR` を並べると、1 MiB 未満のファイルで数十 GiB を確保する (テクスチャの数の検査はすべて読んだ後) | T0150 を起票 (公開仕様は 1 チャンクの上限だけを定め、合計の上限は仕様の判断が要る) |
| 中 | `src/features.rs` (`check`) | `FEAT` の宣言 1 つごとに宣言の全体を線形に探していて、宣言の数の 2 乗の時間がかかる。知らない宣言 20 万個で 2 分以上止まる (外から来るデータで止まる) | 直した (集合で調べる。20 万個の宣言のテストを先に足し、修正前は 2 分で打ち切り、修正後は 0.3 秒) |
| 中 | `src/bundle.rs` (`read_table`) | 長さ 0 のチャンクがほかのチャンクのペイロードの中を指すと `OverlappingChunks` で拒否していた。仕様 (`spec/bundle.md` の Chunk table) は「長さ 0 のペイロードは何とも重ならない」 | 直した (重なりの検査から長さ 0 を除く。失敗するテストを先に足した)。公開仕様の例は T0151 を起票 |
| 中 | `src/lib.rs` (`model_from_json`・`module_from_json`) | 全体をデコードしてから `schema_version` を調べるので、形の変わった新しい版の JSON がデコードのエラーになる (バンドルは先に版を読む) | 直した (`schema_version` だけを先に読む。テストを先に足し、`docs/spec/file-formats.md#バージョニング` に追記) |
| 中 | `src/bundle.rs` (`ChunkWriter::push_encoded`) | 圧縮の設定のとき、256 MiB を超えるペイロードも圧縮して、自分の読み込みが `DecompressedTooLarge` で拒否するファイルを作る (仕様は MUST NOT) | 直した (上限を超えるペイロードは圧縮しない。上限を試験で小さくしてテスト。仕様書に追記) |
| 中 | `src/bundle.rs` | `DecompressedTooLarge` の経路にテストがなかった | 直した (`Entry::payload_within` で上限を小さくしてテスト) |
| 低 | `src/lib.rs` (`physics_to_json`) | 未知の値を含む物理を、`model_to_json` と違って `UnknownValues` (場所の一覧) でなく serde の分かりにくいエラーで拒否する | 直した (テストを先に足した) |
| 低 | `tests/compat.rs` | 書いたものを読み直して書くと同じになるかを、フィクスチャの全体で確かめていなかった (`roundtrip_*` は一部の組み合わせだけ) | 直した (`dev/` の資料のすべてで、読んで書き直すと同じバイト列になることを検査) |
| 低 | `src/bundle.rs` のテスト | バンドルの `META` の重複のテストがなかった (モジュールにはある) | 直した |
| 低 | `src/lib.rs`・`README.md`・`docs/spec/file-formats.md` | `FormatError` に doc コメントがない。`model_from_json` が検証しないことが書かれていない。README が未知の値の扱いを `features` モジュールのものと書いている。長さ 0 のチャンクがチャンク表より後ろから始まる必要があることが日本語の仕様書にない | 直した (文書だけ) |
| 低 | `src/bundle.rs` (`read_with_known`) | `MOTN` `EXPR` は読み込みで検証しない (`ugoe-model` に `validate_motion` がない) | 見送り (`ugoe-motion` の評価は壊れたカーブで `None` を返し、パニックしないことを確かめた。仕様も検証を定めていない) |
| 低 | `src/bundle.rs` / `src/module_file.rs` | `read_texture` と `read_normal_map`、バンドルとモジュールの `read_with_known` がよく似ている | 見送り (エラーの種類とチャンクの種類が違い、まとめると読みにくくなる) |
| 低 | `tests/spec_examples.rs` | ランタイムの評価の例の検査が ugoe-format にある | 見送り (`model_from_json` を使うためで、置き場所の好みの範囲) |
| 低 | `src/bundle.rs` (`normal_map_of`) | テクスチャの番号から法線マップを引くたびに前のテクスチャを数える (O(n)) | 見送り (読み込み時と描画の準備で数回呼ぶだけで、テクスチャの数は小さい) |

確かめたこと (所見なし): 切り詰め・範囲外・オーバーフロー・境界・重なり・予約フラグ・`chunk_count` の偽装 (表が入力に収まることを先に確かめるので確保は入力長で抑えられる)・MessagePack の入れ子の深さ・JSON の入れ子 (serde_json の既定の 128)・
未知のタグのチャンクを展開しないこと・`FEAT` の検査がデコードより先なこと・書き出しのチャンクの順と 16 バイト境界・未知の値の書き出しの拒否・`.ugoe` と `.ugoemod` の取り違え・ライブラリのコードの `unwrap`/添字 (すべて範囲を確かめた後)・`unsafe` なし・ファイルシステムに触れないこと (パスのたどりは、コンテナにパスがなく名前は ID なので起きない。プロジェクト形式は ugoe-editor-core の範囲)。
`just ci` と `just wasm-check` は通った。
`.ugoemod` はコンテナを共有するので、上の高・中の所見はモジュールにも同じく当てはまる (直したものはどちらにも効く)。

### 横断

- **外から来る宣言や一覧の重複検査の 2 乗:** `Vec::contains` や `iter().any` を要素ごとに呼ぶ重複の検査は、外から来るデータ (ファイル・IPC・FFI) では止まる原因になる。ugoe-model の検証 (`validate`) や ugoe-editor-core のプロジェクトの読み込みにも同じ形がないか見るとよい。
- **展開・デコードの合計の上限:** 1 つの要素ごとの上限 (チャンク・画像の幅と高さ) があっても、数の上限がないと合計で爆発する。PNG / KTX2 のデコード (ugoe-render・ビューア・スタジオ・`@ugoe/runtime`) の大きさの上限と、`TextureRef` の `width`/`height` と実際の画像の大きさの食い違いの扱いも見るとよい。
- **長さ 0 の扱い:** 範囲の重なりや空の入力の扱いは、仕様の文言 (「何とも重ならない」) とずれやすい。ほかの読み込み器 (バインディング・spec-check の道具) も、公開仕様の例 (T0151) で確かめる。
- **試験のために上限を差し替える口:** 巨大な入力でしか通らない経路 (上限・オーバーフロー) は、上限を引数で受ける内部の関数を作るとテストできる (`Entry::payload_within`・`ChunkWriter::compress_limit`)。
