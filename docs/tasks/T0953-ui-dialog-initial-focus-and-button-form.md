---
id: T0953
title: "@ugoe/ui: Dialog に初めのフォーカスを選ぶ口と、Button に form の口を足す"
status: done
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

- [x] `Dialog` に初めのフォーカスを選ぶ口 (`initialFocus` など。要素か選び方か、形をメモに書く) があり、指定が無いときは今の振る舞いのまま。部品のテストがある
- [x] `Button` が `form` 属性を受け取り、フォームの外のボタンで送れる。部品のテストがある
- [x] エディタの外での書き換えのダイアログ (T0673 で見つけたもの) の初めのフォーカスを決めて直した
- [x] docs/ui.md の部品の表 (口)・packages/ui/README.md を直し、見本の画面 (`just ui-shots`) と `just ui-audit --only editor` の指摘が増えない
- [x] `just ci` が通る

## 参考

- T0673 のメモ (やらなかったこと・起票の候補)
- packages/ui/src/components (`Dialog.svelte`・`Button.svelte`)、docs/ui.md

## メモ

- T0041 で起票した (元の候補: T0673 (`Dialog` の初めのフォーカス・`Button` の `form`))。
  同じ並びの T0674 の候補 (`Checkbox` の見えるラベルを持たない形) は、エディタの残りの標準の部品を移す T0949 のメモに書き足した。

- **やったこと:**
  - `Dialog` の `initialFocus?: string`: 開いたときにフォーカスを移す要素の **id** (選び方や要素の参照でなく id にした。`Button`・`TipButton`・
    `IconButton` は `id` を受けるので呼び手は `$props.id()` で作った id を両方に渡すだけで済み、`AlertDialog` の取り消しと同じ形)。
    Bits UI の `onOpenAutoFocus` で移し、省くか見つからないときは今のまま (中の最初の操作できる要素)。
  - 選んだ要素の `Tooltip` は開いたときには出さない (`context.ts` の `focusQuietly`・`isQuietlyFocused`。Tooltip がトリガーの `onfocus` を包む)。
    出すと最初の Esc が説明を消すだけになり、「Esc = 後で」が 2 回押しになった (happy-dom で確かめて直した)。ポインタと、次からのフォーカスでは出る。
  - `Button` (と `IconButton`。`ButtonBaseProps`) の `form?: string`。前から属性は rest で渡っていたが型に無かった。
  - 部品のテスト: `overlays.test.ts` (id へ移る・見つからなければ最初の部品・選んだ要素の Tooltip が出ず最初の Esc で閉じる)、
    `components.test.ts` (フォームの外の `type="submit" form=…` のボタンで送れる。`test/ButtonFormHarness.svelte`)。
  - エディタ: 外での書き換え (`ExternalChangeDialog`) と、同じく不意に出る本体の変化 (`BaseChangeDialog`) の初めのフォーカスを「後で」に
    (打っていた Enter・Space で開き直し・上書き・「自動で開き直す」の切り替えにならないように)。`TipButton` は `id` を Tooltip のトリガーの id にも渡す
    (Bits UI がトリガーを id で探すため)。テストは `ExternalChangeDialog.test.ts` (5 つの場合で「Later」に行く・最初の Esc で `ondismiss`)。
  - 文書: docs/ui.md の部品の表 (`Button` の `form`・`Dialog` の `initialFocus`)、packages/ui/README.md、docs/editor.md#ダイアログ (表とフォーカスの行)。
- **検査:** `just ui-shots` はすべて ok。`just ui-audit --only editor --compare` (前: この作業の前の 7c9283c2 を `target/ui-audit-before-t0953` に撮った) は
  +0 -0 ~0 (前も後も 0 件。ただし監査の場面に外での書き換えのダイアログは無い)。`just e2e --only editor` は 15 件すべて通った。`just ci` は通った。
- **やらなかったこと:**
  - `NewModuleDialog`・`ModuleSettingsDialog` の決定のボタンを `form` で送る形には直していない (今は `onclick` と `<form>` の送信が同じ関数を呼んでいて
    困っていない。T1632 が apps/editor を並行して触っている)。
  - 実物の WebView (WebKitGTK・WebView2・WKWebView) で外での書き換えのダイアログを出して初めのフォーカスを見てはいない (happy-dom の部品のテストだけ)。
- **起票の候補:**
  - ui-audit に外での書き換え・本体の変化のダイアログの場面を足す (今は撮られていない)。
