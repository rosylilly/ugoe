---
id: T0125
title: wasm: 読み込みのほかの警告 (物理を外した・テクスチャを代用した) も構造化されたデータで返す
status: todo
milestone: M4
crates: [ugoe-wasm, apps/viewer, apps/editor, docs]
depends_on: [T0124]
assignee:
---

# T0125: wasm: 読み込みのほかの警告も構造化されたデータで返す

## 目的

T0124 の後続の候補。付け直しで外れたモジュールは構造化されたデータ (`detached_modules_json`) で返すようになったが、読み込みのほかの警告
(物理の設定を外した・テクスチャを代用したなど、`crates/ugoe-wasm/src/load.rs`・`animation.rs`) は英語の文のままで、ビューアとエディタのプレビューでは英語で出る。

## 受け入れ条件

- [ ] 読み込みの警告を、種類 (`code`) と補足の値を持つ構造化されたデータで返す。T0124 の `detached_modules_json` と形を揃えるか、1 つの一覧にまとめるかを決めて README に書く。今の文の警告の扱いは T0124 と同じ方針にする
- [ ] ビューアとエディタのプレビューが、構造化されたデータから日本語 (ビューア) / 英語 (エディタ) の知らせを作る
- [ ] Rust のテストと Vitest。`just wasm-check`
- [ ] `just ci`・ビューアとエディタのビルドが通る

## 参考

- `docs/tasks/T0124-wasm-dress-up-structured-warnings.md` のメモ、`crates/ugoe-wasm/README.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
