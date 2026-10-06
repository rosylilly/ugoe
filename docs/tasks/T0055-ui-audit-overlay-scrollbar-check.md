---
id: T0055
title: "ui-audit: 重ねて描くスクロールバー (WebKitGTK) が中身に掛かるのを見つける検査を足す"
status: in-progress
milestone: M10
crates: [tests, docs]
depends_on: [T0054, T1662]
assignee: claude-worker-T0055
---

# T0055: ui-audit: 重ねて描くスクロールバー (WebKitGTK) が中身に掛かるのを見つける検査を足す

## 目的

WebKitGTK のスクロールバーは幅を取らずに中身の上に重ねて描き、指を乗せると 6〜14px に太る (T0970 で測った)。ヘッドレスの Chrome は
スクロールバーに幅を取るので、`just ui-audit` では中身が隠れる形を見つけられない。T0970 は `--ugoe-scrollbar-gap` (16px) を送る枠の
行末に空ける決めにした。その決めが守られているかを、画面の上で確かめる検査を足す。

## 受け入れ条件

- [ ] 送る枠 (`overflow(-y): auto|scroll` で今送れるもの) の中の文字・押せる部品の右端が、枠の見える右端から `--ugoe-scrollbar-gap`
      の中に入っていれば指摘する (`<部品> is under the overlay scrollbar of <枠>` のような文)。数えないもの (意図して端まで描く地・区切り線など)
      と閾値を決め、tests/ui-audit/README.md に書いた
- [ ] 単体テスト (`test/checks.test.mjs`): 行末を空けない枠の中の部品を見つけ、空けた枠・送らない枠・横に送る枠を数えない
- [ ] 3 つのアプリのすべての場面を流し、指摘を直したか、意図したもの (理由) としてメモに書いた (T0970・T1662 で直した枠は 0 件のはず)
- [ ] `just ui-audit-check` と `just ci` が通る

## 参考

- T0970 のメモ (測ったこと・起票の候補)、docs/ui.md (`--ugoe-scrollbar-gap`)、packages/ui/src/tokens.test.ts
- tests/ui-audit/lib/page-checks.js、tests/ui-audit/README.md

## メモ

- T0052 で起票した (元の候補: T0970 (重ねて描くスクロールバーを真似た検査))。
- エディタの送る枠を直す T1662 と、同じ tests/ui-audit を触る T0054 の後にした。Select・Menu の中 (T0976) が先に入っていなければ、
  それらの指摘はメモに書くだけでよい。
