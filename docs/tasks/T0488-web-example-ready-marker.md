---
id: T0488
title: "bindings/web: 見本の window.ugoeExample を、操作の処理をすべて付けた後に置く (自動のテストが待つ印)"
status: todo
milestone: M9
crates: [bindings/web, tests/conformance, docs]
depends_on: []
assignee:
---

# T0488: bindings/web: 見本の window.ugoeExample を、操作の処理をすべて付けた後に置く (自動のテストが待つ印)

## 目的

`bindings/web/examples/basic/index.html` は `initRuntime()` の直後に `window.ugoeExample` を置き、その後で `<ugoe-avatar>` の登録や
操作の処理を付ける。自動のテスト (`tests/conformance/example.mjs`) が `window.ugoeExample` を待っても、まだ操作が効かない時がある (T0487 の候補)。
待つものを明確にする。

## 受け入れ条件

- [ ] 見本が、操作の処理 (バックエンドの選択・ボタンなど) をすべて付けた後に準備ができた印を出す (`window.ugoeExample` を置く場所を後ろにするか、`ready` の値を足す。どちらにしたかをメモに書いた)
- [ ] `tests/conformance/example.mjs` がその印を待つ。`just conformance --only example` (または見本の手順) が通る
- [ ] 見本の README (bindings/web/README.md の見本の節) に印のことを書いた
- [ ] `just ci` が通る

## 参考

- T0487 のメモ (起票の候補)
- bindings/web/examples/basic/index.html、tests/conformance/example.mjs、tests/conformance/README.md

## メモ

- T0043 で起票した (元の候補: T0487 (`window.ugoeExample` の置き場所))。
