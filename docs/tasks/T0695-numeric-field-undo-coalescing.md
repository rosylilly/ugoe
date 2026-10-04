---
id: T0695
title: 数値の欄の続けての確定で Undo の履歴が積み上がるのをまとめる
status: todo
milestone: M9
crates: [ugoe-editor-core, apps/editor, packages/ui, docs]
depends_on: [T0671]
assignee:
---

# T0695: 数値の欄の続けての確定で Undo の履歴が積み上がるのをまとめる

## 目的

T0671 (パネルのコードレビュー) の所見。`@ugoe/ui` の `NumberField` は、矢印キー・PageUp / PageDown の 1 回ごとに `onchange` で確定する
(docs/ui.md#数値の部品と-onchange)。エディタのパネルは `onchange` ごとにコマンドを 1 つ送るので、物理の係数・グルーの強さ・
キーフォームの描画順・タイムラインの時刻などで矢印キーを押し続けると、1 回ごとに Undo の履歴が 1 つ積まれ、適用と読み直しの IPC も
その回数だけ走る。スライダー (`Slider`) は `oncommit` で離したときだけ送るので、つまみでは起きない。

どこでまとめるかは設計の判断が要る (例: editor-core で、同じ対象・同じ種類の続けてのコマンドを短い時間の内なら 1 つの Undo にまとめる。
または `NumberField` に、キーを離したとき・フォーカスを外したときだけ確定する設定を足す)。

## 受け入れ条件

- [ ] まとめる場所と規則を決め、必要なら ADR に書いた
- [ ] 矢印キーを押し続けたときの Undo の履歴が 1 つ (または決めた規則どおり) になることをテストした
- [ ] docs/ui.md (と docs/editor.md の Undo の節) を更新した
- [ ] `just ci` が通る

## 参考

- docs/tasks/T0671-review-editor-panels.md (所見の表)
- docs/ui.md#数値の部品と-onchange、packages/ui/src/components/NumberField.svelte
- crates/ugoe-editor-core (Undo の履歴)

## メモ
