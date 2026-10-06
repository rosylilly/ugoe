---
id: T0488
title: "bindings/web: 見本の window.ugoeExample を、操作の処理をすべて付けた後に置く (自動のテストが待つ印)"
status: done
milestone: M9
crates: [bindings/web, tests/conformance, docs]
depends_on: []
assignee: claude-worker-T0488
---

# T0488: bindings/web: 見本の window.ugoeExample を、操作の処理をすべて付けた後に置く (自動のテストが待つ印)

## 目的

`bindings/web/examples/basic/index.html` は `initRuntime()` の直後に `window.ugoeExample` を置き、その後で `<ugoe-avatar>` の登録や
操作の処理を付ける。自動のテスト (`tests/conformance/example.mjs`) が `window.ugoeExample` を待っても、まだ操作が効かない時がある (T0487 の候補)。
待つものを明確にする。

## 受け入れ条件

- [x] 見本が、操作の処理 (バックエンドの選択・ボタンなど) をすべて付けた後に準備ができた印を出す (`window.ugoeExample` を置く場所を後ろにするか、`ready` の値を足す。どちらにしたかをメモに書いた)
- [x] `tests/conformance/example.mjs` がその印を待つ。`just conformance --only example` (または見本の手順) が通る
- [x] 見本の README (bindings/web/README.md の見本の節) に印のことを書いた
- [x] `just ci` が通る

## 参考

- T0487 のメモ (起票の候補)
- bindings/web/examples/basic/index.html、tests/conformance/example.mjs、tests/conformance/README.md

## メモ

- T0043 で起票した (元の候補: T0487 (`window.ugoeExample` の置き場所))。
- やったこと:
  - `ready` の値を足す方にした (`window.ugoeExample` は最初に置いたまま。`initRuntime()` の失敗などの途中の状態も見えるように)。
    見本の `result` に `ready: false` を足し、操作の処理 (バックエンドの選択・paused・係数の欄・tracking・camera) と `<ugoe-avatar>` の
    `load`・`error` の処理を付け、`@ugoe/runtime/element` を登録した後に `result.ready = true` にする。
  - 要素の登録 (`import("@ugoe/runtime/element")`) を `load`・`error` の処理を付けた後へ移した (前は登録の後に処理を付けていたので、
    早く届いた `load` を取りこぼし得た)。属性 (`backend`) を決めてから登録する順は保った。
  - tests/conformance/example.mjs は `window.ugoeExample?.ready` を待ってから「camera (MediaPipe)」を押す (T0487 の `element` を待つ回避をやめた。
    モデルの読み込みは待たない。`setTracking` は読み込みの前でも入力を覚える)。
  - 文書: bindings/web/README.md の「見本 (`examples/basic`)」に `window.ugoeExample` と `ready` の段落、tests/conformance/README.md の
    見本の節に「待つもの」。
- 確かめたこと: `just conformance --only example` を 6 回 (すべて通った)、`just ci`。
- 確かめていないこと: CI のランナーの上では動かしていない (push しないため)。
- 起票の候補: なし。
