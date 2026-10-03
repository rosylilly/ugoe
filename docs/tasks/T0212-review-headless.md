---
id: T0212
title: コードレビュー: ugoe-headless
status: in-progress
milestone: M9
crates: [ugoe-headless, docs]
depends_on: []
assignee: claude-worker-T0212
---

# T0212: コードレビュー: ugoe-headless

## 目的

ugoe-headless のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `crates/ugoe-headless` のすべて

## 受け入れ条件

- [ ] 範囲のコードを、テストを含めてすべて読んだ
- [ ] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - GPU が無い・アダプタが選べない環境でのエラー、大きさと fit の境界 (0・巨大な大きさ)
  - CLI と MCP サーバの両方から呼ばれる前提 (スレッド・時間の進め方) が崩れていないか
- [ ] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [ ] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [ ] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [ ] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0218〜T0222 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)
