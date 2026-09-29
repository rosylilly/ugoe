---
id: T0119
title: 公開仕様: JSON Schema・例のファイル・同期の検査
status: todo
milestone: M5
crates: [spec, ugoe-model, scripts, justfile, .github/workflows, docs]
depends_on: [T0116]
assignee:
---

# T0119: 公開仕様: JSON Schema・例のファイル・同期の検査

## 目的

公開仕様に JSON Schema と例のファイル (適合性を確かめるためのモデルと期待する評価結果) を付け、日本語の開発用の仕様書と英語の公開仕様の食い違いを CI で見つけられるようにする。

## 受け入れ条件

- [ ] `spec/schema/` に `model.json` などの JSON Schema を生成して置く (`just gen` で作り、差分がないことを CI で確かめる)。`$id` とバージョン
- [ ] `spec/examples/`: 小さなモデルと、指定したパラメータの値での頂点の位置・不透明度などの期待値。ugoe のランタイムがそれを満たすテスト
- [ ] `docs/spec/` と `spec/` の同期の検査 (例: 両方の見出し・フィールドの一覧・機能の表の対応) を `just` のレシピと CI に入れる
- [ ] `just ci` が通る

## 参考

- `docs/adr/0021-public-specification.md`
- `crates/ugoe-editor-core/src/codegen.rs` (型の生成)

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
