---
id: T0918
title: UI 品質: スクリーンリーダーと OS の設定 (3 つのアプリ横断)
status: todo
milestone: M10
crates: [packages, apps/editor, apps/viewer, apps/studio, docs]
depends_on: [T0673, T0674, T0675, T0676, T0725, T0818]
assignee:
---

# T0918: UI 品質: スクリーンリーダーと OS の設定 (3 つのアプリ横断)

## 目的

T0907 で確かめていない、スクリーンリーダーでの読み上げと OS の設定 (動きを減らす・高コントラスト・200% の拡大) への対応を 3 つのアプリで確かめる。
基準は docs/ui.md (T0915 で足す「UI の品質の基準」を含む)。直せないもの・大きいものは後続のタスクにする (milestone は M10)。

## 受け入れ条件

- [ ] ランドマーク (header・nav・main・aside) と見出しの階層が、3 つのアプリで画面の構造と合っている
- [ ] 状態の変化 (保存した・AI の提案が届いた・エラー・シーンが変わった) を live region で知らせる (うるさくならない範囲で)
- [ ] NVDA (Windows) か Orca (Linux) で主な流れを読み上げ、名前・役割・状態が分かることを確かめた (確かめられない環境なら、手順を docs/manual-check.md に足して利用者の確認に回す)
- [ ] `prefers-reduced-motion`・`forced-colors` (Windows の高コントラスト)・WebView の 200% の拡大で崩れない
- [ ] docs/ui.md の「アクセシビリティの基準」に、確かめたことと確かめ方を足した
- [ ] `just ci` が通る

## 参考

- docs/ui.md
- docs/ui.md
- docs/manual-check.md

## メモ
