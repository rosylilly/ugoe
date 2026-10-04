---
id: T0438
title: npm: <ugoe-avatar> のイベントの型と、読み込みの警告・着せ替えのエラーの TypeScript の型
status: done
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

- [x] `UgoeAvatarElementEventMap` (`load`: `CustomEvent<UgoeAvatarLoadDetail>`、`error`: `CustomEvent<{ error: Error }>`) を出し、
      `addEventListener` / `removeEventListener` で `e.detail` が型付けされる (型の検査のテストを足す)
- [x] 読み込みの警告の構造の型 (`code` の和型と補足の値。知らない `code` も受ける) と、それを読む補助 (`loadWarnings(puppet)` など) を出す。
      `load` イベントの `detail` に構造の警告を足すかを決める
- [x] `DressUpError` の `code` の和型と補足の値の型を出す (wasm-bindgen の例外を判定する補助を含めるかを決める)
- [x] README の例を型付きの書き方に直す
- [x] `just ci` が通る

## 参考

- bindings/web/src/avatar.ts、crates/ugoe-wasm/README.md#読み込みの警告-t0125、#着せ替え
- docs/tasks/T0124-wasm-dress-up-structured-warnings.md、docs/tasks/T0125-wasm-load-warnings-structured.md

## メモ

- やったこと (bindings/web):
  - `UgoeAvatarElementEventMap` (`Omit<HTMLElementEventMap, "load" | "error">` に `load`: `CustomEvent<UgoeAvatarLoadDetail>`・
    `error`: `CustomEvent<UgoeAvatarErrorDetail>` を足したもの) と `UgoeAvatarErrorDetail` を出し、`UgoeAvatarElement` に
    `addEventListener` / `removeEventListener` の重ね定義を足した (動作は `super` のまま)。`this` は `UgoeAvatarElement`。
    `avatar.test.ts` の `errorOf` を消し、型の検査 (`expectTypeOf`・`@ts-expect-error`。`pnpm check` の tsc が見る) を足した。
  - `src/warnings.ts` を新設: 読み込みの警告の型 (`code` ごとの `UgoeKnownLoadWarning` の和型と、知らない `code` の
    `UgoeUnknownLoadWarning`。合わせて `UgoeLoadWarning`)、`isKnownLoadWarning`・`loadWarnings(puppet)`・`detachedModules(puppet)`。
    `DressUpError` の `UgoeDressUpErrorCode` (README の 20 種) と補足の値 `UgoeDressUpErrorFields`、`UgoeDressUpError`・`isDressUpError`。
  - 決めたこと:
    - 知らない `code` を同じ和型に入れると、`code` で比べても補足の値の型が絞れない (`code: string` の要素が残る) ので、
      `isKnownLoadWarning` で先に絞る形にした。`DressUpError` は補足の値がどの種類でも同じ形 (省略可の文字列) なので、
      `code` を `UgoeDressUpErrorCode | (string & {})` にしただけで種類ごとには分けていない。
    - wasm-bindgen の例外を判定する補助 `isDressUpError` を含めた (`Error` で `name === "DressUpError"`、`code` が文字列)。
    - `load` イベントの `detail` に構造の警告 `warningDetails` を足した (`warnings` と同じ順・同じ数。要素の `warningDetails` も)。
      文の `warnings` は残す (T0124・T0125 と同じ理由)。読み込みのたびに `load_warnings_json()` を 1 回 parse する。
  - README の `<ugoe-avatar>` の例を TS にし、「読み込みの警告と着せ替えの失敗の型」の節を足した。
- 範囲外の小さな変更: crates/ugoe-wasm/README.md の「読み込みの警告」に、TS の型の置き場所と「`code` を増やしたら
  `bindings/web/src/warnings.ts` も同じ PR で直す」の 1 項目を足した。
- やらなかったこと: `examples/basic/index.html` (JS) は変えていない。`modules_json()` の型 (`warning_details` を含む) は出していない。
  型の和と Rust の `code` の一覧が食い違ったことを機械で検査する仕組みは無い (下の起票の候補)。
- 起票の候補:
  - `modules_json()`・`slots_json()`・`expressions_json()` など、ほかの `*_json()` の戻り値の TS の型と読む補助。
  - Rust の `LoadWarning::code` / `DressUpError::code` の一覧と `bindings/web/src/warnings.ts` の和型を突き合わせる検査
    (wasm のテストか stub の検査で、知っている `code` の一覧を出して比べる)。
