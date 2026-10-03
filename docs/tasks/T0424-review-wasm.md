---
id: T0424
title: コードレビュー: ugoe-wasm
status: todo
milestone: M9
crates: [ugoe-wasm, docs]
depends_on: []
assignee:
---

# T0424: コードレビュー: ugoe-wasm

## 目的

ugoe-wasm のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `crates/ugoe-wasm` のすべて

## 受け入れ条件

- [ ] 範囲のコードを、テストを含めてすべて読んだ
- [ ] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - JS へ出す API の一貫性・型 (TypeScript の宣言)・エラーの形 (T0124・T0125)
  - メモリ (解放の口・コピーの回数)、`@ugoe/runtime` (bindings/web) とのずれ (bindings/ は直さず起票する)
- [ ] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [ ] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [ ] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [ ] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ
