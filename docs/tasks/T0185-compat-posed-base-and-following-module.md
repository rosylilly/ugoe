---
id: T0185
title: "互換性テスト: ポーズのグループと pose_parts を持つ本体と、それに追従する (ModulePose の links) モジュールの資料を足す"
status: todo
milestone: M9
crates: [ugoe-model, ugoe-format, assets/compat, docs]
depends_on: []
assignee:
---

# T0185: 互換性テスト: ポーズのグループと pose_parts を持つ本体と、それに追従する (ModulePose の links) モジュールの資料を足す

## 目的

T0178 で互換性テストの資料にポーズを持つモジュール (`jacket.ugoemod`) を足したが、本体のポーズに追従する (`ModulePose.links`) モジュールは足せなかった。
追従はスロットの `pose_parts` にある本体のポーズの要素にしか付けられず、資料の本体 `dress-up-base.ugoe` (`fixtures::dress_up_base`) は
ポーズのグループも `pose_parts` も持たないため (`every_release_loads_without_warnings` がどのモジュールもこの本体に付け、`PoseNotExposed` で失敗する)。

## 受け入れ条件

- [ ] ポーズのグループと `pose_parts` を持つ本体のフィクスチャ (例: `Outfit` のスロットに腕の差分 A / B を出す) と、その要素に追従するモジュールの
      フィクスチャが `ugoe_model::fixtures` にある。`dress_up_base` を変えるか、別の本体のフィクスチャを足すかを決め、理由をメモに書いた
      (`dress_up_base` は ugoe-ffi・ugoe-wasm・ugoe-cli・ugoe-editor-core・Godot・Unity などのテストが使う。推奨は別の本体を足す)
- [ ] 両方を `assets/compat/dev/` に入れ (`UGOE_UPDATE_FIXTURES=1`)、`every_release_loads_without_warnings` などの資料の検査が、モジュールを
      合う本体に付けて評価する。既存の資料のバイト列は変わらない
- [ ] 追従するモジュールを読み、本体に付けて、本体のポーズの要素を切り替えるとモジュールのパーツが追従することを確かめるテストがある
- [ ] `dev/` を走査するほかの検査 (`tools/spec-check`・`ugoe-cli` の往復) が新しいファイルで通る。`dress_up_base` を変えたなら、
      Godot・Unity・FFI の検査 (各 README の手順) も通る
- [ ] docs/testing.md#互換性テスト と assets/compat/README.md を直した
- [ ] `just wasm-check` と `just ci` が通る

## 参考

- T0178 のメモ (ポーズの追従の判断・起票の候補)
- crates/ugoe-model/src/fixtures (modules.rs)、crates/ugoe-format/tests/compat.rs、assets/compat/README.md、docs/testing.md#互換性テスト

## メモ

- T0046 で起票した (元の候補: T0178 (ポーズのグループと `pose_parts` を持つ本体と追従するモジュールの資料))。
- T0178 のもう 1 つの候補 (スタジオの `turning_the_api_on_saves_the_choice_and_listens` の揺れ) は T0961 で直ったので含めない。
