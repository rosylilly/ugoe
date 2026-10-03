---
id: T0525
title: コードレビュー: ugoe-psd と ugoe-mesher
status: in-progress
milestone: M9
crates: [ugoe-psd, ugoe-mesher, docs]
depends_on: []
assignee: claude-worker-T0525
---

# T0525: コードレビュー: ugoe-psd と ugoe-mesher

## 目的

ugoe-psd と ugoe-mesher のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `crates/ugoe-psd` と `crates/ugoe-mesher` のすべて

## 受け入れ条件

- [ ] 範囲のコードを、テストを含めてすべて読んだ
- [ ] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - 壊れた・巨大な PSD (長さの偽装・圧縮の展開・レイヤーの深さ) で止まらないか、確保の上限
  - メッシュの生成の退化 (透明だけのレイヤー・細い形・1px)、決定性
- [ ] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [ ] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [ ] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [ ] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0531〜T0535 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)
