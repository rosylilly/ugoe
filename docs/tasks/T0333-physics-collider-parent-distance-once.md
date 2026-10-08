---
id: T0333
title: "性能: 物理の衝突の判定で、掃引の間は変わらない親と衝突体の距離を 1 度だけ求める"
status: todo
milestone: M7
crates: [ugoe-physics, docs]
depends_on: []
assignee:
---

# T0333: 性能: 物理の衝突の判定で、掃引の間は変わらない親と衝突体の距離を 1 度だけ求める

## 目的

T0193 のプロファイルで、物理そのもの (`update_instance` の 1 段目の評価を除いた部分) の時間の大半は衝突の判定 (`collider.rs`) の
`WorldCollider::blocks` と掃引の算術だった。`constrain_segment` は区間を親のまわりに回しながら標本と二分法で `blocked` を何度も呼ぶが、
そのたびに `near()` の `out_of_reach` と `blocks` の 2 つ目の条件で、親と衝突体の距離 (`point_segment_distance(parent, a, b)`。平方根を含む) を
求め直している。親 (`parent`) と衝突体は掃引の間変わらないので、`constrain_segment` の始めに衝突体ごとに 1 度だけ求めれば、
結果のビット列を変えずに判定 1 回あたりの距離の計算が減る。ネイティブと wasm の両方で物理が 1〜2 割速くなる見込み (T0193 のメモ)。急がない。

## 受け入れ条件

- [ ] `constrain_segment` の始めに、衝突体ごとの親からの距離 (と届く範囲にあるか) を 1 度だけ求め、`blocks`・`out_of_reach` はそれを使う。
      `constrain_segment` の呼び出しのたびのヒープの確保を増やさない (呼び出し側が持つ作業用のバッファを渡すか、衝突体の数が少ないことを前提に
      確保しない形にする。どちらにしたかと理由をコメントに書く)
- [ ] 結果のビット列が変わらない: 物理の golden (`golden_tests.rs`) と `ugoe-physics` の既存のテストがそのまま通る。加えて、衝突体のあるフィクスチャを
      数百フレーム進めた粒子の位置が、直す前と後で完全に一致する (`to_bits` で比べる) テストを足した
- [ ] `physics_timing` (と `puppet_timing`) で前と後を測り、衝突体のある設定で速くなったか (変わらなかったか) をメモに書いた。
      目に見えて速くなれば docs/testing.md#1-フレームの評価 の表を直す (wasm は bindings/web/examples/timing の `measure.mjs` で測る)
- [ ] `just wasm-check` が通る
- [ ] `just ci` が通る

## 参考

- T0193 のメモ (結果・測り方・起票の候補)、docs/testing.md#1-フレームの評価、docs/spec/physics.md (衝突の判定)
- crates/ugoe-physics/src/collider.rs (`constrain_segment`・`WorldCollider::blocks`・`out_of_reach`・`point_segment_distance`)、
  crates/ugoe-physics/examples/physics_timing.rs、crates/ugoe/examples/puppet_timing.rs

## メモ

- T0080 で起票した (元の候補: T0193 (物理の衝突の判定で、掃引の間は変わらない親の距離を 1 度だけ求める。結果のビット列は変わらず 1〜2 割速くなる見込み))。
- M7 の後続 (docs/roadmap.md の M7 の性能の行)。M7 の完了の条件には含めない。
- 結果のビット列を変える変更 (判定の式を変える・近似を使う) は仕様の判断になる。その場合は案をメモに書いて止まる。
- 手元の計測は、ほかのワーカーのビルドが並行していると揺れる。load average が下がってから測り、測った時の条件をメモに書く (T0193 と同じ)。
