---
id: T0124
title: 着せ替え: 付け直しで外れたモジュールを構造化されたデータで返す
status: todo
milestone: M4
crates: [ugoe-wasm, apps/viewer, apps/editor, docs]
depends_on: [T0123, T0706]
assignee:
---

# T0124: 着せ替え: 付け直しで外れたモジュールを構造化されたデータで返す

## 目的

利用者の要望 (T0706 の後続の候補)。モデルを開き直したときに合わなくなって外れたモジュールの警告を、wasm は英語の文で返している。
ビューアはその文を照らし合わせて日本語にしているので、文面が変わると英語のまま出る。

## 受け入れ条件

- [ ] wasm が、付け直しで外れたモジュールを構造化されたデータ (モジュールの名前・スロット・`DressUpError` と同じ `code` と項目) で返す。今の文の警告はそのまま残すか、置き換えるかを決めて README に書く
- [ ] ビューア (`dressUp.ts`) とエディタのプレビュー (T0649) が、文の照らし合わせをやめて構造化されたデータを使う
- [ ] Rust のテストと Vitest。`just wasm-check`
- [ ] `just ci`・ビューアとエディタのビルドが通る

## 参考

- `docs/tasks/T0706-viewer-dress-up.md`・`docs/tasks/T0123-wasm-dress-up.md` のメモ、`crates/ugoe-wasm/README.md#着せ替え-t0123`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
