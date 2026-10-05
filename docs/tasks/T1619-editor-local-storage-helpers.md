---
id: T1619
title: "エディタ: localStorage の読み書きを @ugoe/i18n の readStoredItem などに寄せる"
status: in-progress
milestone: M9
crates: [apps/editor]
depends_on: [T1610]
assignee: claude-worker-T1619
---

# T1619: エディタ: localStorage の読み書きを @ugoe/i18n の readStoredItem などに寄せる

## 目的

T0937 でビューアとスタジオの localStorage の読み書きを `@ugoe/i18n` の `readStoredItem` などに寄せた。エディタの `TimelinePanel.svelte`・
`lib/layout.ts`・`lib/externalChange.ts` は、まだ自前の try で読み書きしている (T0937 の候補。try の写しを減らすだけの小さな整理)。
タイムラインを触る T1610 の後に着手する。

## 受け入れ条件

- [ ] エディタの localStorage の読み書きが `readStoredItem` など共有の関数を通り、自前の try の写しが無い (`grep localStorage apps/editor/src` で、テストの外に直接の呼び出しが無い)
- [ ] 保存した値の読み書きの振る舞い (壊れた値・使えないストレージで既定に戻る) は今のままで、テストが通る
- [ ] `just ci` が通る

## 参考

- T0937 のメモ (起票の候補)
- packages/i18n/README.md

## メモ

- T0032 で起票した (元の候補: T0937)。
