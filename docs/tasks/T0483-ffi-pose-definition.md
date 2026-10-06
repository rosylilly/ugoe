---
id: T0483
title: "C ABI・Unity: ポーズの定義 (グループごとの要素のパーツ) を読む口"
status: done
milestone: M9
crates: [ugoe-ffi, bindings/unity, docs]
depends_on: []
assignee: claude-worker-T0483
---

# T0483: C ABI・Unity: ポーズの定義 (グループごとの要素のパーツ) を読む口

## 目的

T0478 で C ABI に表示中の要素を読む口 (`ugoe_puppet_pose_group_count`・`ugoe_puppet_pose_visible_part`) が入ったが、ポーズの定義
(各グループにどの要素があるか。wasm の `pose_json` に当たるもの) を読む口が無い (T0478 の候補)。C ABI だけのホスト (Unity など) が
ポーズを選ぶ欄を作れるように足す。

## 受け入れ条件

- [x] C ABI にグループの要素の数と要素のパーツの番号を読む関数 (`ugoe_puppet_pose_entry_count`・`ugoe_puppet_pose_entry_part` など。範囲の外は
      `UGOE_ERROR_OUT_OF_RANGE`) がある。付けたモジュールのグループも含む。`tests/host.rs` で確かめる。関数を足すだけなら ABI の版は上げない (README の規則)
- [x] ヘッダを作り直し (`just refs` が通る)、C の見本・crates/ugoe-ffi/README.md に足した
- [x] Unity の `Native` の宣言と `Puppet` の読む関数 (グループごとの要素の ID の配列など) と、`HostApiTests` のテストがある
- [x] `just ffi-check`・`just conformance`・`just unity-check`・`just ci` が通る

## 参考

- T0478 のメモ (C ABI の口と、JSON でなく番号にした理由)
- crates/ugoe-ffi、bindings/unity、docs/spec/motion.md#ポーズ

## メモ

- T0040 で起票した (元の候補: T0478 (C ABI のポーズの定義))。
- やったこと: C ABI に `ugoe_puppet_pose_entry_count(puppet, group, &count)`・`ugoe_puppet_pose_entry_part(puppet, group, entry, &part)` を足した
  (crates/ugoe-ffi/src/part.rs)。グループの並びは `ugoe_puppet_pose_visible_part` と同じ合成したモデルのポーズ (付けたモジュールのグループを含む)、
  要素の並びはモデルの `entries` の並びで、`links` のパーツは返さない。グループ・要素の番号が範囲の外なら `UGOE_ERROR_OUT_OF_RANGE`、要素の無い
  グループの要素の数は 0。関数を足しただけなので ABI の版は上げていない。`tests/host.rs` に定義の並び・範囲の外・NULL・モジュールの付け外しの
  テストを足し、ヘッダを作り直し、C の見本 (表示中の要素が定義の要素にあることを確かめる)・crates/ugoe-ffi/README.md・crates/ugoe-wasm/README.md・
  docs/spec/motion.md#ポーズ に書いた。
- Unity: `Native` に 2 つの宣言、`Puppet.GetPoseEntryParts()` (グループごとの要素のパーツの ID の配列 `string[][]`)、`HostApiTests.PoseEntryPartsListTheDefinition`
  (spec/examples/pose-select)、bindings/unity/README.md。`UgoePuppet` (MonoBehaviour) には足していない (`puppet.Puppet` から呼べる)。実際の Unity では
  確かめていない (`just unity-check` の .NET と Unity のスタブでのコンパイルとテストだけ)。
- 検査: `just ffi-check`・`just refs`・`just unity-check` (108 合格)・`just conformance`・`just ci` が通った。
- 起票の候補: リンクのパーツ (`links`) を C ABI で読む口 (`ugoe_puppet_pose_entry_link_count`・`..._link`)。ポーズを選ぶ欄には要らないが、wasm の
  `pose_json` と同じ情報にするなら要る。
