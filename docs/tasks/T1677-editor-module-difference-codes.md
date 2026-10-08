---
id: T1677
title: "エディタ: 着せ替えのモジュールの違いの文を、英語の文ではなくコードと引数で返して訳す"
status: in-progress
milestone: M8
crates: [ugoe-editor-core, apps/editor, docs]
depends_on: []
assignee: claude-worker-T1677
---

# T1677: エディタ: 着せ替えの違いの文をコードと引数で返す

## 目的

エディタのプレビューの着せ替えで、今の編集と書き出した `.ugoemod` の違いを出す (`compare_preview_module`。docs/editor.md#プレビューの着せ替え)。
違いの 1 行 (`ModuleDifference.text`。例: `Meshes: 2 → 1 (removed: Ears; changed: Band)`) と要素ごとの違い
(`DifferenceItem.text`。例: `vertices 12 → 14; draw order 500 → 520`・`added`・`removed`) は、`ugoe-editor-core` の
`module_compare.rs` と `module_compare/changes.rs` が英語の文で作るので、日本語の画面でも英語のまま出る (T0665 のメモ)。
バックエンドのエラーをコードで返したのと同じく (ADR 0029)、違いを **コードと引数** で返し、フロントエンドが @ugoe/i18n で文にする。

`ugoe-editor-core` はツール層で、型は ts-rs でエディタに渡すだけ (外部の公開 API・ファイル形式ではない) なので、利用者の判断は要らない。

## 受け入れ条件

- [ ] `ModuleDifference` と `DifferenceItem` の `text` を、種類 (コード) と引数 (数・ID・前後の値) の構造に置き換えた
      (例: 行は `{ kind: "count", what: "meshes", before: 2, after: 1, removed: [...], changed: [...] }`、要素の違いは
      `{ kind: "vertices", before: 12, after: 14 }` の並び・`added`・`removed`)。形は作業者が決め、docs/editor.md に表で書く。
      今 `text` で判断している所 (`differenceHighlight.ts` の `item.text === "added"` など) は種類で判断する
- [ ] エディタのフロントエンドが、日英のカタログ (`apps/editor` のカタログの `preview.*` の下) で文にする。数は複数形の規則
      (i18next の `count`) と、一覧は `formatList` を使う。文にする関数の単体テスト (日英、すべての種類) を書いた
- [ ] 違いの一覧の絞り込み (`filterDifferences`。行の文か要素の ID・違いの文で絞る) が、今の言語の文で絞れる
- [ ] `ugoe-editor-core` のテスト (`module_compare_tests.rs`) を、文ではなく構造で比べる形に直した。ts-rs の生成物 (`apps/editor/src/lib/generated/`) を作り直した
- [ ] 画面の文言が変わるので `just e2e --only editor` を流した (マニュアルの画像 `module-compare*` の文が変わるなら、撮り直すかを
      メモに書く)。`just ui-audit` のエディタの着せ替えの場面に、はみ出しが増えていない
- [ ] `just ci` が通る

## 参考

- T0665 のメモ (後続の候補)、docs/adr/0029-shared-ui-and-i18n.md (エラーのコード)、docs/editor.md#プレビューの着せ替え
- crates/ugoe-editor-core/src/module_compare.rs・module_compare/changes.rs・module_compare_tests.rs
- apps/editor/src/lib/preview/dressUp.svelte.ts・differenceHighlight.ts・DressUpSection.svelte

## メモ

- T0075 で起票した (元: docs/roadmap.md の M8 の「起票していない後続の候補」の「UI: 着せ替えの違いの文をコードと引数に (T0665)」)。
- T0665 のメモにある「書き出しの要約・着せ替えの知らせ・読み込みの警告は作った時の言語の文字列で残る」は別のこと (言語を切り替えたときの
  作り直し) なので、このタスクには含めない。
