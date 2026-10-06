---
id: T0953
title: "@ugoe/ui: Dialog に初めのフォーカスを選ぶ口と、Button に form の口を足す"
status: in-progress
milestone: M10
crates: [packages/ui, apps/editor, docs]
depends_on: []
assignee: claude-worker-T0953
---

# T0953: @ugoe/ui: Dialog に初めのフォーカスを選ぶ口と、Button に form の口を足す

## 目的

T0673 のエディタの見直しで、`@ugoe/ui` の部品に足りない口が見つかった (T0673 の候補):

- `Dialog` は開いたときに本文の最初の部品へフォーカスを移すだけで、どこへ移すかを選べない (外での書き換えのダイアログで、最初のチェックボックスでなく
  「後で」などに置きたい)。
- `Button` の props の型に `form` が無く、フォームの外にあるボタンからフォームを送れない。

## 受け入れ条件

- [ ] `Dialog` に初めのフォーカスを選ぶ口 (`initialFocus` など。要素か選び方か、形をメモに書く) があり、指定が無いときは今の振る舞いのまま。部品のテストがある
- [ ] `Button` が `form` 属性を受け取り、フォームの外のボタンで送れる。部品のテストがある
- [ ] エディタの外での書き換えのダイアログ (T0673 で見つけたもの) の初めのフォーカスを決めて直した
- [ ] docs/ui.md の部品の表 (口)・packages/ui/README.md を直し、見本の画面 (`just ui-shots`) と `just ui-audit --only editor` の指摘が増えない
- [ ] `just ci` が通る

## 参考

- T0673 のメモ (やらなかったこと・起票の候補)
- packages/ui/src/components (`Dialog.svelte`・`Button.svelte`)、docs/ui.md

## メモ

- T0041 で起票した (元の候補: T0673 (`Dialog` の初めのフォーカス・`Button` の `form`))。
  同じ並びの T0674 の候補 (`Checkbox` の見えるラベルを持たない形) は、エディタの残りの標準の部品を移す T0949 のメモに書き足した。
