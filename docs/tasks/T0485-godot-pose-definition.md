---
id: T0485
title: "Godot: UgoePuppet2D にポーズの定義 (グループごとの要素のパーツ) を読む関数を足す"
status: done
milestone: M9
crates: [bindings/godot, docs]
depends_on: []
assignee: claude-worker-T0485
---

# T0485: Godot: UgoePuppet2D にポーズの定義 (グループごとの要素のパーツ) を読む関数を足す

## 目的

T0482 で Godot の `UgoePuppet2D` に表示中の要素を読む `get_visible_pose_parts` を足した。Unity は T0483 でポーズの定義 (グループごとの要素のパーツ) も
読めるが、Godot には無く、GDScript でポーズを選ぶ欄を作るにはパーツの名前を外で知っている必要がある (T0482 の候補)。

## 受け入れ条件

- [x] `UgoePuppet2D` にポーズの定義を読む関数 (例: `get_pose_groups()` が要素のパーツの配列の配列を返す。形と、リンクのパーツを含めるかをメモに書く) がある。
      Unity (T0483) と同じ情報にそろえる
- [x] bindings/godot のテスト (README にある検査) で、見本のモデルの定義と `show_pose` との組み合わせを確かめる
- [x] bindings/godot/README.md (関数の表) を直した
- [x] bindings/godot の README にある検査と `just ci` が通る

## 参考

- T0482 のメモ (起票の候補)、T0483 (C ABI・Unity のポーズの定義)
- bindings/godot/src、bindings/godot/README.md

## メモ

- T0041 で起票した (元の候補: T0482 (ポーズの定義を読む口))。同じ候補の `show_pose_immediately` は、T0329 で「C ABI・Unity・Godot に足すかは要るホストが出てから決める」
  としたので含めない。
- やったこと: `UgoePuppet2D.get_pose_entry_parts()` (`Array[PackedStringArray]`) を足した (bindings/godot/src/node/sdk.rs)。形はグループごとの
  要素のパーツの ID の配列の配列で、Unity の `Puppet.GetPoseEntryParts()` (`string[][]`)・C ABI の `ugoe_puppet_pose_entry_part` と同じ情報。
  グループの並びは `get_visible_pose_parts` と同じ合成したモデルのポーズ (付けた着せ替えのモジュールのグループも入る)、要素の並びはモデルの
  `entries` の並び (先頭が初めに表示する要素)。**`links` のパーツは含めない** (`show_pose` に渡せるパーツだけ。C ABI・Unity と同じ)。要素の
  無いグループは空の配列、モデルが無ければ空。名前は Unity の `GetPoseEntryParts` に合わせた。
- テスト: 単体テスト `pose_entry_parts_list_the_definition` (リンクを持つ要素・要素の無いグループと `get_visible_pose_parts` の並びの一致・
  定義の要素を `show` できること・付けたモジュールのグループ)。smoke (`example/tests/smoke.gd`) で pose-select の定義 (`["ArmA", "ArmB"]`。
  リンクの SleeveB を含まず `show_pose("SleeveB")` は false) と、dress-up の定義の要素をそれぞれ `show_pose` して表示中の要素になること。
- bindings/godot/README.md の API の表と docs/spec/motion.md#ポーズ の口の一覧に足した (英語の spec/ は口の名前を持たないので変えていない)。
- 検査: `just godot-check` (smoke と適合テスト 69 ケース)・`just conformance` (全バインディング)・`just ci` が通った。
- やらなかったこと: `show_pose_immediately` (上のとおり範囲外)。リンクのパーツを読む口 (T0483 の候補と同じく、ポーズを選ぶ欄には要らない)。
- 起票の候補: なし (リンクのパーツを読む口は T0483 の候補と合わせて、C ABI・Unity・Godot をそろえて足すなら 1 つにまとめる)。
