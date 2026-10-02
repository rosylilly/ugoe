---
id: T0909
title: UI: テーマの切り替えの設定 (OS に合わせる・ライト・ダーク) を 3 つのアプリに
status: in-progress
milestone: M8
crates: [packages, apps/editor, apps/viewer, apps/studio, docs]
depends_on: [T0723, T0663, T0801]
assignee: claude-worker-T0909
---

# T0909: テーマの切り替え

## 目的

docs/ui.md のトークンはライトとダークを持つが、利用者が選ぶ画面がどのタスクにも無い (T0900 のメモ)。

## 受け入れ条件

- [ ] @ugoe/ui にテーマの設定の部品と保存の仕組み (OS に合わせる・ライト・ダーク)
- [ ] エディタ・ビューア・スタジオの設定に置く
- [ ] `just ci` が通る

## 参考

- docs/ui.md

## メモ
