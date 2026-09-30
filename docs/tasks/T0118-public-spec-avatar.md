---
id: T0118
title: 公開仕様: アバタープロファイル (英語)
status: in-progress
milestone: M5
crates: [spec, docs]
depends_on: [T0116]
assignee: claude-worker-T0118
---

# T0118: 公開仕様: アバタープロファイル (英語)

## 目的

VRM のように、どのアプリでも同じモデルが動くための公開仕様として、アバタープロファイル (標準パラメータ・拡張パラメータ・ARKit との対応・向きの約束・メタデータと利用許諾・準拠レベル) を英語で書く。

## 受け入れ条件

- [ ] `spec/avatar-profile.md`: 準拠レベルごとの要件、パラメータの ID と値域と意味、トラッキングとの対応の式、上書きの規則、利用許諾の項目と既定値、`profile_version`
- [ ] アプリ (トラッキングのアプリ・ゲーム) の実装者向けの節: 何を読み、何を守るべきか (利用許諾の警告など)
- [ ] T0306・T0703 で残った符号の未決 (`MouthX`・`JawX`) を、利用者の確認の結果で解消するか、未決として明記する
- [ ] `just ci` が通る

## 参考

- `docs/spec/avatar-profile.md`
- `docs/adr/0012-avatar-profile.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
