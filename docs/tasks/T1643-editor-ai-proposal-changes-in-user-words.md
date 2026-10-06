---
id: T1643
title: "エディタ: AI の提案の「採用で変わるもの」を、コマンドの種類名でなく利用者の言葉 (「パラメータを足す × 5」) で出す"
status: done
milestone: M10
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1643
---

# T1643: エディタ: AI の提案の「採用で変わるもの」を、コマンドの種類名でなく利用者の言葉 (「パラメータを足す × 5」) で出す

## 目的

T0676 で AI の提案のカードを見直したが、「採用で変わるもの」はコマンドの種類の名前のまま出ている。種類ごとの訳 (「パラメータを足す × 5」
「キーフォームを置く × 12」など) にして、採用する前に何が変わるかを読めるようにする (T0676 の候補)。

## 受け入れ条件

- [x] エディタの翻訳のカタログに、提案に出るコマンドの種類ごとの名前 (英語・日本語。数を取る形) を足し、カードの「採用で変わるもの」がそれを使う
- [x] カタログに無い種類が来たときの出し方 (例 種類名のまま) を決め、テストがある
- [x] 部品か表示のテスト (種類ごとの数をまとめた文)。`just ui-audit --only editor` の AI の場面の指摘が増えない
- [x] docs/editor.md (AI の提案のカード) を直し、マニュアルの画像を撮り直すか、マニュアルの更新の候補をメモに書いた
- [x] `just e2e --only editor` と `just ci` が通る

## 参考

- T0676 のメモ (やらなかったこと・起票の候補)
- apps/editor/src/lib/ai/ (ProposalsPanel.svelte など)、エディタの i18n のカタログ

## メモ

- T0043 で起票した (元の候補: T0676 (「採用で変わるもの」をコマンドの種類ごとの訳に))。
- **やったこと:**
  - カタログ (`apps/editor/locales/{en,ja}.json`) に `proposals.changeKinds.*` を足した。`ugoe_editor_core::Command` の 109 種類すべてに、
    英語は数を取る文 (`add 5 parameters`・`place 12 keyforms`。`_one`・`_other`)、日本語は「パラメータを足す × 5」「キーフォームを置く × 12」の形。
  - `lib/ai/proposals.ts`: 種類とキーを並べた表 `CHANGE_KINDS` (キーを組み立てない。docs/translation.md)、`changeLabel(op, count)`
    (表に無ければ `null`。`Object.hasOwn` で `constructor` などを種類と取り違えない)、`changeSummary` の各行に `label` を足した。
  - カード (ProposalsPanel.svelte): `label` があればそれを、無い種類は前と同じく `<code>op</code> × N` (種類名のまま) で出す。
  - テスト: proposals.test.ts (`changeSummary` の `label`・`changeLabel` の英語と日本語・表に無い種類・表が `command.rs` の `Command` の
    すべての種類を持つこと)、ProposalsPanel.svelte.test.ts (カードの文・表に無い種類が `code` で出ること)。
  - 文書: docs/editor.md (提案のカード)、docs/ui.md (T0676 の節の提案のカード)、docs/manual/ai.md (「Adopting changes:」の説明と例)。
- **`just ui-audit --only editor`:** 前 (cf09669d の apps/editor を一時的に戻して撮った `target/ui-audit-before-t1643`) と後で、ともに 0 件・412 場面、
  `--compare` は `editor: +0 -0 ~0`。AI の場面の文は「Adopting changes: correct the classification of 3 layers · add 5 parameters · add 1 deformer ·
  `set_keyform_bindings` × 4 · place 5 keyforms」「採用で変わるもの: 法線マップを置く × 5 · ライティングを変える × 1」など。
  偽のホストの見本 (`tests/ui-audit/lib/editor-fixtures.mjs`) が実在しない種類 `set_keyform_bindings` を使っていて、表に無い種類の出し方で出る (範囲の外なので直していない)。
- **マニュアルの画像:** 撮り直していない。提案のカードが写る `docs/manual/images/ai-revision.png` は T0676 より前のカード (「Adopting changes:」の
  行もボタンの今の並びも無い) のまま。実物の WebView で撮り直す (T0919 の手順) のが更新の候補。文 (docs/manual/ai.md) は直した。
- **やらなかったこと:** 日本語の数の言い方を「5 個のパラメータを足す」のような助数詞の形にはしなかった (動詞ごとに助数詞が変わり、
  キャンバス・ライティングのように数えない変更もあるので、タスクの例の「× N」にそろえた)。
- **`just e2e --only editor`:** 15 件すべて通った。**`just ci`:** 通った (main 1050c1cd に rebase した後)。
- **起票の候補:**
  - マニュアルの AI の画像 (`ai-proposal.png`・`ai-revision.png` など、提案のカードが写るもの) の撮り直し。
  - ui-audit の偽のホストの見本の `set_keyform_bindings` を実在する種類 (`set_keyform_cell` など) に直す (表に無い種類の出し方を撮る場面が要るなら、別に作る)。
