---
id: T0425
title: コードレビュー: bindings/web (@ugoe/runtime と <ugoe-avatar>)
status: todo
milestone: M9
crates: [bindings/web, docs]
depends_on: []
assignee:
---

# T0425: コードレビュー: bindings/web (@ugoe/runtime と <ugoe-avatar>)

## 目的

bindings/web (@ugoe/runtime と <ugoe-avatar>) のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `bindings/web` のすべて (src・stub・scripts・examples)

## 受け入れ条件

- [ ] 範囲のコードを、テストを含めてすべて読んだ
- [ ] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - `ugoe-wasm` の API との対応、TypeScript の型・エラーの形 (T0124・T0125) のずれ
  - `<ugoe-avatar>` の属性とイベント・資源の解放 (切り離し・再接続)・WebGL のコンテキストの喪失
  - MIT のパッケージに GPL のコード (packages/・apps/) が入っていないか、npm に出すファイルの範囲とサイズ (T0413)
- [ ] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [ ] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [ ] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [ ] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ
