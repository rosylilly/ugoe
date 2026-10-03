---
id: T0136
title: コードレビュー: ugoe-core と ugoe-model
status: in-progress
milestone: M9
crates: [ugoe-core, ugoe-model, docs]
depends_on: []
assignee: claude-worker-T0136
---

# T0136: コードレビュー: ugoe-core と ugoe-model

## 目的

ugoe-core と ugoe-model のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `crates/ugoe-core` と `crates/ugoe-model` のすべて (フィクスチャ・検証・シリアライズを含む)

## 受け入れ条件

- [ ] 範囲のコードを、テストを含めてすべて読んだ
- [ ] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - `ugoe-model` がデータだけを持っているか (補間・変形のロジックが紛れていないか)
  - 型と `docs/spec/model.md`・英語の公開仕様 (`spec/`) の一致、`SCHEMA_VERSION` と前方互換 (ADR 0013) の扱い
  - 検証 (validation) の漏れ: 参照の切れ・循環・範囲外の値が検証を通らないか
  - 公開 API の変更が要る所見は、直さずにインターフェース先行のタスクとして起票する
- [ ] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [ ] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [ ] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [ ] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0140〜T0144 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)
