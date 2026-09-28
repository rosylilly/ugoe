---
id: T0115
title: 互換性テストの資料と CI
status: todo
milestone: M4
crates: [ugoe-format, assets/compat, CI]
depends_on: [T0109]
assignee:
---

# T0115: 互換性テストの資料と CI

## 目的

各リリースのファイルを `assets/compat/` に保存し、新しい機能を含むファイルを古い読み込みルールで劣化させて読めることを CI で検査する。

## 受け入れ条件

- [ ] 資料の作り方と追加の手順を `docs/testing.md` に書く
- [ ] 未知の任意機能を含むファイルを、フォールバックを適用して読めるテストがある
- [ ] CI で実行される
- [ ] `just ci` が通る

## 参考

- `docs/adr/0013-forward-compatibility.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
