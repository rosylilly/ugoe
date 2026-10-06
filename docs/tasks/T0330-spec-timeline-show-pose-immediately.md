---
id: T0330
title: "spec/examples: タイムラインに show_pose_immediately の操作を足して確かめる"
status: in-progress
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

- [ ] タイムライン (`timeline.json`) に `{ "show_pose_immediately": <part> }` の操作がある。spec/examples/README.md の操作の表に書いた
- [ ] `pose-select` (か新しい例) のタイムラインで、呼んだ直後のフレームで不透明度が切り替わり、その後のモーションの要求からフェードすることを、
      期待する値で確かめる (`show_pose` との違いが分かる組)
- [ ] spec-check (`tools/spec-check/src/timeline.ts`) と `crates/ugoe/tests/spec_timelines.rs` の両方が新しい操作を読み、例を満たす
- [ ] spec/motion.md#pose から例を参照した
- [ ] `just ci` が通る

## 参考

- T0329 のメモ (やったこと・起票の候補)
- spec/motion.md#pose、spec/examples/pose-select、tools/spec-check/src/timeline.ts、crates/ugoe/tests/spec_timelines.rs

## メモ

- T0041 で起票した (元の候補: T0329 (`spec/examples` のタイムライン))。C ABI には `show_pose_immediately` が無い
  (T0329 で「要るホストが出てから決める」)。タイムラインを読むほかの検査 (C ABI・適合テスト) があれば、この操作を飛ばすか確かめる。
