---
id: T0639
title: エディタ: 書き出しでテクスチャのアトラス化を選ぶ
status: todo
milestone: M4
crates: [apps/editor, ugoe-editor-core, docs]
depends_on: [T0114]
assignee:
---

# T0639: エディタ: 書き出しでテクスチャのアトラス化を選ぶ

## 目的

T0114 のアトラス化 (`Project::with_atlas`) を、エディタの `.ugoe` の書き出しから選べるようにする。

## 受け入れ条件

- [ ] 書き出しの画面でアトラス化の有無・最大の大きさ・余白を選べ、プロジェクトに設定が残る
- [ ] 書き出しの後に、テクスチャの枚数と描画の状態の切り替え回数の変化を示す
- [ ] 同じ画像の重複を除くか、着せ替えのモジュールのテクスチャをどう扱うかを決めて書く (T0114 のメモの後続の候補)
- [ ] `just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0114-texture-atlas.md` のメモ
- `docs/spec/file-formats.md#テクスチャのアトラス化`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
