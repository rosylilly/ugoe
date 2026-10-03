---
id: T0668
title: コードレビュー: ugoe-editor-core (編集の各モジュール)
status: in-progress
milestone: M9
crates: [ugoe-editor-core, docs]
depends_on: []
assignee: claude-worker-T0668
---

# T0668: コードレビュー: ugoe-editor-core (編集の各モジュール)

## 目的

ugoe-editor-core (編集の各モジュール) のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `crates/ugoe-editor-core` の deformer_edit・keyform・motion_edit・physics_edit・inference・import・atlas・grid・codegen・message_codes とそのテスト

## 受け入れ条件

- [ ] 範囲のコードを、テストを含めてすべて読んだ
- [ ] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - 各編集が `Command` として Undo できるか、検証の漏れ
  - message_codes とフロントエンドの翻訳のキーの対応 (ADR 0029)、codegen の出力と `generated/` の一致
  - ugoe-rigging・ugoe-runtime との重複
- [ ] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [ ] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [ ] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [ ] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0681〜T0684 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)
