---
id: T0913
title: コードレビュー: @ugoe/i18n と @ugoe/face-tracking
status: in-progress
milestone: M9
crates: [packages, docs]
depends_on: []
assignee: claude-worker-T0913
---

# T0913: コードレビュー: @ugoe/i18n と @ugoe/face-tracking

## 目的

@ugoe/i18n と @ugoe/face-tracking のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `packages/i18n` と `packages/face-tracking` のすべて

## 受け入れ条件

- [ ] 範囲のコードを、テストを含めてすべて読んだ
- [ ] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - 翻訳のカタログのキーの過不足 (日英)、複数形と引数、言語の切り替えでの再描画
  - カメラの資源の解放・推論のループの停止、third-party-notices の網羅
- [ ] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [ ] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [ ] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [ ] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0925〜T0929 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)
