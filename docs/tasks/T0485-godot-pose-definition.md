---
id: T0485
title: "Godot: UgoePuppet2D にポーズの定義 (グループごとの要素のパーツ) を読む関数を足す"
status: todo
milestone: M9
crates: [bindings/godot, docs]
depends_on: []
assignee:
---

# T0485: Godot: UgoePuppet2D にポーズの定義 (グループごとの要素のパーツ) を読む関数を足す

## 目的

T0482 で Godot の `UgoePuppet2D` に表示中の要素を読む `get_visible_pose_parts` を足した。Unity は T0483 でポーズの定義 (グループごとの要素のパーツ) も
読めるが、Godot には無く、GDScript でポーズを選ぶ欄を作るにはパーツの名前を外で知っている必要がある (T0482 の候補)。

## 受け入れ条件

- [ ] `UgoePuppet2D` にポーズの定義を読む関数 (例: `get_pose_groups()` が要素のパーツの配列の配列を返す。形と、リンクのパーツを含めるかをメモに書く) がある。
      Unity (T0483) と同じ情報にそろえる
- [ ] bindings/godot のテスト (README にある検査) で、見本のモデルの定義と `show_pose` との組み合わせを確かめる
- [ ] bindings/godot/README.md (関数の表) を直した
- [ ] bindings/godot の README にある検査と `just ci` が通る

## 参考

- T0482 のメモ (起票の候補)、T0483 (C ABI・Unity のポーズの定義)
- bindings/godot/src、bindings/godot/README.md

## メモ

- T0041 で起票した (元の候補: T0482 (ポーズの定義を読む口))。同じ候補の `show_pose_immediately` は、T0329 で「C ABI・Unity・Godot に足すかは要るホストが出てから決める」
  としたので含めない。
