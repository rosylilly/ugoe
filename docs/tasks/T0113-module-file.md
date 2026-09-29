---
id: T0113
title: 着せ替えモジュールのファイル形式
status: todo
milestone: M6
crates: [ugoe-format]
depends_on: [T0111]
assignee:
---

# T0113: 着せ替えモジュールのファイル形式

## 目的

モジュールを `.ugoemod` (`MODU` チャンク) として読み書きする。

## 受け入れ条件

- [ ] `write_module` / `read_module` があり、往復できる
- [ ] 壊れた入力でパニックしない
- [ ] 機能 `modules` を宣言する
- [ ] `just ci` が通る

## 参考

- `docs/spec/file-formats.md#機能の宣言と段階的な劣化`
- `docs/spec/modules.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
