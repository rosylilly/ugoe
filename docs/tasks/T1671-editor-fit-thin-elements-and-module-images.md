---
id: T1671
title: "エディタ: 細長い要素で「Fit」が画面いっぱいに寄せすぎるのを見直し、モジュールのマニュアルの画像を撮り直す"
status: done
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

- [x] 「Fit」の寄せ方を見直すかを決め、決めたことと理由を docs/editor.md (ビューポート) に書いた。変えるなら、細長い要素・小さい要素・
      何も選んでいないときの寄せ方を、ビューポートの部品のテスト (controller の `fit`) で確かめた
- [x] 変えたなら、`module-compare-pose` の筋書きの縮小を外し (要るだけ残すなら理由をコメントに書く)、module-compare-pose.png を撮り直した
- [x] module-base-readonly.png・module-workspace.png を `just e2e --manual` で撮り直し、前の画像と見比べた結果をメモに書いた
- [x] マニュアル (docs/manual の「Fit」の説明があれば、その箇所と dress-up.md の画像の説明) が今の振る舞いと合う
- [x] `just e2e --only editor` と `just ci` が通る

## 参考

- T0968 のメモ (起票の候補)、T1667
- apps/editor/src/lib/viewport/Viewport.svelte (`controller.fit()`)、tests/e2e/specs/editor.manual.mjs (`module-compare-pose`)、docs/manual/dress-up.md

## メモ

- T0058 で起票した (元の候補: T0968 (module-compare-pose.png の「Fit」)・T0968 (T1667 の後に module-base-readonly.png などを撮り直す))。
- 同じ editor.manual.mjs を触る T0061 の後にした。

### やったこと (claude-worker-T1671)

- **寄せ方を変えた:** 筋書きの「Fit」はビューポートの右下の「全体を表示」(`controller.fit()`。キャンバス全体に戻すだけ) ではなく、
  違いの強調の知らせの「Fit」(`DifferenceOverlay.svelte` の `fit` → `differenceHighlight.ts` の `fitMove`) だった。`fitMove` は要素の範囲を
  ビューポートの 80% (`FIT_FILL`) に寄せていたのを、幅と高さの半分 (0.5) にした。倍率は縦横のうち先に届く方 (要素の長い辺の向き) で決まる
  ので、細長い要素も画面の半分で止まり、周りに要素ほどの余白が残る。短い辺に合わせる案は細い要素ほど寄りすぎ、画素の余白を足す案は小さい
  要素で周りが見えないので採らなかった。点・小さい要素は従来の 24px の下限のまま、描いたものが無ければ「Fit」は押せない (`found`)。
  決めたことと理由は docs/editor.md (「違いの強調」の「Fit」の段) に書いた。
- **テスト:** 受け入れ条件の「controller の `fit`」は全体に戻すだけで変えていないので、`differenceHighlight.test.ts` の `fitMove` で、
  ふつうの範囲・点 1 つ (下限)・縦と横に細長い要素・画面より大きい要素 (縮小)・点が無い / ビューポートの大きさ 0 (null) を確かめた。
- **筋書き:** `module-compare-pose` の 3 回の縮小を外した。撮り直した module-compare-pose.png は袖が画面の高さの半分ほど (前の画像の
  3 回縮小した後とほぼ同じ大きさ) になった。dress-up.md の「Fit」の説明 (画面の半分ほどで真ん中、周りも見える) と画像の説明
  (「少し縮小した」を消した) を直した。
- **撮り直し:** `just e2e --update-images --only editor` (39 件通過) で撮り、module-compare-pose・module-base-readonly・module-workspace の
  3 枚だけを入れた (ほかは戻した)。base-readonly と workspace を前の画像と比べると、ツリーの行の札 (`base`・`hidden`) と右の列の欄が
  4px ほど左に寄り、長い名前の省略が 1 文字早くなった (`HairFront` → `HairFr…`、`Hai…` → `Ha…`)。T1662 (WebKitGTK の重ねて描く
  スクロールバーの分を送る枠の行末に空ける) によるもので、T1667 の見えない文による違いは見えない (想定どおり)。ほかの違いは顔の
  アニメーション (目の光) だけ。3 枚とも開いて、/home・worktree・mbx のハッシュ・一時フォルダが写っていないことを見た。

### 確かめたこと

- `pnpm exec vitest run src/lib/preview/differenceHighlight.test.ts`: 9 件通過。
- `just e2e --update-images --only editor`: 通過。`just e2e --only editor`: 16 件通過。
- `just ci`: 通過 (main 575d80be に rebase した後、前面で実行。終了コード 0)。

### やらなかったこと

- ほかのエディタのマニュアルの画像 (36 枚) も、撮り直すと T1662 の行末の空き (4px) などで違いが出たが、このタスクの範囲の 3 枚だけを入れた。

### 起票の候補

- T1662 (送る枠の行末の空き) の後に、エディタのマニュアルの画像 (ツリー・インスペクタが写る 36 枚ほど) を撮り直す。
