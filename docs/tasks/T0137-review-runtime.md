---
id: T0137
title: コードレビュー: ugoe-runtime
status: in-progress
milestone: M9
crates: [ugoe-runtime, docs]
depends_on: []
assignee: claude-worker-T0137
---

# T0137: コードレビュー: ugoe-runtime

## 目的

ugoe-runtime のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `crates/ugoe-runtime` のすべて

## 受け入れ条件

- [ ] 範囲のコードを、テストを含めてすべて読んだ
- [ ] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - キーフォームの補間とデフォーマの評価が `docs/spec/evaluation.md` と一致するか (境界・外挿・退化した格子)
  - 決定性 (浮動小数点の演算の順序・HashMap の順序) と、毎フレームの確保
  - 変更フラグ (T0106) の立て忘れ・立てすぎ
- [ ] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [ ] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [ ] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [ ] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0145〜T0149 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)
