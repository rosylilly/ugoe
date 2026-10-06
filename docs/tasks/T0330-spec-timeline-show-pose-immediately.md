---
id: T0330
title: "spec/examples: タイムラインに show_pose_immediately の操作を足して確かめる"
status: done
milestone: M9
crates: [spec, tools/spec-check, ugoe, docs]
depends_on: []
assignee: claude-worker-T0330
---

# T0330: spec/examples: タイムラインに show_pose_immediately の操作を足して確かめる

## 目的

T0329 で、ポーズの要素をフェードせずに表示する口 (`PosePlayer::show_immediately`・wasm の `show_pose_immediately`) を足し、
spec/motion.md#pose に書いた。公開仕様の例のタイムラインには `show_pose` の操作しか無く、フェードを飛ばす表示を例で確かめていない (T0329 の候補)。

## 受け入れ条件

- [x] タイムライン (`timeline.json`) に `{ "show_pose_immediately": <part> }` の操作がある。spec/examples/README.md の操作の表に書いた
- [x] `pose-select` (か新しい例) のタイムラインで、呼んだ直後のフレームで不透明度が切り替わり、その後のモーションの要求からフェードすることを、
      期待する値で確かめる (`show_pose` との違いが分かる組)
- [x] spec-check (`tools/spec-check/src/timeline.ts`) と `crates/ugoe/tests/spec_timelines.rs` の両方が新しい操作を読み、例を満たす
- [x] spec/motion.md#pose から例を参照した
- [x] `just ci` が通る

## 参考

- T0329 のメモ (やったこと・起票の候補)
- spec/motion.md#pose、spec/examples/pose-select、tools/spec-check/src/timeline.ts、crates/ugoe/tests/spec_timelines.rs

## メモ

- T0041 で起票した (元の候補: T0329 (`spec/examples` のタイムライン))。C ABI には `show_pose_immediately` が無い
  (T0329 で「要るホストが出てから決める」)。タイムラインを読むほかの検査 (C ABI・適合テスト) があれば、この操作を飛ばすか確かめる。

### やったこと (claude-worker-T0330)

- タイムラインの操作 `{ "show_pose_immediately": <part> }` を足し、spec/examples/README.md の操作の表に書いた
  (`show_pose_immediately` を持たないランタイムはその操作のあるタイムラインを飛ばしてよい (MAY)、と添えた。仕様の段落が MAY のため)。
- `pose-select/timeline.json` にフレーム 14〜20 を足した: リンクのパーツ・ポーズに無いパーツは何も変えない / ArmB を呼んだフレームで
  ArmA 0・ArmB 1 (フレーム 1 の `show_pose` の 0.75 / 0.25 と対になる) / ポーズの段を止めて ArmA を呼ぶと何も書かず、段を戻すと
  ArmA 1 から続く / ArmB を呼んだ後に `back` (ArmA を要求) を始めると ArmA が 0.25・0.5・0.75 とそこからフェードする。
  summary と README の例の一覧も直した。
- spec-check (`PoseState.showImmediately`・`Runtime.showPoseImmediately`・timeline.ts) を spec/motion.md#pose の段落だけから書き、
  `crates/ugoe/tests/spec_timelines.rs` は `PosePlayer::show_immediately` を呼ぶ。期待値を一時的に崩すと両方が失敗することを確かめた。
- spec/motion.md#pose の「Showing an entry without a fade」の段落から pose-select を参照した。docs/testing.md の操作の列挙も直した。
- タイムラインを読むほかの検査: C ABI・適合テスト (tests/conformance) はタイムラインを読まない。Unity の `HostApiTests` は
  pose-select のフレーム 0〜5 だけを手で写しているので、足したフレームの影響はない。
- `just spec`・`just ci` が通った。

### 起票の候補

- なし (C ABI に `show_pose_immediately` を足すなら、Unity の `HostApiTests` に pose-select のフレーム 14〜20 を写す)。
