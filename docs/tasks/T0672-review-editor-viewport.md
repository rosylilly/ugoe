---
id: T0672
title: コードレビュー: エディタのフロントエンド (ビューポートとプレビュー)
status: todo
milestone: M9
crates: [apps/editor, docs]
depends_on: []
assignee:
---

# T0672: コードレビュー: エディタのフロントエンド (ビューポートとプレビュー)

## 目的

エディタのフロントエンド (ビューポートとプレビュー) のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `apps/editor/src/lib/viewport/` と `apps/editor/src/lib/preview/` のすべて

## 受け入れ条件

- [ ] 範囲のコードを、テストを含めてすべて読んだ
- [ ] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - WebGL2 の資源の解放・コンテキストの喪失、ポインタのキャプチャとイベントの解除
  - ギズモの座標変換 (高 DPI・拡大縮小)、ドラッグ中のコマンドの積み方
  - プレビューの wasm の資源の解放、毎フレームの確保
- [ ] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [ ] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [ ] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [ ] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ
