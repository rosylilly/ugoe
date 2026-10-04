---
id: T0548
title: AI: 名前の規則の取り違えを減らす (部位の語のないハイライト・リボン・top)
status: in-progress
milestone: M9
crates: [ugoe-ai, docs]
depends_on: [T0528]
assignee: claude-worker-T0548
---

# T0548: AI: 名前の規則の取り違えを減らす

## 目的

T0528 のレビューで見つけた、名前の規則 (`classify/rules.rs`) の取り違え。T0528 では髪のハイライト (`前髪ハイライト`・`back hair hl`) が
目のハイライトになるのを、髪の規則を目の規則より先に見ることで直した。残りは規則の並びだけでは直らないので、まとめて扱う。

- `highlight`・`hl`・「ハイライト」は部位の語がなくても目のハイライトになる。顔・頬・服のハイライト (`顔ハイライト`・`cheek hl`) も
  `eye_highlight` になり、目の開閉でつぶれる
- 「リボン」・`ribbon` はいつも `headwear`。首のリボン (`neck ribbon`・`胸リボン`) は `neckwear` にしたい (`docs` の語彙の説明では
  `neckwear` にリボンを含む)
- `top` は `topwear` の語なので、`hair top`・`top hair` が服になる

## 受け入れ条件

- [ ] 上の名前が期待するラベルになる (部位の語との組み合わせで決める規則など)。See-Through の PSD の正解 (`tests/classify.rs`) は変わらない
- [ ] 規則のテスト (`classify/rules.rs` の単体テスト)
- [ ] 規則の説明 (`docs/ai-integration.md#レイヤーの意味分類`) を直す (必要なら)
- [ ] `just ci` が通る

## 参考

- `crates/ugoe-ai/src/classify/rules.rs`
- `docs/tasks/T0528-review-ai-autorig.md` のメモ

## メモ
