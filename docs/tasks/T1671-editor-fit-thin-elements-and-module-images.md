---
id: T1671
title: "エディタ: 細長い要素で「Fit」が画面いっぱいに寄せすぎるのを見直し、モジュールのマニュアルの画像を撮り直す"
status: in-progress
milestone: M10
crates: [apps/editor, tests/e2e, docs]
depends_on: [T0061]
assignee: claude-worker-T1671
---

# T1671: エディタ: 細長い要素で「Fit」が画面いっぱいに寄せすぎるのを見直し、モジュールのマニュアルの画像を撮り直す

## 目的

T0968 で module-compare-pose.png を撮るとき、ビューポートの「Fit」で選んだ袖 (細長い) に寄せると、袖だけが画面いっぱいになり周りが
見えなかった。筋書き (`tests/e2e/specs/editor.manual.mjs` の `module-compare-pose`) は「Fit」の後に 3 回縮小して撮っている。
利用者が「Fit」を押したときも同じになるので、寄せ方 (要素の短い辺に合わせる・余白・寄せる大きさの上限など) を見直すかを決める。

あわせて、T0968 は T1667 (本体の札の見えない文) が入る直前に module-base-readonly.png・module-workspace.png を撮ったので、撮り直して見比べる
(見た目は変わらない想定)。

## 受け入れ条件

- [ ] 「Fit」の寄せ方を見直すかを決め、決めたことと理由を docs/editor.md (ビューポート) に書いた。変えるなら、細長い要素・小さい要素・
      何も選んでいないときの寄せ方を、ビューポートの部品のテスト (controller の `fit`) で確かめた
- [ ] 変えたなら、`module-compare-pose` の筋書きの縮小を外し (要るだけ残すなら理由をコメントに書く)、module-compare-pose.png を撮り直した
- [ ] module-base-readonly.png・module-workspace.png を `just e2e --manual` で撮り直し、前の画像と見比べた結果をメモに書いた
- [ ] マニュアル (docs/manual の「Fit」の説明があれば、その箇所と dress-up.md の画像の説明) が今の振る舞いと合う
- [ ] `just e2e --only editor` と `just ci` が通る

## 参考

- T0968 のメモ (起票の候補)、T1667
- apps/editor/src/lib/viewport/Viewport.svelte (`controller.fit()`)、tests/e2e/specs/editor.manual.mjs (`module-compare-pose`)、docs/manual/dress-up.md

## メモ

- T0058 で起票した (元の候補: T0968 (module-compare-pose.png の「Fit」)・T0968 (T1667 の後に module-base-readonly.png などを撮り直す))。
- 同じ editor.manual.mjs を触る T0061 の後にした。
