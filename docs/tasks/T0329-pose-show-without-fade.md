---
id: T0329
title: "モーション: ポーズの要素をフェードせずに表示する口 (PosePlayer と wasm)"
status: done
milestone: M9
crates: [ugoe-motion, ugoe-wasm, docs]
depends_on: []
assignee: claude-worker-T0329
---

# T0329: モーション: ポーズの要素をフェードせずに表示する口 (PosePlayer と wasm)

## 目的

`PosePlayer::show` (wasm の `show_pose`) は、選んだ要素へ今の要素からフェードで切り替える。スタジオでモデルを読み直したシーンでは、
覚えた要素を出すときにも先頭の要素からフェードして見えてしまう (T0842 のやらなかったこと・候補)。読み込みの直後など、切り替えの途中を見せたくないときのために、
フェードを飛ばしてすぐに表示する口を足す。スタジオでの使い方は T0844。

## 受け入れ条件

- [x] `ugoe-motion` の `PosePlayer` に、選んだ要素をフェードせずに表示する関数がある (同じグループのほかの要素の不透明度もすぐに 0。
      次の `update` でモーションのカーブの要求があればそれに従う)。単体テストで、呼んだ直後の `opacities` と、`show` との違いを確かめる
- [x] wasm に同じ口がある (`show_pose` に引数を足すか別の関数か。形と理由をメモに書く)。`@ugoe/runtime` のスタブと型 (`just wasm-types`) を合わせた
- [x] docs/spec/motion.md#ポーズ と spec/motion.md#pose に、ホストがフェードせずに表示させられることを書いた (公開仕様の書き方は SHOULD / MAY を選んでメモに理由)
- [x] `just wasm-check` と `just ci` が通る

## 参考

- T0842 のメモ (やらなかったこと・起票の候補)
- crates/ugoe-motion/src/pose.rs、crates/ugoe-wasm/src/animation.rs (`show_pose`)
- T0478 (表示中の要素を読む口。wasm・C ABI・Unity・`@ugoe/runtime` の足し方の前例)

## メモ

- T0040 で起票した (元の候補: T0842 (フェードを飛ばす口))。C ABI・Unity・Godot に足すかは、要るホストが出てから決める (この候補はスタジオのためのもの)。

- **やったこと (T0329):**
  - `PosePlayer::show_immediately(part)` を足した。要素の選び方は `show` と同じ (リンクのパーツ・要素でないパーツは何も変えず `false`)
    で、そのグループの不透明度を選んだ要素 1・ほかの要素 0 にすぐに変える。次の `update` は今までどおりで、モーションの要求が
    あればそこからフェードする。単体テストで、呼んだ直後の `opacities` (`show` では変わらないこと)・フェードの途中から呼んだとき・
    その後の要求からのフェードを確かめた。
  - wasm に `show_pose_immediately(part): boolean` を足した (`animation::show_pose_immediately` とテスト・`WebPuppet`・
    `bindings/web/stub/ugoe_wasm.d.ts`・`crates/ugoe-wasm/README.md`)。
  - **口の形:** `show_pose` に引数を足さず別の関数にした。`show_pose` の型と意味を変えない (既存の呼び出しとスタブ・
    `@ugoe/player` の interface がそのまま) ことと、JS で引数を省いたときにどちらの意味になるかを利用者に気にさせないため。
    Rust の `PosePlayer::show`・`show_immediately` とも 1 対 1 になる。
  - docs/spec/motion.md#ポーズ と spec/motion.md#pose (「Showing an entry without a fade」) に書いた。**MAY にした理由:**
    ホストが要素を選ぶこと自体が MAY で、フェードを飛ばす口はその上の便利な口であり、C ABI・Unity・Godot には (要るホストが
    出るまで) 足さない方針なので、SHOULD にすると自前のランタイムの口が推奨を満たさなくなるため。
  - `just wasm-check`・`just wasm-types` (`just wasm` の生成物と比べて一致)・`just conformance`・`just ci` が通った。
- **やらなかったこと:** C ABI・Unity・Godot・`@ugoe/player` (`Player`) の口、`spec/examples` のタイムラインの操作
  (`show_pose` に並ぶ操作) は足していない。スタジオでの使い方は T0844。
- **起票の候補:**
  - `@ugoe/player` の `Player` と `packages/player/src/wasm.ts` の interface に `show_pose_immediately` を通す (T0844 で要るなら
    そこで)。
  - `spec/examples` のタイムラインに `{ "show_pose_immediately": <part> }` の操作を足し、spec-check と `spec_timelines.rs` で確かめる。
