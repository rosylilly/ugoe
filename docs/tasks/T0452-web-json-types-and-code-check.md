---
id: T0452
title: "@ugoe/runtime: ほかの *_json() の型と、警告・エラーのコードの一覧の突き合わせ"
status: in-progress
milestone: M9
crates: [bindings/web, ugoe-wasm, docs]
depends_on: [T0434]
assignee: claude-worker-T0452
---

# T0452: @ugoe/runtime: ほかの *_json() の型と、警告・エラーのコードの一覧の突き合わせ

## 目的

T0438 で `<ugoe-avatar>` のイベントと警告を型付きにした (`bindings/web/src/warnings.ts` の和型) が、次が残った。

- `modules_json()`・`slots_json()`・`expressions_json()` など、ほかの `*_json()` の戻り値には TS の型と読む補助が無い (利用者が `JSON.parse` して形を推測する)。
- Rust の `LoadWarning::code` / `DressUpError::code` の一覧と `warnings.ts` の和型が食い違っても、機械では気づけない。
  wasm のテストか stub の検査で、知っている `code` の一覧を出して比べる。T0434 で足す読み込みのエラーのコードも同じ検査に入れる。

## 受け入れ条件

- [ ] `*_json()` の戻り値ごとに TS の型と読む補助を足し、テストがある (足さないものは理由を README に書いた)
- [ ] Rust の `code` の一覧 (`LoadWarning`・`DressUpError`・T0434 の読み込みのエラー) と TS の和型を比べる検査があり、`just ci` で流れる。片方だけに足すと失敗することを確かめた
- [ ] `bindings/web/README.md` を更新した
- [ ] `just ci` と `just wasm-check` が通る

## 参考

- T0438 のメモ (起票の候補)
- bindings/web/src/warnings.ts、crates/ugoe/src/warning.rs・dress_up.rs、T0435 (手書きの型の写しと生成物の比較)

## メモ

- T0019 で起票した (元の候補: T0438 の 2 つ)。読み込みのエラーのコード (T0434) も同じ検査に入れるので、その後に着手する。
