---
id: T0316
title: コードレビュー: ugoe-physics
status: todo
milestone: M9
crates: [ugoe-physics, docs]
depends_on: []
assignee:
---

# T0316: コードレビュー: ugoe-physics

## 目的

ugoe-physics のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `crates/ugoe-physics` のすべて

## 受け入れ条件

- [ ] 範囲のコードを、テストを含めてすべて読んだ
- [ ] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - `docs/spec/physics.md` との一致、`dt` が 0・巨大・負のときの安定性、NaN の伝播
  - 衝突 (T0311) とリグの順 (T0312)・加算の出力 (T0313) の正しさ
- [ ] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [ ] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [ ] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [ ] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ
