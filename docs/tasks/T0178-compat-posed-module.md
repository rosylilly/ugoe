---
id: T0178
title: "互換性テスト: ポーズを持つモジュール (jacket_module) の .ugoemod を assets/compat/dev に足す"
status: done
milestone: M9
crates: [ugoe-format, assets/compat, docs]
depends_on: []
assignee: claude-worker-T0178
---

# T0178: 互換性テスト: ポーズを持つモジュール (jacket_module) の .ugoemod を assets/compat/dev に足す

## 目的

互換性テストの資料 (`assets/compat/dev/`) のモジュール (`long-hair.ugoemod`・`dress.ugoemod`) はポーズのグループを持たない。T0850 で
ポーズを持つモジュールのフィクスチャ `ugoe_model::fixtures::jacket_module` が入ったので、これを資料に足し、モジュールのポーズの読み書きを
互換性テストで確かめる (T0850 の候補)。

## 受け入れ条件

- [x] `crates/ugoe-format/tests/compat.rs` の `dev_corpus` に `jacket_module` から作る `.ugoemod` (例 `jacket.ugoemod`) を足し、`assets/compat/dev/` に置いた (`UGOE_UPDATE_FIXTURES=1` で作る)
- [x] 読んだモジュールのポーズのグループ (要素とパーツ) が書き出す前と同じであることをテストで確かめる。`dress-up-base.ugoe` に取り付けられることも確かめる
- [x] ポーズの追従 (`ModulePose`) を持たせるかを決めた (`jacket_module` が持たなければ、持つモジュールを資料にだけ足すかをメモに書いた)
- [x] docs/testing.md#互換性テスト の資料の一覧 (あれば) と assets/compat/README.md を直した
- [ ] `just crate ugoe-format` と `just ci` が通る (`just crate ugoe-format` は通る。`just ci` は無関係のスタジオのテスト 1 件で失敗。メモ)

## 参考

- T0850 のメモ (やらなかったこと・起票の候補)
- crates/ugoe-format/tests/compat.rs、assets/compat/README.md、docs/testing.md#互換性テスト

## メモ

- T0043 で起票した (元の候補: T0850 (`assets/compat/dev/` のポーズを持つモジュール))。

### やったこと

- **資料:** `dev_corpus` に `jacket.ugoemod` (`fixtures::jacket_module`、圧縮なし、1,236 バイト) を足し、`UGOE_UPDATE_FIXTURES=1` で
  `assets/compat/dev/jacket.ugoemod` を作った。既存の資料は変わっていない。`corpus_is_up_to_date`・`dev_corpus_roundtrips_byte_for_byte`・
  `every_release_loads_without_warnings` (本体 `dress-up-base.ugoe` に取り付けて評価) が新しいファイルにも効く。
- **テスト:** `module_pose_groups_survive_and_compose`。読んだモジュールの `pose` と `parts` がフィクスチャと等しいこと、本体に取り付けると
  ポーズのグループが本体のグループの後ろに 1 つ加わり、要素が `Jacket/Closed`・`Jacket/Open` (追従なし) で、そのパーツが合成したモデルにあること、
  評価できることを確かめる。
- `dev/` を走査するほかの検査も新しいファイルを読む: `tools/spec-check` (`dev/jacket.ugoemod loads without warnings and evaluates` が通る)、
  `ugoe-cli` の `ugoe build` の往復のテスト。
- 文書: docs/testing.md#互換性テスト (資料の一覧は無かったので、`dev/` のモジュールの中身と新しいテストを 1 項目で書いた)、assets/compat/README.md。
- `just wasm-check` が通った。

### ポーズの追従 (`ModulePose` の `links`) の判断

- `jacket_module` は追従を持たない。追従はスロットの `pose_parts` にある本体のポーズの要素にしか付けられないが、資料の本体
  `dress-up-base.ugoe` (`fixtures::dress_up_base`) はポーズのグループも `pose_parts` も持たない。`every_release_loads_without_warnings` は
  どのモジュールもこの本体に付けるので、追従を持つモジュールを資料にだけ足すと合成が `PoseNotExposed` で失敗する。
- 本体にポーズを足すと `dress-up-base.ugoe` を使うほかの検査 (Godot・Unity・CLI・spec-check・FFI) にも響くので、このタスクでは足さない (下の起票の候補)。

### just ci

- `just ci` は 3 回流して、1 回目は通り、2・3 回目は `ugoe-studio` の `api::tests::turning_the_api_on_saves_the_choice_and_listens`
  (`apps/studio/src-tauri/src/api/tests.rs:1759` の `assert!(reloaded.api_view().listening)`) で失敗した。このタスクの変更 (互換性テストの資料と
  `ugoe-format` のテスト) はスタジオに関わらず、`cargo test -p ugoe-studio --lib` を単独で 3 回流すとすべて通る。ほかのワーカーが並行して
  負荷の高いビルドとテストを流している中での、待ち受けの開始の時間に依る失敗と見ている。

### 起票の候補

- ポーズのグループと `pose_parts` を持つ本体 (例: `dress_up_base` の `Outfit` に腕の差分 A / B を出す版) と、それに追従 (`ModulePose.links`) する
  モジュールのフィクスチャを足し、互換性テストの資料にも入れる。
- `turning_the_api_on_saves_the_choice_and_listens` が負荷の高いときに失敗する (待ち受けの開始を待たずに `listening` を見ている疑い)。
