---
id: T0814
title: コードレビュー: スタジオのバックエンド (src-tauri)
status: todo
milestone: M9
crates: [apps/studio, docs]
depends_on: []
assignee:
---

# T0814: コードレビュー: スタジオのバックエンド (src-tauri)

## 目的

スタジオのバックエンド (src-tauri) のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `apps/studio/src-tauri` のすべて (vcam は T0816)

## 受け入れ条件

- [ ] 範囲のコードを、テストを含めてすべて読んだ
- [ ] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - 外部の API (T0805): ループバック限定・トークン・許可・既定は切、入力の検証
  - ブラウザソースの URL (T0808) のサーバ、ホットキーの登録と解除、シーンとプロファイルの保存の壊れにくさ
  - 指揮役と受け手の間の通知の取りこぼし・順序
- [ ] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [ ] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [ ] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [ ] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ
