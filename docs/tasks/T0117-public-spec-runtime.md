---
id: T0117
title: 公開仕様: 評価・描画・物理・モーション (英語)
status: todo
milestone: M5
crates: [spec, docs]
depends_on: [T0116]
assignee:
---

# T0117: 公開仕様: 評価・描画・物理・モーション (英語)

## 目的

ugoe 以外のランタイムが同じ見た目と動きを再現できるよう、評価 (補間・デフォーマ・グルー・ブレンドシェイプ・曲線補間・2 段階の評価)、描画 (合成・マスク)、物理 (振り子・動的物理・順序と中継・加算)、モーション・表情・エフェクト・フレーム内の順序を英語で書く。

## 受け入れ条件

- [ ] `spec/evaluation.md`・`spec/rendering.md`・`spec/physics.md`・`spec/motion.md`。式と順序を省略せず、決定性の要件 (浮動小数点の扱いの許容) を書く
- [ ] 互換の基準 (どこまで一致すればよいか。例: 頂点の位置の許容誤差、描画の画素の許容) を決めて書く
- [ ] `docs/spec/` との対応表を更新する
- [ ] `just ci` が通る

## 参考

- `docs/spec/evaluation.md`・`rendering.md`・`physics.md`・`motion.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
