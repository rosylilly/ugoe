---
id: T0553
title: "PSD: PackBits の展開の確保の上限 (入力の 64 倍) を見直す"
status: in-progress
milestone: M9
crates: [ugoe-psd]
depends_on: []
assignee: claude-worker-T0553
---

# T0553: PSD: PackBits の展開の確保の上限 (入力の 64 倍) を見直す

## 目的

T0531 で PSD のデコードに資源の上限を入れた。PackBits の展開は確保を入力の 64 倍まで許すので、実質の目安は合計の上限 (4 GiB) になる。
小さな悪意のファイルで大きな確保を起こせないか、より厳しい上限 (例: 平面ごとに幅 × 高さから決まる大きさを先に確かめ、それを超える確保をしない) にできるかを検討する。

## 受け入れ条件

- [ ] PackBits の展開で確保する大きさを、ヘッダの幅・高さから決まる大きさで抑えた (またはしない理由をメモと README に書いた)
- [ ] 小さな入力から大きな確保を起こそうとするファイルのテストがあり、上限のエラーで止まる
- [ ] `crates/ugoe-psd/README.md` の上限の説明を更新した
- [ ] `just ci` が通る

## 参考

- T0531 のメモ (起票の候補)
- crates/ugoe-psd/src/parse.rs (PackBits)・lib.rs (上限)

## メモ

- T0019 で起票した (元の候補: T0531 の PackBits)。
