---
id: T0323
title: モーションと表情の数値の検証 (NaN・無限大・負の長さ)
status: todo
milestone: M9
crates: [ugoe-model, ugoe-format, ugoe-cli, docs]
depends_on: [T0317, T0141]
assignee:
---

# T0323: モーションと表情の数値の検証 (NaN・無限大・負の長さ)

## 目的

モーション (`ugoe_model::motion::Motion`) と表情 (`ugoe_model::expression::Expression`) の数値は、どこでも検証していない
(`ugoe_model::validate` はモデルのエフェクト・ポーズ・物理だけを見る)。MessagePack のバンドルは NaN と無限大を表せるので、
カーブの点の値が NaN のモーションを再生すると、パラメータが NaN になり、描画まで壊れる。T0317 のレビューで見つけた。

ランタイム (`ugoe-motion`) は T0317 で、NaN の `dt`・NaN の `duration`・極端に短い `duration` で止まらないようにしたが、
データの値そのものの検証は読み込み・検証の側で行う。

## 受け入れ条件

- [ ] モーションの検証を足す: `duration`・`fade_in`・`fade_out`・カーブの `fade_in` / `fade_out`・セグメントの点 (`t`・`v`、ベジェの制御点)・イベントの `t` が有限値であること。`duration` が負でないこと。セグメントの時刻が減らないこと (警告かエラーかを決める)
- [ ] 表情の検証を足す: `fade_in`・`fade_out`・各項目の `value` が有限値であること
- [ ] 検証をどこで呼ぶか決める (`ugoe validate`・バンドルの読み込み・`Puppet` にモーションを渡すとき)。読み込みで拒むなら前方互換 (ADR 0013) との関係を書く
- [ ] `docs/spec/model.md` の検証の節と、英語の `spec/model.md#model-validation` を同じ PR で直す
- [ ] `just ci` が通る

## 参考

- docs/tasks/T0317-review-motion.md (メモの所見の表)
- docs/spec/motion.md#カーブ、docs/spec/model.md#検証

## メモ

- T0008: 数値の有限性の検証の方針 (エラーにするかランタイムのフォールバックに任せるか) は T0141 (T0145・T0318 をまとめた) で決める。
  このタスクはデータ (モーション・表情) と呼び出す場所 (`ugoe-format`・`ugoe-cli`・`Puppet`) が違うのでまとめず、T0141 の方針に揃えて後で行う。
