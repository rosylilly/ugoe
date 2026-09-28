---
id: T0702
title: アバタープロファイルに準拠した見本のモデル
status: todo
milestone: M3
crates: [ugoe-cli, assets, docs]
depends_on: [T0110]
assignee:
---

# T0702: アバタープロファイルに準拠した見本のモデル

## 目的

フェイストラッキング (T0306・T0703) で表情が付くことを確かめるための、アバタープロファイルに準拠した見本のモデルを作る。
ARKit のブレンドシェイプから計算するパラメータ (目の開閉・視線・眉・口の開き・口の形・頭の向きなど) が、見て分かる形で絵を動かすこと。

## 受け入れ条件

- [ ] `assets/fixtures/character.psd` (または `psd-real/` の PSD) から、`ugoe_editor_core::Command` の列でリグを組むジェネレータ (例: `ugoe sample avatar -o …`) がある。手書きの座標の定数は最小限にし、レイヤーの名前から要素を見つける
- [ ] 標準パラメータのうち、最低 `AngleX/Y/Z`・`EyeLOpen`・`EyeROpen`・`EyeBallX/Y`・`BrowLY`・`BrowRY`・`MouthOpenY`・`MouthForm` が絵を動かす。準拠レベル (決めたもの) を満たし、`ugoe validate --profile avatar` を通る
- [ ] 生成した `.ugoe` をコミットするか、ビルド時に作るかを決める (コミットするなら大きさと出所を `assets/fixtures/README.md` に書く)
- [ ] 各パラメータの端の値で `ugoe render` した絵を目で確かめ、メモに書く (必要なら小さな画像を `docs/` に)
- [ ] `just ci` が通る

## 参考

- `docs/spec/avatar-profile.md`
- `docs/manual/tutorial.md` (character.psd のリグの組み方)

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
