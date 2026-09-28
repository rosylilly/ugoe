---
id: T0105
title: `.ugoe` バンドルの読み書き
status: todo
milestone: M1
crates: [ugoe-format]
depends_on: []
assignee:
---

# T0105: `.ugoe` バンドルの読み書き

## 目的

docs/spec/file-formats.md#バンドル-ugoe のチャンク形式を実装する。バイナリのシリアライズ方式を決める。

## 受け入れ条件

- [ ] `write_bundle` / `read_bundle` があり、モデル・物理・モーション・テクスチャを往復できるテストがある
- [ ] 未知のチャンクを読み飛ばすテストがある
- [ ] 壊れた入力 (切り詰め、不正なオフセット) でパニックせずにエラーを返すテストがある
- [ ] 採用したシリアライズ方式を仕様書に書く (大きな判断なら ADR も書く)
- [ ] wasm32 でビルドできる
- [ ] `just ci` が通る

## 参考

- `docs/spec/file-formats.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
