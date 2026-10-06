---
id: T1666
title: "エディタ: プレビューの着せ替えで、GPU を失っている間・作り直しの途中の付け外しを、すぐ一覧のチェックとスロットの選択に出す"
status: todo
milestone: M10
crates: [apps/editor, docs]
depends_on: []
assignee:
---

# T1666: エディタ: プレビューの着せ替えで、GPU を失っている間・作り直しの途中の付け外しを、すぐ一覧のチェックとスロットの選択に出す

## 目的

T1658・T1659 で、GPU を失っている間と WebPuppet の作り直しの途中の付け外しは、付けたい組 (`wanted`) に残り、作り直した WebPuppet に
付くようになった。ただし一覧の行のチェックとスロットの選択は、実際に付いているか (`attached`) を写すので、作り直すまで付け外しが画面に出ない。
利用者には押しても変わらないように見える。

## 受け入れ条件

- [ ] GPU を失っている間と作り直しの途中は、行のチェックとスロットの選択が `wanted` を写す (か、待っていることを行に出す)。どちらにするかと、
      付けられなかった (作り直しの後に付け損ねた) ときの戻し方を決め、メモと docs/editor.md (プレビューの着せ替え) に書いた
- [ ] 待っている印を出すなら、文字か読み上げで伝え、色だけに頼らない
- [ ] 部品のテスト (DressUpSection・dressUp.test.ts): 失っている間・作り直しの途中にチェックを外す・スロットで替えると、すぐ行とスロットの選択に出て、
      作り直した後も同じ。作り直しが失敗したときの行の状態
- [ ] `just editor-browser` のシナリオ (`dress-up-change-while-lost` など) で、作り直す前に行のチェックが変わることを確かめた
- [ ] `just ui-audit --only editor` (`-preview-dress-up` を含む組) で増えた指摘が無い。`just e2e --only editor` と `just ci` が通る

## 参考

- T1659 のメモ (起票の候補)、T1658・T1656 のメモ、docs/ui.md#webpuppet-の寿命
- apps/editor/src/lib/preview/dressUp.svelte.ts (`wanted`・`attached`)・DressUpSection.svelte、apps/editor/browser-test/

## メモ

- T0052 で起票した (元の候補: T1659 (失っている間・作り直しの途中の付け外しが一覧に出ない))。
