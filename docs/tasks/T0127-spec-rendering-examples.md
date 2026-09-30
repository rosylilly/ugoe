---
id: T0127
title: 公開仕様: 描画の例 (画素の期待値) と、独立の実装での検証
status: todo
milestone: M5
crates: [spec, tools/spec-check, docs]
depends_on: [T0120]
assignee:
---

# T0127: 公開仕様: 描画の例 (画素の期待値) と、独立の実装での検証

## 目的

T0120 の後続の候補。描画 (spec/rendering.md: 合成モード・マスク・乗算色とスクリーン色・ライティング) には例も独立の実装もない。

## 受け入れ条件

- [ ] 作業者は `crates/` と `docs/spec/` を読まず、`spec/` だけを参照する
- [ ] `spec/examples/` に描画の例 (小さな画像と画素の期待値・許容誤差) を足す。GPU の違いで揺れる部分 (ラスタライズの境目・フィルタ) を避ける形を決めて `spec/examples/README.md` に書く
- [ ] 参照実装がそれらの例を満たすことを確かめる (`just test-gpu` のテストの追加は team-lead に相談する)
- [ ] `tools/spec-check` に CPU の小さなラスタライザを足し、期待値と一致させる
- [ ] 見つかった仕様の抜け・曖昧さの一覧と `spec/` への修正 (`docs/spec/` の同期は team-lead に頼む)
- [ ] `just ci` が通る

## 参考

- `docs/tasks/T0120-spec-independent-reader.md`・`spec/rendering.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
