---
id: T0564
title: "ugoe-mcp: 自動リギングの PSD を ugoe_fs::read_regular_file で上限付きで読む"
status: done
milestone: M9
crates: [ugoe-mcp]
depends_on: []
assignee: claude-worker-T0564
---

# T0564: ugoe-mcp: 自動リギングの PSD を ugoe_fs::read_regular_file で上限付きで読む

## 目的

`ugoe-mcp` の `PsdSource::load_with` (`crates/ugoe-mcp/src/autorig.rs`) は、`metadata` を見てから `std::fs::read` で読む。
開いた後に通常のファイルかを見直さず、大きさの上限も無い (T0025 の候補)。T0025 でまとめた `ugoe_fs::read_regular_file` に、PSD の上限 (エディタと同じ 2 GiB) を付けて読む。

## 受け入れ条件

- [x] `PsdSource::load_with` が `ugoe_fs::read_regular_file` で読み、装置・FIFO・ディレクトリを開かずに拒み、上限を超える PSD を読まない
- [x] 上限の値はエディタの PSD の読み込みの上限と同じものを使う (同じ定数を参照するか、違えるなら理由をメモに)
- [x] 拒む場合のテストがある
- [x] `just ci` が通る

## 参考

- T0025 のメモ (起票の候補)
- crates/ugoe-fs/README.md

## メモ

- T0032 で起票した (元の候補: T0025 (ugoe-mcp の PSD))。
- やったこと: `PsdSource::load_with` を `ugoe_fs::read_regular_file(path, max_bytes)` で読むようにした (開く前と開いた後に
  通常のファイルかを確かめ、上限 + 1 バイトまでに限って読む)。`load_with` は大きさの上限も引数で受け、`load` は
  `MAX_PSD_BYTES` (2 GiB) と `ImportLimits::default()` を渡す。展開の上限は従来どおり SDK (`ugoe_psd::ImportLimits`) の既定値で、
  設定には出さない (T0554 の判断)。README の「PSD は通常のファイルだけを読む」の項に上限を書き足した。
- テスト: 既存の `/dev/null`・ディレクトリに加えて FIFO (`mkfifo` が使えるときだけ) を拒むこと、ファイルの大きさが上限を
  1 バイト超えると `file is too large` で拒み、ちょうどなら読めることを `refuses_to_read_a_psd_larger_than_the_limit` で確かめる。
- 上限の定数: エディタの `MAX_PSD_BYTES` (`apps/editor/src-tauri/src/psd.rs`) は GPL のアプリの `pub(crate)` で、MIT の
  `ugoe-mcp` から参照できない。値 (2 GiB) を写し、doc コメントに「エディタと同じ値。変えるときは両方を直す」と書いた。
- 起票の候補: PSD のファイルの大きさの上限を 1 か所にまとめる (例: `ugoe_psd::MAX_PSD_FILE_BYTES` を公開し、エディタ・
  `ugoe-mcp`・CLI の `import-psd` が参照する)。`ugoe-psd` の公開 API を足すので、このタスクの範囲の外にした。
