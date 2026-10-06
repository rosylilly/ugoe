---
id: T0968
title: "マニュアル: 残りの画像 (チュートリアルの途中・モジュール・点光源・スタジオの出力と仮想カメラ) を撮り直す"
status: done
milestone: M10
crates: [tests, docs]
depends_on: [T0972, T1650, T1654]
assignee: claude-worker-T0968
---

# T0968: マニュアル: 残りの画像 (チュートリアルの途中・モジュール・点光源・スタジオの出力と仮想カメラ) を撮り直す

## 目的

T0919 で `just e2e --manual` (本物の Tauri のアプリ、WebKitGTK・Xvfb) でマニュアルの画像 40 枚を撮り直したが、筋書きの要る画像は前のまま残った。
今の画面と合わない画像を、同じ仕組みで撮り直す。AI と MCP の画像は T0969 で扱う。

## 受け入れ条件

- [x] チュートリアルの途中の画像 (`keyform-grid`・`mesh-edit`・`mirror`・`depth-rotation`) を、チュートリアルの手順を筋書きで通して撮った
      (`tests/e2e/specs/editor.manual.mjs`)。手順と画像が合わなければ docs/manual/tutorial.md の文を直した
- [x] モジュールの作成と比較の画像 (`module-*` のうち T0919 で撮っていないもの。`module-glue.png` は「Pairs」の一覧 (T1648) が写る) と
      `lighting-point.png` を撮った
- [x] スタジオの `studio-output.png`・`studio-virtual-camera.png`: コンポジタ・v4l2loopback を e2e で用意できれば撮る。用意できなければ、
      撮り方を docs/manual-check.md に手で撮る項目として書いた
- [x] 撮った画像が `--update-images` で 256 色になって docs/manual/images に入り、マニュアルの文と画像の説明が合っている
- [x] tests/e2e/README.md#マニュアルの画像 の一覧を直した
- [x] `just e2e --only editor` (筋書きを足したアプリ) と `just ci` が通る

## 参考

- T0919 のメモ (やらなかったこと・起票の候補)・docs/roadmap.md の M10 の「後続の候補 (T0919)」、T1648 のメモ (module-glue.png)
- tests/e2e/README.md#マニュアルの画像、docs/manual/README.md

## メモ

- T0046 で起票した (元の候補: T0919 (残りのマニュアルの画像)・T1648 (module-glue.png の撮り直し))。
- 写る画面を先に直すタスク (T0972 のマニュアルの文・T1650 のグルーの行・T1654 のツリーの札) の後にした。

### やったこと (claude-worker-T0968)

- **チュートリアル (`editor.manual.mjs` の `tutorial: …` の 6 手順):** `character.psd` から 3〜10 章を手順のとおりに画面で操作する
  (メッシュ編集・顔のワープ / 頭の回転 / 目のワープで包む・パラメータ 5 つ・キー・ワープの点の四角選択とドラッグ・角度の欄・
  パラメータと要素のミラー・深度 (Dome)・深度からの回転と「Mark derived」「Infer (overwrite derived)」)。`mesh-edit`・`keyform-grid`・
  `mirror`・`depth-rotation` を撮る。チュートリアルに書いた ID (`WarpGroup18`・`RotationGroup18`・`WarpLayer13`・`WarpLayer12`)、
  ミラーの報告 (`Mirrored 1 cell from …. Kept 1 cell authored (not overwritten).`)、深度の要約 (`16 control points · 0 – 35.28 px`) が
  画面に出るのを待つので、文と画面がずれると筋書きが失敗する。
- **モジュール (`module-*` の 15 枚) と `lighting-point`:** 見本の cat-ears で設定・スロットの移し替え (keep-look)・アトラスで書き出し・
  書き出したファイルとの比較 (同じ → 耳を消す → Band の頂点と描画順を変える → 名前を押して強調)、sailor で袖のポーズの追従を外して
  比較 (Wave を見せて強調し「Fit」)、long-hair でグルー (Back + Front、重み 0.25、「Pairs」の一覧が写るようにインスペクタを最も高くする) と
  「Add pairs」、本体の要素 (Head) の選択、TwinTails・Sailor との差し替え、「New Module…」の欄。本体の問題は、見本の本体の写しから
  スロット Headwear を消したもの・物理を消したものを作業の場所に作り、その隣に写したモジュールを開いて撮る。点光源は「Add point」の
  取っ手を光の円の中心から右へドラッグする。
