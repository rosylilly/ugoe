---
id: T0568
title: "PSD のファイルの大きさの上限を ugoe-psd の 1 か所にまとめ、CLI の import-psd も上限付きで読む"
status: todo
milestone: M9
crates: [ugoe-psd, ugoe-mcp, ugoe-cli, apps/editor, docs]
depends_on: []
assignee:
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

- [ ] `ugoe-psd` が PSD のファイルの大きさの上限の定数を公開し、doc コメントに値と理由がある
- [ ] エディタ・`ugoe-mcp` が写しをやめてその定数を使う
- [ ] `ugoe import-psd` が `ugoe_fs::read_regular_file` とその上限で読む (装置・FIFO・上限を超えるファイルを拒む)。テストがある
- [ ] crates/ugoe-psd/README.md・crates/ugoe-mcp/README.md・crates/ugoe-cli/README.md を直した
- [ ] `just ci` が通る

## 参考

- T0564 のメモ (起票の候補)、T0554 のメモ (利用者の判断)
- crates/ugoe-fs/README.md

## メモ

- T0036 で起票した (元の候補: T0564)。
