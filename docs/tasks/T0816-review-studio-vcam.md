---
id: T0816
title: コードレビュー: スタジオの仮想カメラ (vcam)
status: in-progress
milestone: M9
crates: [apps/studio, docs]
depends_on: []
assignee: claude-worker-T0816
---

# T0816: コードレビュー: スタジオの仮想カメラ (vcam)

## 目的

スタジオの仮想カメラ (vcam) のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `apps/studio/vcam` のすべて (Windows の Media Foundation・Linux の v4l2loopback・macOS)

## 受け入れ条件

- [ ] 範囲のコードを、テストを含めてすべて読んだ
- [ ] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - すべての `unsafe` に `// SAFETY:` があるか、COM の参照の数・解放・スレッドの前提 (ADR 0028)
  - unsafe を使う範囲が最小か (安全な包みの外に漏れていないか)、デバイスが無い・奪われたときの扱い
- [ ] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [ ] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [ ] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [ ] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0829〜T0833 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)
