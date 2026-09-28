---
id: T0310
title: M3 の受け入れテスト
status: todo
milestone: M3
crates: [ugoe, ugoe-editor-core, ugoe-cli, docs]
depends_on: [T0303, T0305, T0306, T0307, T0308, T0309, T0629, T0630, T0703]
assignee:
---

# T0310: M3 の受け入れテスト

## 目的

`docs/roadmap.md` の M3 の完了の条件を、自動のテストと記録された確認で示す。

## 受け入れ条件

- [ ] 物理・モーション・表情・目パチ・呼吸・リップシンク・ポーズが同じ `Puppet` で動くテスト (フレーム内の順序を含む)
- [ ] 揺れ用パラメータなしで髪 (回転デフォーマの鎖) が揺れ、体の衝突体にぶつかるテスト
- [ ] アバタープロファイルに準拠したモデル (T0702) が `ugoe validate --profile avatar` を通り、ARKit の値の列から期待するパラメータになるテスト
- [ ] エディタのコマンドで物理の設定とモーションを作り、保存・書き出し・再生できるテスト
- [ ] `ugoe render` などで、モーションの途中の絵を目で確かめられる手順を `docs/` に書く
- [ ] `docs/roadmap.md` の M3 を完了にする
- [ ] `just ci` が通る

## 参考

- `crates/ugoe-editor-core/tests/m2/` (T0616)

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
