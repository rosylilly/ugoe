---
id: T0110
title: アバタープロファイルのデータと検証
status: todo
milestone: M3
crates: [ugoe-model, ugoe-format, ugoe-cli]
depends_on: [T0109]
assignee:
---

# T0110: アバタープロファイルのデータと検証

## 目的

`Model::avatar` (準拠レベル・メタデータ・トラッキングの対応の上書き・アプリ向けの情報) と拡張パラメータを追加し、`ugoe validate --profile avatar` で準拠を検査する。

## 受け入れ条件

- [ ] `docs/spec/avatar-profile.md` のメタデータと拡張パラメータを型として定義する (許諾の既定値は最も制限の強い値)
- [ ] `ugoe_model::standard` に拡張パラメータを追加する
- [ ] 準拠レベルごとに必要なパラメータとメタデータを検査する関数と、`ugoe validate --profile avatar` がある
- [ ] 機能 `avatar.profile` を宣言する
- [ ] 仕様書の「実装待ち」を更新する
- [ ] `just ci` が通る

## 参考

- `docs/adr/0012-avatar-profile.md`
- `docs/spec/avatar-profile.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
