---
id: T1649
title: "エディタ: 回転デフォーマの原点を数値の欄 (親の座標系の値) で入れる"
status: done
milestone: M10
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1649
---

# T1649: エディタ: 回転デフォーマの原点を数値の欄 (親の座標系の値) で入れる

## 目的

T1636 で回転デフォーマの今のセルの角度と拡大率を数値の欄で入れられるようになったが、原点はギズモのドラッグと矢印キーでしか変えられず、
正確な値を入れられない (T1636 の候補)。

## 受け入れ条件

- [x] 回転デフォーマを選び、キーの上にいるとき、インスペクタの「Keyforms」の節に原点の X・Y の `NumberField` (親の座標系の値) がある。確定でギズモと同じコマンドを送り、Undo が 1 回で戻る
- [x] 「Origin: all cells」が入のときは、矢印キーと同じくすべてのセルに同じ差を足す。キーの間 (セルが無いとき) の見せ方は T1636 の角度・拡大率の欄とそろえる
- [x] 部品のテスト (確定で送るコマンドとその値・すべてのセル)
- [x] `just ui-audit --only editor` の `-rotation`・`-rotation-between` の指摘が増えない。docs/editor.md (インスペクタの表・代わりの手段の表) を直し、マニュアルの更新の候補をメモに書いた
- [x] `just e2e --only editor` と `just ci` が通る

## 参考

- T1636 のメモ (やったこと・起票の候補)
- apps/editor/src (インスペクタのキーフォームの節・`keyforms.ts` の `rotationCellCommand`・回転のギズモ)

## メモ

- T0043 で起票した (元の候補: T1636 (回転デフォーマの原点の数値の欄))。
- **やったこと:**
  - `KeyformInspector.svelte`: 回転デフォーマの角度・拡大率の欄 (T1636) の後に「Origin X (this cell)」「Origin Y (this cell)」の `NumberField`
    (親の座標系の値)。確定で `doc.send` から、送る直前の最新のモデルで組み立てたコマンドを送る (`keyforms.ts` の `rotationOriginCommand`)。
    切なら今のセルだけの `set_keyform_cell` (ギズモの原点のドラッグと同じ。ほかの値はそのセルのまま)、入なら今のセルの値との差を
    `move_rotation_origin` ですべてのセルに足す (矢印キーと同じ。差が 0 なら送らない)。1 回の確定が 1 つのコマンドなので Undo 1 回で戻る。
  - 「Origin: all cells」の状態をギズモの中の `$state` から `ViewportController.originAllCells` へ移した (インスペクタの欄が読むため。
    深度の「Show in viewport」の `showDepth` と同じ形)。チェックはギズモの帯のまま。デフォーマを選び直してもチェックが残るようになった
    (前はギズモが `{#key}` で作り直されるたびに切に戻っていた)。
  - 入のときは欄のラベルを「Origin X (all cells)」「Origin Y (all cells)」に変え、書き込む範囲を示す (日英の文言を足した)。
  - 刻み: 親がワープなら原点は親の UV なので刻み 0.01・小数 4 桁、親が回転デフォーマか無いならピクセルで刻み 1・小数 3 桁
    (`rotationOriginStep`)。見本の HairSway (親は HeadWarp) で 0.5・0.341 のような値になるのを ui-audit の画像で見て足した。
  - **キーの間:** 角度・拡大率と同じく、欄を残して空 (`optional` の `null`) にし押せなくする (入のときも同じ。差の元になるセルが無いため)。
  - テスト: `KeyformInspector.test.ts` (今のセルの原点の表示・確定で送る `set_keyform_cell` とその値・入のとき `move_rotation_origin` と差・
    キーの間で 4 つの欄が空で押せない・入のときのラベル)、`keyforms.test.ts` (`rotationOriginCommand`・`rotationOriginStep`)、
    `DeformerGizmo.test.ts` (チェックが controller に入り、矢印キーがすべてのセルを動かす)。
  - 文書: docs/editor.md (インスペクタの表・回転の中心の移動の「画面」・ギズモの節・代わりの手段の表)、docs/ui.md (代わりの手段の後続)。
- **`just ui-audit --only editor`:** 前 (同じ worktree で apps/editor を変更前に戻したもの) と後 (`--compare`) で +0 -0 ~0。全 412 場面で 0 件
  (`-rotation`・`-rotation-between` も 0 件)。画像 (en-light-wide-rotation・ja-light-narrow-rotation-between) で欄が角度・拡大率の下に並び、
  キーの間で空の押せない欄になることを見た。rebase と刻みの変更の後にもう一度流して +0 -0 ~0。
- **e2e:** `just e2e --only editor` は rebase の後に 15 件すべて通った (1 回目は「opens the sample project」が見本を開くのを 20 秒待って
  時間切れ。ほかのワーカーと並行して重かったためで、続けて流した 2 回目はすべて通った)。
- **`just ci`:** 通った (main 25032f62 に rebase した後、前面で実行、終了コード 0)。
- **やらなかったこと:**
  - マニュアル (docs/manual) の更新。**マニュアルの更新の候補:** tools.md の回転のギズモの節の「角度と拡大率は…数値で入れることも」の
    行に原点の X・Y の欄 (「Origin: all cells」でラベルが変わり、すべてのセルに足す) を足す。tutorial.md の 3 章 (中心を置き直す囲み) に、
    「Origin: all cells」を付けて欄に数を入れても置き直せることを足す。
  - インスペクタに「Origin: all cells」のチェックを重ねて置くこと (ラベルで状態を示すだけにした。チェックはギズモの帯にある)。
- **起票の候補:**
  - キーの間で補間した今の原点 (親の座標系) を押せない欄に出す (T1636 の候補と同じく、wasm の `rotation_frame` はワールドの変換なので要る口が別)。
