---
id: T0735
title: "ビューア: ポーズの要素を選ぶ画面"
status: todo
milestone: M9
crates: [packages/player, apps/viewer, docs]
depends_on: [T0734]
assignee:
---

# T0735: ビューア: ポーズの要素を選ぶ画面

## 目的

T0468 で見本のアバター (耳飾りの有無) と着せ替えの本体 (手を振る) がポーズを持つようになったが、ビューアにはポーズの要素を選ぶ画面が無い
(エディタのプレビューにはある。T0468 の候補)。wasm には `show_pose`・`pose_json` があるので、ビューアから見本のポーズを試せるようにする。

## 受け入れ条件

- [ ] モデルにポーズがあれば、ビューアにグループごとに表示する要素を選ぶ欄があり、選ぶと `show_pose` で切り替わる (ポーズが無ければ欄を出さない)
- [ ] モデルの読み直し・着せ替えの付け外しの後も、欄が今のポーズ (`pose_json`) と食い違わない
- [ ] `packages/player` の Vitest と部品のテストがある。文言は `@ugoe/i18n` のカタログ (日英) にある
- [ ] docs/manual のビューアの節を直した (画像の撮り直しは T0919 のメモに書き足す)
- [ ] `just ci` が通る

## 参考

- T0468 のメモ (起票の候補)
- crates/ugoe-wasm/README.md (`show_pose`・`pose_json`)、docs/spec/motion.md (ポーズ)

## メモ

- T0036 で起票した (元の候補: T0468 (ビューアのポーズ))。同じパネルの周りを触る T0734 の後に着手する。
