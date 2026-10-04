---
id: T0438
title: npm: <ugoe-avatar> のイベントの型と、読み込みの警告・着せ替えのエラーの TypeScript の型
status: in-progress
milestone: M9
crates: [bindings/web, docs]
depends_on: [T0425]
assignee: claude-worker-T0438
---

# T0438: <ugoe-avatar> のイベントの型と、警告・エラーの構造の型

## 目的

T0425 (bindings/web のレビュー) で見つけた、TypeScript から使うときの型の穴を埋める。公開 API (型) を足すので、レビューでは直さずに起票した。

- `<ugoe-avatar>` は `load` と `error` を `CustomEvent` で送るが、要素の型にイベントの対応 (`HTMLElementEventMap` の拡張や
  `addEventListener` の重ね定義) が無い。`avatar.addEventListener("error", (e) => e.detail.error)` は、TypeScript では
  `e` が `ErrorEvent` になって型の誤りになる (`load` は `Event`)。利用者は `as unknown as CustomEvent<...>` と書くしかない
  (`bindings/web/src/avatar.test.ts` の `errorOf`)。
- 読み込みの警告の構造 (`load_warnings_json()`。T0125) と、着せ替えの `DressUpError` (`e.code`・`e.module` など。T0124) は
  crates/ugoe-wasm/README.md に表で書かれているだけで、型が無い。`UgoeAvatarLoadDetail` も文の警告 (`warnings: string[]`) だけを持つ。

## 受け入れ条件

- [ ] `UgoeAvatarElementEventMap` (`load`: `CustomEvent<UgoeAvatarLoadDetail>`、`error`: `CustomEvent<{ error: Error }>`) を出し、
      `addEventListener` / `removeEventListener` で `e.detail` が型付けされる (型の検査のテストを足す)
- [ ] 読み込みの警告の構造の型 (`code` の和型と補足の値。知らない `code` も受ける) と、それを読む補助 (`loadWarnings(puppet)` など) を出す。
      `load` イベントの `detail` に構造の警告を足すかを決める
- [ ] `DressUpError` の `code` の和型と補足の値の型を出す (wasm-bindgen の例外を判定する補助を含めるかを決める)
- [ ] README の例を型付きの書き方に直す
- [ ] `just ci` が通る

## 参考

- bindings/web/src/avatar.ts、crates/ugoe-wasm/README.md#読み込みの警告-t0125、#着せ替え
- docs/tasks/T0124-wasm-dress-up-structured-warnings.md、docs/tasks/T0125-wasm-load-warnings-structured.md

## メモ
