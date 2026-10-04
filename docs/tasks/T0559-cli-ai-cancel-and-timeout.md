---
id: T0559
title: "ugoe-cli: AI のコマンドに Ctrl+C での取り消しと時間の上限のオプションを付ける"
status: in-progress
milestone: M9
crates: [ugoe-cli, docs]
depends_on: []
assignee: claude-worker-T0559
---

# T0559: ugoe-cli: AI のコマンドに Ctrl+C での取り消しと時間の上限のオプションを付ける

## 目的

T0541 で `ugoe-ai` の要求を取り消せるようにしたが、CLI の AI のコマンドは取り消しも時間の上限も無い (T0541 の候補)。
Ctrl+C で今の要求を取り消し、`--timeout <SECONDS>` で上限を付けられるようにする。

## 受け入れ条件

- [ ] AI のコマンドに `--timeout` があり、上限を超えたら取り消して分かるエラーで終わる。引数のエラー (負・NaN) のテストがある
- [ ] Ctrl+C で取り消しの印を立て、書きかけのファイルを残さずに終わる (印を渡す部分のテストがある。シグナルそのものは手で確かめた範囲をメモに)
- [ ] crates/ugoe-cli/README.md を更新した
- [ ] `just ci` が通る

## 参考

- T0541 のメモ (起票の候補)

## メモ

- T0026 で起票した (元の候補: T0541 の CLI)。
- T0541 のもう 1 つの候補「`send_cancellable` (`crates/ugoe-ai/src/transport.rs`) が応答を待つたびにスレッドを作る。ureq に中断の手段ができたら通信そのものを止める形に替える」は、
  ureq の対応を待つので起票しない。ureq を上げるときに確かめる。
