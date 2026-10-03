---
id: T0529
title: コードレビュー: ugoe-mcp
status: todo
milestone: M9
crates: [ugoe-mcp, docs]
depends_on: []
assignee:
---

# T0529: コードレビュー: ugoe-mcp

## 目的

ugoe-mcp のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `crates/ugoe-mcp` のすべて (stdio と HTTP の transport・relay・host・tools・autorig)

## 受け入れ条件

- [ ] 範囲のコードを、テストを含めてすべて読んだ
- [ ] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - HTTP の transport の認証・ループバック限定・Origin の検査、ルートの外のファイルを読めないか (T0515)
  - ツールの入力の検証、提案と適用の分離 (AI はモデルを直接書き換えない)
  - エディタとの relay (T0642) の切断・再接続・競合
- [ ] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [ ] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [ ] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [ ] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ
