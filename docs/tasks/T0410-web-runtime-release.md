---
id: T0410
title: npm: @ugoe/runtime の公開の準備・wasm の縮小・<ugoe-avatar> のフェイストラッキング
status: in-progress
milestone: M7
crates: [bindings/web, ugoe-wasm, docs]
depends_on: [T0404]
assignee: claude-worker-T0410
---

# T0410: npm: @ugoe/runtime の公開の準備と改善

## 目的

T0404 の後続の候補をまとめる。

## 受け入れ条件

- [ ] npm への公開の手順とリリースの自動化 (版の付け方・CI からの公開・`private: true` の外し方)。公開そのものは利用者が行う
- [ ] wasm の縮小 (いま約 8MB、gzip で約 2MB)。wasm-opt・機能の切り分け・不要な依存の見直し。数字をメモに書く
- [ ] `<ugoe-avatar>` のフェイストラッキング。GPL の `packages/face-tracking` は使えないので、MIT で書き直すか、ホストから入力を渡す口だけにするかを決める (ADR 0025 との関係)
- [ ] モーション入りの見本で `motion` 属性を確かめる。WebGPU が作れないときの WebGL2 への切り替えを試す方法を用意する
- [ ] `just ci`・`just wasm-check` が通る

## 参考

- `docs/tasks/T0404-npm-web-components.md` のメモ、`docs/adr/0025-web-runtime-package.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
