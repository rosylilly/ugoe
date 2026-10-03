---
id: T0670
title: コードレビュー: エディタのフロントエンド (外枠・状態・AI)
status: todo
milestone: M9
crates: [apps/editor, docs]
depends_on: []
assignee:
---

# T0670: コードレビュー: エディタのフロントエンド (外枠・状態・AI)

## 目的

エディタのフロントエンド (外枠・状態・AI) のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `apps/editor/src` のうち、App とルートの部品、`lib/` 直下の状態と補助 (document・selection・ipc・shortcuts・layout など)・ダイアログ、`lib/ai/`・`lib/agents/`

## 受け入れ条件

- [ ] 範囲のコードを、テストを含めてすべて読んだ
- [ ] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - 状態 (`document.svelte.ts`・`selection.svelte.ts`) の更新の流れ、`$effect` の後片付け・無限の更新
  - ショートカットとフォーカスの扱い、IPC の失敗時の表示
  - AI の提案の承認の流れ (承認の前に適用されないか)
- [ ] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [ ] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [ ] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [ ] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ
