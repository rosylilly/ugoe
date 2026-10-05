---
id: T0478
title: "wasm・C ABI・@ugoe/runtime に表示中のポーズの要素を読む口を足す"
status: done
milestone: M9
crates: [ugoe-wasm, ugoe-ffi, bindings/web, bindings/unity, docs]
depends_on: []
assignee: claude-worker-T0478
---

# T0478: wasm・C ABI・@ugoe/runtime に表示中のポーズの要素を読む口を足す

## 目的

モーションがパーツの不透明度のカーブでポーズの要素を要求すると、描く姿はモーションに従うが、ホストは今どの要素が表示されているかを
読めない (`show_pose` で書く口と `pose_json` の定義だけ)。Rust の SDK には `Puppet::pose()` の `PosePlayer::visible_parts` がある。
ビューア・エディタのプレビューのポーズの欄をモーションに追従させる (T0736) ために、ランタイムの各入口に読む口を足す (T0735 の候補)。

## 受け入れ条件

- [x] wasm に表示中の要素を読む口 (例: `pose_visible_json`。グループごとの表示中のパーツ。モジュールのグループを含む) を足し、テストで `show_pose` とモーションによる切り替えの両方を確かめる
- [x] C ABI に同じ口を足し (ヘッダを作り直す。足すだけなので ABI の版の扱いを crates/ugoe-ffi/README.md の規則で決める)、テストがある。Unity の宣言も足す (`just refs` の過不足の検査)
- [x] `@ugoe/runtime` の型 (`WebPuppet` の宣言)・README を直した。Godot に足すかはメモに書く (足さないなら起票の候補)
- [x] docs/spec/motion.md (または対応するランタイムの文書) と英語の spec の該当の節に、読む口があることを書く必要があるか確かめ、要るなら直した
- [x] `just wasm-check`・`just ffi-check`・`just unity-check`・`just ci` が通る

## 参考

- T0735 のメモ (やらなかったこと・起票の候補)
- crates/ugoe-motion/src/pose.rs (`visible_parts`)、crates/ugoe-wasm/src/animation.rs (`show_pose`・`pose_json`)、crates/ugoe-ffi

## メモ

- T0038 で起票した (元の候補: T0735 (表示中の要素を読む口))。ビューアとエディタの欄の追従は T0736。

### やったこと

- wasm: `pose_visible_json()` (`string | null` の配列。`pose_json()` の `groups` と同じ並びでグループごとに 1 つ、要素の無いグループは `null`)。
  中身は `crate::animation::pose_visible_parts` (`PosePlayer::visible_parts` は要素の無いグループを飛ばすので、グループと並びを合わせる)。
  テストで `show_pose` (すぐ入る)・モーションのパーツの不透明度のカーブの要求 (次の `update` の後)・要素の無いグループ・付けたモジュールのグループを確かめた。
- C ABI: `ugoe_puppet_pose_group_count` と `ugoe_puppet_pose_visible_part(puppet, group, &part)` (表示中の要素のパーツの番号。
  `ugoe_puppet_part_id` で ID にする)。範囲の外は `UGOE_ERROR_OUT_OF_RANGE`、要素の無いグループは `UGOE_ERROR_NOT_FOUND`。
  C ABI にはポーズの定義 (`pose_json`) が無いので JSON ではなく番号の口にした。関数を足しただけなので ABI の版は上げない (README の規則)。
  ヘッダを作り直し、C の見本・README・`tests/host.rs` (バンドルのモーションでの切り替え・モジュール) に足した。
- Unity: `Native` の宣言と `Puppet.GetVisiblePoseParts()` (ID の配列、要素の無いグループは null)、`HostApiTests` に `ShowPose` との組のテスト。
- `@ugoe/runtime`: スタブの `WebPuppet.pose_visible_json`、`visiblePoseParts(puppet)`、問い合わせのキーの突き合わせ (`QueryKeyCheck` が
  `pose_json` のグループの数と合うかも見る。適合テストの `queries` が本物の wasm で通る)、README。
- 仕様: docs/spec/motion.md#ポーズ と spec/motion.md#pose に、ホストが表示中の要素を読めること (SHOULD) と、いつ変わるかを書いた。
- 検査: `just wasm-check`・`just wasm-types` (スタブと、`just wasm` の生成物の両方)・`just ffi-check`・`just refs`・`just conformance`・`just unity-check`・`just ci`。

### やらなかったこと

- Godot (bindings/godot。`show_pose` はある) には足していない (`crates:` の外)。
- packages/player・ビューア・エディタの欄の追従は T0736。

### 起票の候補

- Godot の `UgoePuppet` に表示中のポーズの要素を読む口 (`visible_pose_parts()` など。`show_pose` の隣)。
- C ABI にポーズの定義 (グループの要素のパーツ) を読む口が無い (wasm の `pose_json` に当たるもの)。ホストが C ABI だけで
  ポーズの欄を作るなら要る (`ugoe_puppet_pose_entry_count`・`ugoe_puppet_pose_entry_part` など)。
