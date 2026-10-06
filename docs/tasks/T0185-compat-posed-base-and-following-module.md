---
id: T0185
title: "互換性テスト: ポーズのグループと pose_parts を持つ本体と、それに追従する (ModulePose の links) モジュールの資料を足す"
status: done
milestone: M9
crates: [ugoe-model, ugoe-format, assets/compat, docs]
depends_on: []
assignee: claude-worker-T0185
---

# T0185: 互換性テスト: ポーズのグループと pose_parts を持つ本体と、それに追従する (ModulePose の links) モジュールの資料を足す

## 目的

T0178 で互換性テストの資料にポーズを持つモジュール (`jacket.ugoemod`) を足したが、本体のポーズに追従する (`ModulePose.links`) モジュールは足せなかった。
追従はスロットの `pose_parts` にある本体のポーズの要素にしか付けられず、資料の本体 `dress-up-base.ugoe` (`fixtures::dress_up_base`) は
ポーズのグループも `pose_parts` も持たないため (`every_release_loads_without_warnings` がどのモジュールもこの本体に付け、`PoseNotExposed` で失敗する)。

## 受け入れ条件

- [x] ポーズのグループと `pose_parts` を持つ本体のフィクスチャ (例: `Outfit` のスロットに腕の差分 A / B を出す) と、その要素に追従するモジュールの
      フィクスチャが `ugoe_model::fixtures` にある。`dress_up_base` を変えるか、別の本体のフィクスチャを足すかを決め、理由をメモに書いた
      (`dress_up_base` は ugoe-ffi・ugoe-wasm・ugoe-cli・ugoe-editor-core・Godot・Unity などのテストが使う。推奨は別の本体を足す)
- [x] 両方を `assets/compat/dev/` に入れ (`UGOE_UPDATE_FIXTURES=1`)、`every_release_loads_without_warnings` などの資料の検査が、モジュールを
      合う本体に付けて評価する。既存の資料のバイト列は変わらない
- [x] 追従するモジュールを読み、本体に付けて、本体のポーズの要素を切り替えるとモジュールのパーツが追従することを確かめるテストがある
- [x] `dev/` を走査するほかの検査 (`tools/spec-check`・`ugoe-cli` の往復) が新しいファイルで通る。`dress_up_base` を変えたなら、
      Godot・Unity・FFI の検査 (各 README の手順) も通る
- [x] docs/testing.md#互換性テスト と assets/compat/README.md を直した
- [x] `just wasm-check` と `just ci` が通る

## 参考

- T0178 のメモ (ポーズの追従の判断・起票の候補)
- crates/ugoe-model/src/fixtures (modules.rs)、crates/ugoe-format/tests/compat.rs、assets/compat/README.md、docs/testing.md#互換性テスト

## メモ

- T0046 で起票した (元の候補: T0178 (ポーズのグループと `pose_parts` を持つ本体と追従するモジュールの資料))。
- T0178 のもう 1 つの候補 (スタジオの `turning_the_api_on_saves_the_choice_and_listens` の揺れ) は T0961 で直ったので含めない。

### やったこと

- **フィクスチャ (別の本体を足した):** `fixtures::posed_dress_up_base` (`dress_up_base` にパーツ `ArmA`・`ArmB` (`Body` の子) とそのメッシュ、
  ポーズのグループ `[ArmA, ArmB]` を加え、スロット `Outfit` の `pose_parts` に出したもの) と、`fixtures::sleeves_module`
  (スロット `Outfit`、パーツ `SleeveA`・`SleeveB` とメッシュ、`pose.links` で `ArmA` に `SleeveA`、`ArmB` に `SleeveB` を追従させる) を
  `fixtures/modules.rs` に足した。`fixtures_are_valid` で両方を検査する。
  - `dress_up_base` を変えなかった理由: ugoe-ffi・ugoe-wasm・ugoe-cli・ugoe-editor-core・Godot・Unity・スタジオなどのテストと
    `dress-up-base.ugoe` (と、それから作る `future/optional-feature.ugoe`) が使っていて、変えると資料のバイト列と各検査に響くため。
    `posed_dress_up_base` は `dress_up_base` を元に作るので、`dress_up_base` が変われば一緒に変わる。
- **資料:** `dev_corpus` に `posed-dress-up-base.ugoe` (3,812 バイト) と `sleeves.ugoemod` (1,268 バイト) を足し、`UGOE_UPDATE_FIXTURES=1` で作った。
  既存の資料のバイト列は変わっていない。`every_release_loads_without_warnings` はモジュールごとに本体を選ぶ (`module_base`:
  `sleeves.ugoemod` は `posed-dress-up-base.ugoe`、ほかは `dress-up-base.ugoe`)。
- **テスト:** `module_pose_links_follow_base_entries` (`crates/ugoe-format/tests/compat.rs`)。読んだ追従がフィクスチャと等しい、本体が
  フィクスチャと等しい、ポーズを出さない `dress-up-base.ugoe` には `PoseNotExposed` で付けられない、`posed-dress-up-base.ugoe` に
  付けると要素の `links` が `Sleeves/SleeveA`・`Sleeves/SleeveB` になる、`PosePlayer` の不透明度を (`Puppet` と同じに) パーツの
  不透明度の上書きとして渡すと、最初は腕 A と袖 A だけが見え、`ArmB` に切り替えると腕 B と袖 B に替わり、フェードの途中でも袖の
  不透明度が腕と同じになることを確かめる。このために `ugoe-format` の dev-dependencies に `ugoe-motion` を足した (dev なので層の検査には響かない)。
- **`crates:` の外の小さな変更:** `tools/spec-check/test/compat.test.ts`。`dev/` のモジュールをすべて `dress-up-base.ugoe` に付けていたので、
  `sleeves.ugoemod` だけ `posed-dress-up-base.ugoe` に付けるようにし、合成で追従が要素の `links` に加わる (ポーズを出さない本体では
  拒まれる) テストを 1 つ足した。spec-check の往復 (`compatSources`) と `ugoe-cli` の `build_round_trips_every_compat_file` は新しいファイルでも通る。
- 文書: docs/testing.md#互換性テスト、assets/compat/README.md。
- `just wasm-check` と `just ci` が通った。

### やらなかったこと

- `dress_up_base` を変えていないので、Godot・Unity・FFI の検査は流していない (それらは `dress-up-base.ugoe` と `future/` しか読まない)。

### 起票の候補

- なし。
