---
id: T0564
title: "ugoe-mcp: 自動リギングの PSD を ugoe_fs::read_regular_file で上限付きで読む"
status: in-progress
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

- [ ] `PsdSource::load_with` が `ugoe_fs::read_regular_file` で読み、装置・FIFO・ディレクトリを開かずに拒み、上限を超える PSD を読まない
- [ ] 上限の値はエディタの PSD の読み込みの上限と同じものを使う (同じ定数を参照するか、違えるなら理由をメモに)
- [ ] 拒む場合のテストがある
- [ ] `just ci` が通る

## 参考

- T0025 のメモ (起票の候補)
- crates/ugoe-fs/README.md

## メモ

- T0032 で起票した (元の候補: T0025 (ugoe-mcp の PSD))。
