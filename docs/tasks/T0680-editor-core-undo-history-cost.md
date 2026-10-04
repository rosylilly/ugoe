---
id: T0680
title: Undo の履歴の大きさと、適用ごとの検証の重複を抑える
status: in-progress
milestone: M9
crates: [ugoe-editor-core, docs]
depends_on: [T0667]
assignee: claude-worker-T0680
---

# T0680: Undo の履歴の大きさと、適用ごとの検証の重複を抑える

## 目的

T0667 のレビューの所見。`Document` はスナップショット方式の Undo (ADR 0005 の暫定の実装) で、

- `undo` の履歴に上限がない。コマンドごとに `Model`・物理・モーションと表情を丸ごと複製して積むので、長い編集の作業でメモリが増え続ける
  (テクスチャの画像は `Arc` で共有するので含まない)。
- `Document::apply_with_report` は、適用のたびに適用前のモデルも `validate` し直す (`validate(&self.current.model)`)。適用前の状態の検証の結果は
  状態が変わらない限り同じなので、スナップショットに覚えておける。物理の検証 (`validate_physics`) も同じ。

履歴の上限 (数か、おおよその大きさ) を決めて古いものから捨てる (保存済みの印 `saved` との関係に注意する)、適用前の検証の結果を覚える、を行う。
上限の値と、上限を超えたときの画面の扱い (知らせるか) は `docs/editor.md#undo` に書く。逆操作の方式への移行 (ADR 0005) はこのタスクではしない。

## 受け入れ条件

- [ ] 履歴が上限を超えると古いものから捨てられ、`is_modified` が正しいまま (テストで確かめる)
- [ ] 適用前の状態の検証を、同じ状態で 2 度行わない
- [ ] 大きなモデル (`ugoe_model::fixtures` の最大のもの) で、上限まで編集したときのメモリのおおよその量を測ってメモに書いた
- [ ] `just ci` が通る

## 参考

- docs/adr/0005-editor-command-model.md
- docs/editor.md#undo
- T0667 のメモ

## メモ
