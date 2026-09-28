---
id: T0306
title: トラッキングの対応付け (ARKit → パラメータ)
status: in-progress
milestone: M3
crates: [ugoe, ugoe-motion]
depends_on: [T0110, T0302]
assignee: claude-worker-T0306
---

# T0306: トラッキングの対応付け (ARKit → パラメータ)

## 目的

アバタープロファイルの対応表に従い、頭の向きと ARKit の 52 ブレンドシェイプからパラメータを計算する API を作る。モデルの上書き設定を反映する。

## 受け入れ条件

- [ ] `docs/spec/avatar-profile.md` の表のすべての式にテストがある
- [ ] モデルの `avatar.tracking` の上書きが効く
- [ ] 符号の約束 (`EyeBallX` など) を実際のトラッキングの値で確かめ、仕様の `> 未決:` を解消する (確かめられない場合はメモに残す)
- [ ] `just ci` が通る

## 参考

- `docs/spec/avatar-profile.md#フェイストラッキングとの対応`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
