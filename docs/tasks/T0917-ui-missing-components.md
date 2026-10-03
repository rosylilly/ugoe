---
id: T0917
title: UI 品質: 足りない部品と口を足す (TextField・tooltip・Menu の説明の行)
status: todo
milestone: M10
crates: [packages, docs, apps/editor, apps/viewer, apps/studio]
depends_on: [T0915, T0912]
assignee:
---

# T0917: UI 品質: 足りない部品と口を足す (TextField・tooltip・Menu の説明の行)

## 目的

T0907 の後続の候補として残った部品の不足を埋め、アプリに残る自前の部品と `title` を減らす。
基準は docs/ui.md (T0915 で足す「UI の品質の基準」を含む)。直せないもの・大きいものは後続のタスクにする (milestone は M10)。

## 受け入れ条件

- [ ] `@ugoe/ui` に TextField (ラベル・補足・エラー・無効) を足し、Vitest でキーボード操作と aria の属性を検査した
- [ ] `Slider`・`Select`・`NumberField` に tooltip の口を足した (トラッキングの平滑化の説明などを戻せるか判断する)
- [ ] `Menu` の項目に説明の行を出せるようにした
- [ ] 3 つのアプリで、標準の `<input type=text>` と自前の部品を TextField などに置き換えた (置き換えないものは理由を書く)
- [ ] 残った `title` (行の中の説明: 物理のリグの欄・カーブのフェードの欄など) を、見える補足か Tooltip にした
- [ ] docs/ui.md の部品の一覧を更新した
- [ ] `just ci` が通る

## 参考

- docs/ui.md
- T0907 のメモの「後続の候補」
- docs/roadmap.md の M8 の「起票していない後続の候補」の UI の行

## メモ
