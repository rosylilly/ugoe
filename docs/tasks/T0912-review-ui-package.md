---
id: T0912
title: コードレビュー: @ugoe/ui
status: in-progress
milestone: M9
crates: [packages, docs]
depends_on: []
assignee: claude-worker-T0912
---

# T0912: コードレビュー: @ugoe/ui

## 目的

@ugoe/ui のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `packages/ui` のすべて

## 受け入れ条件

- [ ] 範囲のコードを、テストを含めてすべて読んだ
- [ ] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - Bits UI の包み方の一貫性、props の名前と既定値、`mergeAttributes` などの公開の範囲
  - 部品のテスト (happy-dom) の穴、docs/ui.md の部品の一覧との一致 (見た目の見直しは M10 の T0916・T0917)
- [ ] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [ ] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [ ] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [ ] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0920〜T0924 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)
