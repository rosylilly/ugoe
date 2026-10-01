---
id: T0522
title: AI: 法線マップの提案の仕上げ (縁の幅の単位・丸みとならし・提案の対象の解析・英語の題名)
status: in-progress
milestone: M6
crates: [ugoe-ai, ugoe-editor-core, apps/editor, docs]
depends_on: [T0520]
assignee: claude-worker-T0522
---

# T0522: 法線マップの提案の仕上げ

## 目的

T0520 の後続の候補をまとめる。

## 受け入れ条件

- [ ] 「Edge width (px)」の単位が、まとまりの格子の画素で、キャンバスの px と紛らわしい (見本の顔で 148 と出るが、キャンバスでは約 67 px)。キャンバスの px で見せるか、単位を明示する
- [ ] AI タブで、縁の丸みとならしも選べるようにする
- [ ] editor-core の proposal::analyze が set_normal_map の対象のテクスチャを拾わない (T0520 では区切りの subjects で補った)。analyze を直す
- [ ] 提案の題名と理由が、英語の画面に日本語のまま出る (T0637 からの課題)。英語にする
- [ ] 眼鏡のほかに向きだけを受けたいラベルがあれば orientation_only に足す
- [ ] `just ci`・`just test-gpu`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0520-ai-normal-map-panel.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
