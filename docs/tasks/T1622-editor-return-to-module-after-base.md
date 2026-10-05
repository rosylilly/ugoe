---
id: T1622
title: "エディタ: 「Open base project」で本体を直した後、モジュールに戻る道を作る"
status: in-progress
milestone: M9
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1622
---

# T1622: エディタ: 「Open base project」で本体を直した後、モジュールに戻る道を作る

## 目的

T1609 で、モジュールの適用のエラーから「Open base project」で本体を開けるようにした。本体を直し終えた後、元のモジュールに戻る道
(最近開いたモジュールを開き直すボタンなど) が無い。本体の問題の枠の同じボタンも同じ (T1609 の候補)。

## 受け入れ条件

- [ ] 本体を「Open base project」で開いたとき、元のモジュールを開き直す手段 (ボタンなど) が見える所にあり、押すとモジュールを開き直して適用をやり直す
- [ ] 文言は `@ugoe/i18n` のカタログ (日英) にあり、部品のテストがある
- [ ] docs/editor.md (とマニュアルの更新の候補を T0919 のメモ) に書いた
- [ ] `just ci` が通る

## 参考

- T1609 のメモ (起票の候補)
- docs/editor.md

## メモ

- T0032 で起票した (元の候補: T1609 (モジュールに戻る道))。
