---
id: T0612
title: ミラーと深度からの回転のアルゴリズム
status: todo
milestone: M2
crates: [ugoe-rigging]
depends_on: [T0610]
assignee:
---

# T0612: ミラーと深度からの回転のアルゴリズム

## 目的

`docs/spec/keyform-inference.md` のミラー (頂点の対応付け・反転) と深度からの回転を実装する。

## 受け入れ条件

- [ ] 頂点の対応付けが許容距離を超えたらエラーと対応しない頂点の一覧を返す
- [ ] ワープの制御点の列の反転、回転デフォーマの角度の符号反転のテストがある
- [ ] 深度からの回転で、中間・斜めのセルが式どおりになるテストがある
- [ ] `ugoe render` で振り向きを目視確認した結果をメモに書く
- [ ] `just ci` が通る

## 参考

- `docs/spec/keyform-inference.md#ミラー`
- `docs/spec/keyform-inference.md#深度からの回転`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
