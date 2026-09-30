---
id: T0652
title: エディタ: プレビューの着せ替えの一覧のモジュールの警告を英語の知らせにする
status: todo
milestone: M4
crates: [apps/editor, docs]
depends_on: [T0125]
assignee:
---

# T0652: エディタ: プレビューの着せ替えの一覧のモジュールの警告

## 目的

T0125 の後続の候補。エディタのプレビューの着せ替えの一覧に出るモジュールの警告 (`entry.warnings`) が、wasm の文のまま出ている。
T0125 で `modules_json()` の要素に `warning_details` が足されたので、そこから英語の知らせを作る。

## 受け入れ条件

- [ ] `warning_details` から、読み込みの警告 (`preview/loadWarnings.ts`) と同じ言い回しの英語の知らせを作って一覧に出す
- [ ] Vitest。`just ci`・エディタのビルドが通る

## 参考

- `docs/tasks/T0125-wasm-load-warnings-structured.md` のメモ、`crates/ugoe-wasm/README.md` の「読み込みの警告 (T0125)」

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
