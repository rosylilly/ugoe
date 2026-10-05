---
id: T0035
title: ugoe-headless の read_file を ugoe-fs に寄せるか決める (依存方向の判断が要る)
status: done
milestone: M9
crates: [ugoe-headless, scripts, docs]
depends_on: []
assignee: claude-worker-T0035
---

# T0035: ugoe-headless の read_file を ugoe-fs に寄せるか決める (依存方向の判断が要る)

## 目的

T0025 で、ファイルの読み込みを `ugoe-fs` の `read_regular_file` (通常のファイルだけを上限付きで読む) にまとめた。
`ugoe-headless` の `read_file` (`crates/ugoe-headless/src/lib.rs`) も `metadata` を見てから読む同じ形の読み込みだが、
今の依存方向 (`scripts/check-layers.mjs` の `"ugoe-headless": RUNTIME_TIER`) では、ツール層の `ugoe-fs` に依存できない (T0025 の候補)。

**判断が要る:** (a) `ugoe-headless` が `ugoe-fs` に依存してよいことにする (`ugoe-fs` は std だけに依存する葉なので、ランタイム層の制約には触れない。
`tests/conformance` も `ugoe-headless` を使う)、(b) `ugoe-headless` の中で同じ検査 (開いた後に通常のファイルかと大きさを見直す) を写す、(c) 今のまま。
推奨を書き、利用者の判断を待ってから実装する。

## 受け入れ条件

- [x] 案と推奨をメモに書き、利用者の判断を得た
- [x] (a) か (b) なら: `read_file` が装置・FIFO・ディレクトリを開かずに拒み、上限を超えるファイルを読まない。そのテストがある
- [x] (a) なら `scripts/check-layers.mjs` と docs/architecture.md・crates/ugoe-fs/README.md の依存の説明を更新した
- [x] `just ci` が通る

## 参考

- T0025 のメモ (起票の候補)
- crates/ugoe-fs/README.md、scripts/check-layers.mjs

## メモ

- T0032 で起票した (元の候補: T0025 (ugoe-headless の read_file))。
- 判断 (2026-10-05、利用者の判断): (a) `ugoe-headless` が `ugoe-fs` に依存してよいことにする (check-layers で許し、理由を docs/architecture.md に書く)。
- やったこと (2026-10-05):
  - `ugoe-headless` が `ugoe-fs` に依存する (`Cargo.toml`)。`read_file(path, max)` は `ugoe_fs::read_regular_file` を呼び、
    `ReadError` を `std::io::Error` に変えて今までどおり `Error::Io { context: "reading <path>" }` で返す。
  - 上限: テクスチャ (`--texture`) は `MAX_TEXTURE_FILE_BYTES` = 1 GiB + 1 MiB (一辺 16384 の圧縮していない RGBA の KTX2 + ヘッダの余裕)、
    JSON (`--motion`・`--expression`・`--physics`) は `MAX_JSON_FILE_BYTES` = `ugoe_format::bundle::MAX_DECOMPRESSED_CHUNK_LEN` (256 MiB。バンドルの中のモーションと同じ上限)。
  - テスト `read_file_refuses_non_regular_files_and_files_over_the_limit` (`/dev/zero`・ディレクトリ・FIFO を拒み、上限を超えるファイルは `FileTooLarge`)。既存の `refuses_to_read_files_that_are_not_regular` もそのまま通る。
  - `scripts/check-layers.mjs` で `ugoe-headless` に `FS` を許し、理由を docs/architecture.md (層の表と「層を分ける理由」) に書いた。
    crates/ugoe-headless/README.md・crates/ugoe-fs/README.md と `ugoe-fs` の doc コメント (`lib.rs`・`read.rs`) の利用者の一覧を更新した
    (`ugoe-fs` は `crates:` の外だが、受け入れ条件が README の更新を求めているのと、doc コメントの 2 行だけなので行った)。
  - `just ci`・`just wasm-check`・`just layers`・`just conformance` が通った。
- やらなかったこと: `ugoe-cli`・`ugoe-mcp` がモデルのファイル本体を読む箇所は範囲の外なので触っていない。