- **道具:** `lib/webdriver.mjs` に `pointer` (マウスで点の列をたどる。Shift を押したままも)・`moveTo` と、キー (Backspace・End・Home・
  ArrowUp・ArrowRight) を足した。`lib/manual.mjs` の `resizeTo` は、窓を小さくした後にページ全体が送られて残る (ヘッダーが 6px 欠ける。
  WebDriver のポインタの位置もずれる) のを頭に戻す。筋書きの `settle` は、撮る前にポインタをヘッダーの空いた所へ動かす (Tooltip と
  ホバーの輪郭を写さない)。「Edit mesh」はページの中で押す (無効の「Add midpoint」の Tooltip が開いたまま写ったため)。
- **画像:** `just e2e --update-images --only editor` で、上の 20 枚と T0919 で撮ったエディタの 24 枚を撮り直した (T0919 の後の画面の変更: ツリーの札
  (T1654)・回転の欄の補間した値 (T1660) など)。2 回目以降の撮り直しで、アニメーション (呼吸・物理) だけが違う画像は前のままにした。
- **マニュアルの文:** dress-up.md (T1654 の候補: 札 2 つの行は `base` の印を出さず、ほかの印の枠が破線。グルーの「Distance」の単位。
  画像の説明: グルーの Pairs、本体の要素の選択、比較の Band・Ears、ポーズの追従、プレビューの LongHair / TwinTails / Sailor)、
  tutorial.md (インスペクタの先頭の表示 `Mesh: 顔 (Layer8)`・`Deformer: 顔のワープ (WarpGroup18)`、確認の記録)、screen.md・README.md
  (`Mesh: …` の表示)、tools.md (T1660 の候補: キーの間の回転の欄に補間した値が出て押せない。tools-rotation.png の説明)。
- **スタジオ:** WSL2 には v4l2loopback も Xvfb のコンポジタ (picom など) も無く、e2e では用意できなかった。docs/manual-check.md に
  U06 (studio-output.png・studio-virtual-camera.png を手で撮る手順) を足した。README の「撮らないもの」も直した。

### 確かめたこと

- `just e2e --update-images --only editor`: 39 件すべて通過 (main 1812dba8 に rebase した後のビルドで)。`just e2e --only editor`: 16 件通過。
- `just ci`: 通過 (終了コード 0。rebase の後、前面で実行)。最初の `just ci` は `wasm-types` で、rebase で入った T1660 の
  `rotation_local_keyform` が gitignore の生成物 (apps/editor/src/lib/viewport/generated) に無くて失敗した。e2e のビルドで生成物が作り直され、通った。
- 撮った画像はすべて開いて、マニュアルの説明と合うことを見た。

### やらなかったこと

- studio-output.png・studio-virtual-camera.png は撮っていない (前の画像のまま。manual-check の U06)。
- AI と MCP の画像 (T0969)。
- チュートリアルの 11・12 章 (保存・書き出し・ugoe render) は画像が無いので筋書きで通していない。7.2 の回転は取っ手のドラッグでなく
  「Angle (this cell)」の欄で入れた (手順に書いた代わりの方法)。

### 起票の候補

- module-compare-pose.png: 「Fit」で寄せると袖 (細長い) だけが画面いっぱいになるので、筋書きで 3 回縮小している。「Fit」の寄せ方
  (要素の短い辺に合わせる・余白) を見直すかを決める (エディタ)。
- T1667 (見えない「本体」の文のはみ出し) が入ったら、module-base-readonly.png・module-workspace.png を撮り直して見比べる (見た目は変わらない想定)。

