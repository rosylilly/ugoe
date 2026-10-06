---
id: T1663
title: "エディタ: 提案のプレビューの帯が出ている間、その高さも ] [ で選んだ点を見せる判定の上の隠す範囲に入れる"
status: done
milestone: M10
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1663
---

# T1663: エディタ: 提案のプレビューの帯が出ている間、その高さも ] [ で選んだ点を見せる判定の上の隠す範囲に入れる

## 目的

T1653 で `]` `[` で選んだ点を見せるパンの判定を、ビューポートで見える矩形 (上の帯 `.strip` と右下の道具を除いた範囲。
`ViewportController.setCovered`) で行うようにした。AI の提案のプレビュー中に出る帯 (App.svelte の `.proposal-banner`。ビューポートの左上に
absolute で重なる) は入れていないので、その下に入る点が隠れたままになる (今は `.strip` と重なる)。

## 受け入れ条件

- [ ] 提案のプレビューの帯が出ている間、その下端 (`.strip` と重なるなら両方の下端の大きい方) を上の隠す範囲にし、帯が消えたら戻す
      (大きさが変わったら測り直す)。帯と `.strip` が重ならないように並べ直すかも決め、理由をメモに書いた
- [ ] 部品のテスト: 帯を出した状態で、帯の下の点を `]` で選ぶと帯の下の端から余白の所までパンする。帯を消すと前の範囲に戻る
- [x] docs/editor.md の「選んだ点を見える所へ」を直した
- [ ] `just ui-audit --only editor` (提案のプレビューの場面を含む組) で増えた指摘が無い。`just e2e --only editor` と `just ci` が通る

## 参考

- T1653 のメモ (やらなかったこと・起票の候補)
- apps/editor/src/App.svelte (`.proposal-banner`)、apps/editor/src/lib/tools/keys.ts (`visibleRectOf`・`revealPanOf`)、
  apps/editor/src/lib/viewport (`ViewportController.setCovered`)

## メモ

- T0052 で起票した (元の候補: T1653 (提案のプレビューの帯の高さを上の隠す範囲に入れる))。
- 調べた結果、前提が成り立たなかった: App.svelte の overlay は `{#if proposals.preview}` で帯を出し、`{:else if}` で ToolOverlay を出す
  (T0637 からこの形。T1653 の時点でも同じ)。プレビュー中は ToolOverlay (`.strip` と、`]` `[` を受けて `controller.reveal` を呼ぶ
  FormTool・DeformerGizmo) が外れるので、帯と `.strip` は同時に出ず、帯の下の点を `]` で選ぶことも起きない。ToolOverlay が外れると
  その片付けで上の隠す高さは 0 に戻る。docs/editor.md の AI の提案の節にも「編集のツールを隠します」とある。T1653 のメモの
  「今は .strip と重なる」は誤り。
- やったこと:
  - docs/editor.md の「選んだ点を見える所へ」に、提案のプレビューの帯を隠す範囲に入れない理由 (帯は編集の道具の代わりに出る) を書いた。
  - tests/e2e/specs/editor.test.mjs (タスクの `crates:` の外。1 行の検査なので行った): 提案のプレビューの帯が出ている間、
    `.viewport .strip` が無いことを確かめる (2 つの提案のプレビューの場面で通る)。前提が将来崩れたら気づける。
- やらなかったこと:
  - 帯の高さを `setCovered("top")` に渡すこと・並べ直し・部品のテスト。帯が出ている間は reveal を呼ぶものが無く、渡しても使われない
    (働かないコードになる)。受け入れ条件 1・2 はこの理由で満たしていない (チェックしていない)。
  - `just ui-audit --only editor` は流していない (画面の変更が無い)。
- 検査: `just e2e --only editor` 16 件すべて通過。`just ci` 通過。
- 起票の候補: 無し (提案のプレビュー中にも編集の道具を出す方針にするなら、そのとき帯を隠す範囲に入れる)。
