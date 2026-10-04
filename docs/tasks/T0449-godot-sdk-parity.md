---
id: T0449
title: "Godot: SDK (ugoe::Puppet) にあって UgoePuppet2D に無い機能を足す"
status: done
milestone: M9
crates: [bindings/godot, docs]
depends_on: [T0427]
assignee: claude-worker-T0449
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

- [x] 上の機能ごとに、Godot に出すか出さないか (理由) を README の「制限」かこのメモに書く
- [x] 出すものを `#[func]` にし、有限でない値の扱いを C ABI と揃え (T0427)、README の表と `example/tests/smoke.gd` に足す
- [x] `just ci` と `node bindings/godot/build.mjs --check` が通る

## 参考

- crates/ugoe/src/lib.rs、crates/ugoe-ffi/src/puppet.rs、bindings/godot/README.md

## メモ

### やったこと

- 列挙の機能は **すべて出した** (出さないものは無し)。`#[func]` は新しいファイル `bindings/godot/src/node/sdk.rs` の
  `#[godot_api(secondary)]` に置いた (トラッキングの優先の 2 つはプロパティなので `src/node.rs`)。

| 機能 | Godot の API |
| --- | --- |
| 読み込みの警告 | `get_load_warnings()` (`{ code, warning, texture/feature/path }` の配列。キーは wasm・C ABI と同じ) |
| 受け手 | `get_parameter_values()`・`set_base_values(values)`・`follow(values, delta, lip_sync_level, lip_sync_strength, part_opacities)` |
| モーションの優先 | プロパティ `motion_over_tracking`・`tracking_kept` (ID の配列。シーンに保存し、モデルを読み直しても保つ) |
| 標準の当たり判定 | `get_standard_hit_area(name)`・`hit_test_standard(name, pos)`・`get_standard_hit_areas_at(pos)` |
| 表情・パーツ・ポーズ | `stop_expressions()`・`get_part_ids()`・`get_part_opacity(id)`・`get_part_evaluated_opacity(id)`・`show_pose(part)` |
| 効果と物理 | `set_effects_json`・`set_physics_json`・`set_base_physics_json`・`get_physics_json`・`get_base_physics_json` (空の文字列で外す) |
| スロット・トラッキングの対象 | `get_slot_ids()`・`get_tracking_targets()` (パラメータの ID) |

- 有限でない値: `follow` は C ABI と同じ (`delta` は有限で負でない、値の NaN・リップシンクとパーツの不透明度の有限でない値を拒む。
  音量が負なら当てない)。`set_base_values` は NaN を拒む。README の「有限でない値」の段落に足した。
- 本体の物理の差し替えでモデルが変わる (スロットの `colliders`) ときのために `LoadedModel::set_base_physics` を足し、描画の情報と
  レンダラを作り直す (テストを追加)。
- 効果の JSON を読むために `serde_json` を直接の依存にした (`ugoe` 経由で既に Cargo.lock にあったもの。MIT/Apache-2.0)。
- smoke (`example/tests/smoke.gd` の `_sdk_features`) に全機能の検査と、拒む値の検査を足した (公開仕様の例 `pose-select`・
  `effects-replace`・`physics-pendulum` と `assets/compat/future` を使う)。
- **範囲外の小さな変更:** 適合テストの実行器 (`example/tests/conformance.gd`) で計画の `follow` を `follow` で進めるようにし、
  `tests/conformance/src/main.rs` の `FOLLOW_BINDINGS` に `godot` を足した (README の記述も)。飛ばしていた 4 件
  (`avatar-follow`・`dress-up-follow`・`lip-sync-follow`・`part-switch-follow`) が C ABI と同じ値で通る (57 / 57)。
  ノードは読み込みの直後に 1 度評価して物理の粒子を置くので、受け手を始めるときは `reset_physics()` を呼ぶ必要があった
  (呼ばないと物理のある 3 件が最大 0.5px ずれる)。README と doc コメントに書いた。

### 出さなかったもの・制限

- 指揮役の側の値 (`values_before_physics`・`lip_sync_applied`・`part_opacities_applied`) は C ABI・wasm にも無いので出していない。
  Godot のノードを指揮役にするには要る (README の「制限」)。
- 着せ替えのモジュールの読み込みの警告はスクリプトから読めない (Godot の警告に出すだけ。`UgoeModule` は GDScript のリソース)。
- `set_effects_json`・`set_physics_json`・`show_pose` の結果はモデルを読み直すとモデルの設定に戻る (README に書いた)。
- 実際のエディタの画面での操作・Windows・macOS は確かめていない (Xvfb の Compatibility とヘッドレスだけ)。

### 検査

- `cargo test` (36 件)・`cargo clippy --all-targets -D warnings`・`cargo fmt --check` (bindings/godot): 通る
- `node bindings/godot/build.mjs --check --conformance`: ライセンス ok、smoke ok、適合テスト 57 / 57 (skip 無し)、ktx2 ok
- `just ci`: 通る (main に rebase した後)

### 起票の候補

- Godot のノードを指揮役にする口 (`get_values_before_physics`・`get_lip_sync_applied`・`get_part_opacities_applied`)。C ABI・wasm にも
  無いので、3 つの組み込み先で揃えて足すとよい。
- Unity の受け手の口 (`follow`) と適合テストの `FOLLOW_BINDINGS` への追加 (今は Unity だけが `*-follow` を飛ばす)。
- 着せ替えのモジュールの読み込みの警告をスクリプトから読む口 (`UgoeModule` か `attach_module` の結果で)。

