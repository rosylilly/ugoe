---
id: T0647
title: エディタ: 本体にスロットを定義する
status: in-progress
milestone: M4
crates: [ugoe-editor-core, apps/editor, docs]
depends_on: [T0121]
assignee: claude-worker-T0647
---

# T0647: エディタ: 本体にスロットを定義する

## 目的

ADR 0024。本体のプロジェクトで、着せ替えのスロットを定義・編集できるようにする。

## 受け入れ条件

- [ ] スロットを足す・変える・消す `Command` (Undo と検証のテスト)。アバタープロファイルの `avatar.slots` との関係を扱う
- [ ] スロットの画面: 取り付け先 (anchor・part)、描画順の範囲、置き換える要素、公開するパラメータ・衝突体・ポーズの要素。描画順の範囲をビューポートやツリーで確かめられる
- [ ] ヘッドレスのブラウザで確かめる。操作マニュアルに着せ替えの章 (`docs/manual/dress-up.md`) を作り、スロットの節を書く
- [ ] `just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/adr/0024-module-authoring.md`、`docs/spec/modules.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
