---
id: T0568
title: "PSD のファイルの大きさの上限を ugoe-psd の 1 か所にまとめ、CLI の import-psd も上限付きで読む"
status: done
milestone: M9
crates: [ugoe-psd, ugoe-mcp, ugoe-cli, apps/editor, docs]
depends_on: []
assignee: claude-worker-T0568
---

# T0568: PSD のファイルの大きさの上限を ugoe-psd の 1 か所にまとめ、CLI の import-psd も上限付きで読む

## 目的

PSD のファイルの大きさの上限 (2 GiB) が、エディタ (`apps/editor/src-tauri/src/psd.rs` の `MAX_PSD_BYTES`、`pub(crate)`) と
`ugoe-mcp` (`crates/ugoe-mcp/src/autorig.rs` の写し) の 2 か所にある。GPL のアプリの定数は MIT のクレートから参照できないので、
T0564 では値を写した (T0564 の候補)。また CLI の `ugoe import-psd` (`crates/ugoe-cli/src/main.rs` の `run_import_psd`) は
`std::fs::read` で上限なしに読む。

`ugoe_psd::MAX_PSD_FILE_BYTES` (名前は実装で決めてよい) を公開し、エディタ・`ugoe-mcp`・CLI がそれを参照する。
上限を変える設定は出さない (T0554 の利用者の判断のとおり。値を 1 か所にするだけ)。

## 受け入れ条件

- [x] `ugoe-psd` が PSD のファイルの大きさの上限の定数を公開し、doc コメントに値と理由がある
- [x] エディタ・`ugoe-mcp` が写しをやめてその定数を使う
- [x] `ugoe import-psd` が `ugoe_fs::read_regular_file` とその上限で読む (装置・FIFO・上限を超えるファイルを拒む)。テストがある
- [x] crates/ugoe-psd/README.md・crates/ugoe-mcp/README.md・crates/ugoe-cli/README.md を直した
- [x] `just ci` が通る

## 参考

- T0564 のメモ (起票の候補)、T0554 のメモ (利用者の判断)
- crates/ugoe-fs/README.md

## メモ

- T0036 で起票した (元の候補: T0564)。
- やったこと: `ugoe_psd::MAX_PSD_FILE_BYTES` (2 GiB) を公開し、doc コメントに値と理由 (PSD の形式は長さを 32 ビットで持つ・
  このクレートはファイルを読まないので読む側が使う・設定には出さない (T0554)) を書いた。エディタ (`psd.rs` の `MAX_PSD_BYTES`
  を消し、`psd.rs`・`proposals.rs` が参照) と `ugoe-mcp` (`autorig.rs` の写しを消した) がこの定数を使う。
- `ugoe import-psd` は `ugoe_fs::read_regular_file(&input, MAX_PSD_FILE_BYTES)` で読む (ugoe-cli に ugoe-fs・ugoe-psd の依存を
  足した。どちらも `check-layers.mjs` の TOOL_BASE に入っている)。同じ CLI の `ugoe sample avatar --psd` も揃えた (受け入れ条件の外だが
  同じ PSD の読み込みなので)。テスト `import_psd_refuses_non_regular_files_and_files_over_the_limit` (Unix だけ) が `/dev/zero`・
  ディレクトリ・FIFO と、上限より 1 バイト大きい疎なファイルを拒むこと、出力のプロジェクトを作らないことを確かめる。
- 文書: crates/ugoe-psd/README.md に「ファイルの大きさの上限」の節、ugoe-mcp・ugoe-cli の README、docs/editor.md の
  「読み込みの上限」と IPC のパスの読み方の項 (CLI だけが FIFO を読めるという記述を、`import-psd` は拒み、モデル・バンドルの入力は
  読めるままと直した)。
- やらなかったこと: CLI のモデル・バンドル・`build` の入力は `std::fs::read` のまま (T0530 で FIFO を読めるままにすると決めた範囲)。
  上限ちょうどのファイルを読めることは CLI では確かめていない (2 GiB を読むテストは重いので。境界は ugoe-fs・ugoe-mcp のテストが見る)。
- 起票の候補: なし。
