---
id: T0667
title: コードレビュー: ugoe-editor-core (文書・コマンド・プロジェクト)
status: in-progress
milestone: M9
crates: [ugoe-editor-core, docs]
depends_on: []
assignee: claude-worker-T0667
---

# T0667: コードレビュー: ugoe-editor-core (文書・コマンド・プロジェクト)

## 目的

ugoe-editor-core (文書・コマンド・プロジェクト) のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `crates/ugoe-editor-core` の document・command・project・module_project・module_compare とそのテスト (編集の各モジュールは T0668)

## 受け入れ条件

- [ ] 範囲のコードを、テストを含めてすべて読んだ
- [ ] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - Undo と Redo の対称性 (適用して戻すと元に戻るか)、`Command` enum の網羅
  - `.ugoeproj` の保存と読み込みの往復、外からの変更 (T0641) との競合
  - 1000 行を超えるファイル (document.rs・module_compare.rs・project.rs) の分け方
- [ ] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [ ] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [ ] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [ ] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0677〜T0680 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)
