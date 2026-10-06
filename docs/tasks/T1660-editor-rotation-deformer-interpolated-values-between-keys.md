---
id: T1660
title: "エディタ: キーの間でも、補間した今の回転デフォーマの角度・拡大率・原点を押せない欄に出す"
status: in-progress
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

- [ ] ugoe-wasm の `WebPuppet` に、回転デフォーマの今のパラメータ値で補間した局所の値 (原点・角度・拡大率。親の座標系) を返す口を足した
      (`rotation_local_keyform` を使う。回転デフォーマでない・モデルが無いなら `undefined`)。Rust のテスト (gizmo.rs など) と crates/ugoe-wasm/README.md
- [ ] キーの間では、角度・拡大率・原点の欄に補間した今の値を、押せない (`disabled`) まま出す。書けるように見えない見せ方 (押せない欄の色・
      「interpolated」のような添え書き・読み上げの説明) を決め、T1636 の決め (空にする) を変える理由と一緒に docs/editor.md と docs/ui.md に書いた
- [ ] パラメータを動かすと欄の値が追う。キーの上では今のとおりそのセルの値を押せる欄に出す
- [ ] 部品のテスト (KeyformInspector.test.ts): キーの間で補間した値が押せない欄に出て、キーへ移るとセルの値で押せる。偽の WebPuppet にも口を足した
- [ ] `just wasm-check`・`just ui-audit --only editor` (`-rotation-between` を含む組) で増えた指摘が無い。`just e2e --only editor` と `just ci` が通る

## 参考

- T1649 のメモ (起票の候補)、T1636 のメモ (キーの間の見せ方・やらなかったこと)
- crates/ugoe-wasm/src/gizmo.rs・web.rs (`rotation_frame`)、crates/ugoe-runtime/src/instance.rs (`rotation_local_keyform`)、
  apps/editor/src/lib/panels/inspector (KeyformInspector.svelte)

## メモ

- T0052 で起票した (元の候補: T1649 (キーの間で補間した今の原点を押せない欄に出す)・T1636 (キーの間で補間した角度・拡大率を見せる))。
  同じ欄の並びで、同じ wasm の口が要るので 1 つにまとめた。
- wasm の口は足すだけ (既存の口は変えない)。@ugoe/runtime (bindings/web) から出すかは範囲の外で、出すなら起票の候補にする。
