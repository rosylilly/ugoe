---
id: T0329
title: "モーション: ポーズの要素をフェードせずに表示する口 (PosePlayer と wasm)"
status: todo
milestone: M9
crates: [ugoe-motion, ugoe-wasm, docs]
depends_on: []
assignee:
---

# T0329: モーション: ポーズの要素をフェードせずに表示する口 (PosePlayer と wasm)

## 目的

`PosePlayer::show` (wasm の `show_pose`) は、選んだ要素へ今の要素からフェードで切り替える。スタジオでモデルを読み直したシーンでは、
覚えた要素を出すときにも先頭の要素からフェードして見えてしまう (T0842 のやらなかったこと・候補)。読み込みの直後など、切り替えの途中を見せたくないときのために、
フェードを飛ばしてすぐに表示する口を足す。スタジオでの使い方は T0844。

## 受け入れ条件

- [ ] `ugoe-motion` の `PosePlayer` に、選んだ要素をフェードせずに表示する関数がある (同じグループのほかの要素の不透明度もすぐに 0。
      次の `update` でモーションのカーブの要求があればそれに従う)。単体テストで、呼んだ直後の `opacities` と、`show` との違いを確かめる
- [ ] wasm に同じ口がある (`show_pose` に引数を足すか別の関数か。形と理由をメモに書く)。`@ugoe/runtime` のスタブと型 (`just wasm-types`) を合わせた
- [ ] docs/spec/motion.md#ポーズ と spec/motion.md#pose に、ホストがフェードせずに表示させられることを書いた (公開仕様の書き方は SHOULD / MAY を選んでメモに理由)
- [ ] `just wasm-check` と `just ci` が通る

## 参考

- T0842 のメモ (やらなかったこと・起票の候補)
- crates/ugoe-motion/src/pose.rs、crates/ugoe-wasm/src/animation.rs (`show_pose`)
- T0478 (表示中の要素を読む口。wasm・C ABI・Unity・`@ugoe/runtime` の足し方の前例)

## メモ

- T0040 で起票した (元の候補: T0842 (フェードを飛ばす口))。C ABI・Unity・Godot に足すかは、要るホストが出てから決める (この候補はスタジオのためのもの)。
