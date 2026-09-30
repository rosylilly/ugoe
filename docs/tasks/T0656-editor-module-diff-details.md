---
id: T0656
title: エディタ: モジュールの違いをメッシュの中身まで出す・違いの行からツリーで選ぶ
status: in-progress
milestone: M4
crates: [ugoe-editor-core, apps/editor, docs]
depends_on: [T0654]
assignee: claude-worker-T0656
---

# T0656: モジュールの違いの詳細

## 目的

T0654 の後続の候補をまとめる。

## 受け入れ条件

- [ ] 今の編集と書き出したモジュールの違いを、メッシュの中身 (頂点・キーフォーム・描画順) まで出す。アトラス化したファイルとは UV を元に戻して比べる
- [ ] 違いの行から、その要素をツリーで選べる
- [ ] 見本のモジュールをアトラス化して配るかを決める (決めたら assets/samples と見本の生成を直す)
- [ ] `just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0654-editor-module-compare-details.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
