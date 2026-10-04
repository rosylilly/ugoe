---
id: T0449
title: "Godot: SDK (ugoe::Puppet) にあって UgoePuppet2D に無い機能を足す"
status: todo
milestone: M9
crates: [bindings/godot, docs]
depends_on: [T0427]
assignee:
---

# T0449: Godot: SDK (ugoe::Puppet) にあって UgoePuppet2D に無い機能を足す

## 目的

T0427 のレビューで、C ABI (`ugoe-ffi`) と比べて `UgoePuppet2D` に次の機能が無いことが分かった。Godot のノードの公開 API を
増やす変更なので、レビューのタスクでは直さずに起票した。どれを出すかを決め、出すものは README の表と smoke に足す。

- 読み込みの警告を取り出す API (`ugoe_puppet_load_warning_*` / wasm の `load_warnings_json`。T0427 で Godot の警告として出すようにしたが、スクリプトからは読めない)
- 受け手 (`Puppet::follow`・`set_base_values`。指揮役から届いた値で動かす)
- トラッキング中のモーションの優先 (`set_motion_over_tracking`・`set_tracking_kept`。T0722)
- 標準の当たり判定 (`standard_hit_area`・`hit_test_standard`)
- 表情を止める (`stop_expressions`)、パーツの不透明度の読み取り (`part_opacity`・`part_evaluated_opacity`)、ポーズ (`show_pose`)
- 目パチ・呼吸・リップシンクの設定と物理の設定の差し替え (`set_effects_json`・`set_physics_json`・`set_base_physics_json`)
- モジュールのスロットの一覧 (`slot_count`・`slot_id`)、トラッキングの対象のパラメータ (`tracking_targets`)

## 受け入れ条件

- [ ] 上の機能ごとに、Godot に出すか出さないか (理由) を README の「制限」かこのメモに書く
- [ ] 出すものを `#[func]` にし、有限でない値の扱いを C ABI と揃え (T0427)、README の表と `example/tests/smoke.gd` に足す
- [ ] `just ci` と `node bindings/godot/build.mjs --check` が通る

## 参考

- crates/ugoe/src/lib.rs、crates/ugoe-ffi/src/puppet.rs、bindings/godot/README.md

## メモ

