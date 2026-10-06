---
id: T0850
title: "ポーズを持つモジュールのフィクスチャを足し、着せ替えの直後にフェードを飛ばすことをスタジオで確かめる"
status: done
milestone: M9
crates: [ugoe-model, apps/studio, docs]
depends_on: []
assignee: claude-worker-T0850
---

# T0850: ポーズを持つモジュールのフィクスチャを足し、着せ替えの直後にフェードを飛ばすことをスタジオで確かめる

## 目的

T0844 で、スタジオは読み直したシーンで選んだポーズの要素をフェードせずに出し、`follow_match.rs` にポーズの切り替えを入れた。
`ugoe_model::fixtures` のモジュール (`long_hair_module`・`dress_module`) はポーズを持たない (`ModulePose::default()`) ので、
モジュールを着せた直後 (モジュールのポーズのグループ) にフェードを飛ばすことは確かめていない (T0844 の候補)。

## 受け入れ条件

- [x] `ugoe_model::fixtures::modules` に、ポーズのグループ (2 つ以上の要素) を持つモジュールのフィクスチャがある (既存のフィクスチャの値は変えない)。
      doc コメントで何を確かめるためのものかを書いた
- [x] `conductor.rs` のテストで、そのモジュールを着せたシーンを読み直した直後に、選んだ要素が不透明度 1・ほかが 0 (フェードの途中でない) ことを確かめる
- [x] `follow_match.rs` で、着せ替えの直後のポーズの切り替えでも指揮役と受け手が一致することを確かめる
- [x] フィクスチャの一覧 (docs/testing.md など、載せている所があれば) を直した
- [x] `just ci` が通る

## 参考

- T0844 のメモ (やったこと・起票の候補)
- crates/ugoe-model/src/fixtures/modules.rs、apps/studio/src-tauri/src (`conductor.rs`・`follow_match.rs`)

## メモ

- T0041 で起票した (元の候補: T0844 (ポーズを持つモジュールのフィクスチャ))。`ugoe-model` は fixtures (テスト用のデータ) だけを足す。

### やったこと

- **フィクスチャ:** `ugoe_model::fixtures::jacket_module` (`fixtures/modules.rs`) を足した。スロット `Outfit` に付けるモジュール `Jacket` で、
  パーツ `Closed`・`Open` (メッシュ 1 枚ずつ) とポーズのグループ 1 つ (`Closed`, `Open`) を持つ。`dress_module` と同じスロットなので一緒には付けられない
  (`long_hair_module` とは一緒に付けられる)。既存のフィクスチャの値は変えていない (`dress_up_base` の doc コメントに名前を足しただけ)。
  `validate/module.rs` の `fixtures_are_valid` に足した。
- **`conductor.rs`:** `a_dressed_scene_shows_the_modules_pose_without_a_fade`。本体を読んだ後にモジュールを着せたシーン (`Jacket/Open`) は
  刻む前も後も `Closed` 0・`Open` 1、同じ組のままの選び直しはフェード、組を替えた (髪を足した) シーンはフェードの途中でもすぐ `Open` 1。
- **`follow_match.rs`:** `pose_switches_right_after_dressing_up_match_the_conductor`。受け手の `receive_scene` を、仮想カメラと同じく
  モジュールの版が変わったら付け直すように広げ (`modules` を受け取る)、着せる・同じ組で選び直す・組を替える、を各フレームで指揮役と比べる
  (許容誤差 1e-3)。着せた直後・組を替えた直後は `Closed` 0・`Open` 1、選び直しで実際にフェードしていることも確かめる。
- 空振りでないことの確認: 指揮役の `present` の条件を `model_revision == revision` (読み直しのときだけ飛ばす) に一時的に替えると、
  2 つの新しいテストがともに失敗した (戻した)。
- 文書: docs/studio.md (一致の検査とフェードの項に新しいテストを書いた)。フィクスチャの一覧を載せた文書は無い (docs/testing.md は方針だけ。
  `ugoe-cli` の `fixture` の一覧はモデルだけでモジュールを載せない)。
- `just ci`・`just wasm-check`・`just vcam-check` が通った。

### やらなかったこと

- 互換性テストの資料 (`ugoe-format` の `tests/compat.rs` の `dev_corpus`) にポーズを持つモジュールを足していない (範囲外)。

### 起票の候補

- `assets/compat/dev/` に、ポーズのグループと追従 (`ModulePose`) を持つモジュール (`jacket_module` など) の `.ugoemod` を足す (今の資料のモジュールはポーズを持たない)。
