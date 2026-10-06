---
id: T1660
title: "エディタ: キーの間でも、補間した今の回転デフォーマの角度・拡大率・原点を押せない欄に出す"
status: done
milestone: M10
crates: [ugoe-wasm, apps/editor, docs]
depends_on: []
assignee: claude-worker-T1660
---

# T1660: エディタ: キーの間でも、補間した今の回転デフォーマの角度・拡大率・原点を押せない欄に出す

## 目的

インスペクタの回転デフォーマの数値の欄 (T1636 の角度・拡大率、T1649 の原点) は、キーの間では空 (`null`) で押せない。書き込むセルが無いので
数を出さないと決めた (T1636) が、今の姿がどの値で描かれているかが分からない。wasm の `WebPuppet.rotation_frame` はワールドの変換で、
親の変換を外した局所の値 (親の座標系の原点・角度・拡大率) を読む口が無い。ランタイムには補間した局所の値を返す
`ModelInstance::rotation_local_keyform` がある。

## 受け入れ条件

- [x] ugoe-wasm の `WebPuppet` に、回転デフォーマの今のパラメータ値で補間した局所の値 (原点・角度・拡大率。親の座標系) を返す口を足した
      (`rotation_local_keyform` を使う。回転デフォーマでない・モデルが無いなら `undefined`)。Rust のテスト (gizmo.rs など) と crates/ugoe-wasm/README.md
- [x] キーの間では、角度・拡大率・原点の欄に補間した今の値を、押せない (`disabled`) まま出す。書けるように見えない見せ方 (押せない欄の色・
      「interpolated」のような添え書き・読み上げの説明) を決め、T1636 の決め (空にする) を変える理由と一緒に docs/editor.md と docs/ui.md に書いた
- [x] パラメータを動かすと欄の値が追う。キーの上では今のとおりそのセルの値を押せる欄に出す
- [x] 部品のテスト (KeyformInspector.test.ts): キーの間で補間した値が押せない欄に出て、キーへ移るとセルの値で押せる。偽の WebPuppet にも口を足した
- [x] `just wasm-check`・`just ui-audit --only editor` (`-rotation-between` を含む組) で増えた指摘が無い。`just e2e --only editor` と `just ci` が通る

## 参考

- T1649 のメモ (起票の候補)、T1636 のメモ (キーの間の見せ方・やらなかったこと)
- crates/ugoe-wasm/src/gizmo.rs・web.rs (`rotation_frame`)、crates/ugoe-runtime/src/instance.rs (`rotation_local_keyform`)、
  apps/editor/src/lib/panels/inspector (KeyformInspector.svelte)

## メモ

- T0052 で起票した (元の候補: T1649 (キーの間で補間した今の原点を押せない欄に出す)・T1636 (キーの間で補間した角度・拡大率を見せる))。
  同じ欄の並びで、同じ wasm の口が要るので 1 つにまとめた。
- wasm の口は足すだけ (既存の口は変えない)。@ugoe/runtime (bindings/web) から出すかは範囲の外で、出すなら起票の候補にする。
- **やったこと:**
  - ugoe-wasm: `WebPuppet.rotation_local_keyform(id)` を足した (`gizmo::rotation_local`。`ModelInstance::rotation_local_keyform` の
    `[ox, oy, angle, scale]`。原点は親の座標系、角度は度。キーの間でも補間した値で、ブレンドシェイプの分も含む。回転デフォーマでない・
    モデルが無いなら `undefined`)。既存の口は変えていない。テストは `gizmo.rs` (`rotation_local_interpolates_between_keys_without_parents`:
    キーの間の補間・親の回転を掛けないこと・ワープと知らない ID は None)。README の「エディタ向けの問い合わせ」に書いた。
  - `KeyformInspector.svelte`: キーの間では 4 つの欄 (角度・拡大率・原点 X・Y) を `disabled` のまま、補間した今の値を出す
    (`keyforms.ts` の `rotationLocal` で読む。読めない・有限でないなら今までどおり空)。DeformerGizmo と同じく、モデル・状態・
    パラメータ値・`animationFrame` が変わるたびに wasm から読み直す (パラメータを動かすと欄が追う)。キーの上では今までどおりセルの値を
    押せる欄に出す。
  - **見せ方 (決めたこと):** 欄は無効の薄さ (`--ugoe-opacity-disabled`) で書けないことを示し、欄の下に「Interpolated from the keys around
    the current parameter values (read only).」(日本語も) の補足 (`.ed-hint`) を出して各欄の `aria-describedby` で結ぶ。ラベルはキーの上と
    同じ (「(this cell)」「(all cells)」) にした (スライダーでキーをまたぐたびに並びが動かないように。補足で補間した値だと分かる)。
    T1636 の「空にする」を変えた理由 (今の姿がどの値で描かれているかが分からない) と一緒に docs/editor.md (インスペクタの表) と
    docs/ui.md (使い分けの約束に「書き込めない今の値を数値の欄に出すとき」、代わりの手段の後続) に書いた。
  - テスト: `KeyformInspector.test.ts` (キーの間で補間した値が押せない欄に出て補足に結ばれる・パラメータを動かすと値が追い、キーへ移ると
    セルの値で押せて確定で set_keyform_cell を送る・WebPuppet が無ければ空・「all cells」の欄も補間した原点)、`keyforms.test.ts`
    (`rotationLocal`)。偽の WebPuppet (`harness.svelte.ts`) に `rotation_local_keyform` を足した。
  - 範囲外の小さな変更: `bindings/web/stub/ugoe_wasm.d.ts` (手書きの型の写し) に `rotation_local_keyform` を足した
    (`stub/ugoe_wasm.test.mjs` と `just wasm-types` が名前のずれを検査するため)。@ugoe/runtime の公開の型 (src/) には出していない。
- **wasm:** `just wasm-check` 通過。`just wasm` で生成物を作り直し、`just wasm-types apps/editor/src/lib/viewport/generated/ugoe_wasm.d.ts`
  で手書きの型の写しが生成物と一致した。
- **`just ui-audit --only editor`:** 前 (同じ worktree で apps/editor の src と locales を 25adecbf に戻してビルドしたもの) と後を、組を
  en-light・ja-light・en-dark・ja-dark の 4 つに分けて撮り (1 回で流すと 10 分を超えたため。`--no-build` で同じ dist を使った)、
  `--compare` でどれも +0 -0 ~0 (場面は 133・178・133・88)。`-rotation-between` はどの組も 0 件。画像 (en-light-wide-rotation-between) で、
  キーの間で補間した値が薄い押せない欄に出て、その下に補足が出ることを見た。
- **e2e:** `just e2e --only editor` は 16 件すべて通った (rebase の前)。
- **`just ci`:** 通った (main 02303eb4 に rebase した後、前面で実行、終了コード 0)。rebase の衝突は無し。
- **やらなかったこと:**
  - @ugoe/runtime (bindings/web) から `rotation_local_keyform` を出すこと (範囲の外)。
  - 反転 (`reflect_x`・`reflect_y`) と不透明度の補間した値を返すこと (インスペクタに欄が無いため。口は 4 つの数だけ)。
  - マニュアル (docs/manual の tools.md・tutorial.md) の更新。**マニュアルの更新の候補:** tools.md の回転のギズモの節の「角度と拡大率は…
    数値で入れることも」の行に、キーの間では補間した今の値が押せない欄に出る (書くにはキーへ移る) ことを足す。
- **起票の候補:**
  - @ugoe/runtime (bindings/web) の型と口に `rotation_local_keyform` を出すか決める (エディタ向けの問い合わせを公開するかの方針ごと)。
