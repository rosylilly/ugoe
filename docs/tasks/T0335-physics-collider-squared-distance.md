---
id: T0335
title: "性能: 物理の衝突の判定で、距離を平方根を取らずに 2 乗で比べるか決める (判断が要る)"
status: todo
milestone: M7
crates: [ugoe-physics, docs, spec]
depends_on: []
assignee:
---

# T0335: 性能: 物理の衝突の判定で、距離を平方根を取らずに 2 乗で比べるか決める (判断が要る)

## 目的

T0333 で、掃引の間は変わらない親と衝突体の距離を 1 度だけ求めるようにした (結果のビット列は変えていない。物理そのものがネイティブで約 1 割、
wasm で約 15% 速くなった)。物理そのものの残りの大半は、掃引と二分法のたびの `WorldCollider::blocks_from` の算術
(`point_segment_distance(q, a, b) < radius + margin` と `segment_segment_distance(parent, q, a, b) < radius`。どちらも平方根を含む) である
(crates/ugoe-physics/src/collider.rs)。距離を 2 乗のまま比べれば (`d2 < (radius + margin)^2` など) 平方根が要らなくなり、
物理そのものがネイティブ・wasm ともさらに 1〜2 割速くなる見込み (T0333 のメモ)。ただし境目の判定の丸めが変わるので、
結果のビット列が変わりうる。急がない。

## 判断が要る

2 乗で比べても、判定は数学的には同じで、`std` の有無や OS で結果が変わらない (掛け算と足し算は IEEE 754 で決まる。ADR 0031 の目的は保てる)。
変わるのは、境目 (距離がほぼ `radius + margin`) で当たりかどうかが、丸めの違いで入れ替わりうること。掃引と二分法は不連続なので、1 度入れ替わると
その後の粒子の位置が大きく変わりうる (docs/spec/physics.md の例は境目の場面を含めないとしている)。

- **案 A (推奨。測ってから決める): 2 乗で比べる形を試し、物理そのものが 1 割以上速くなれば入れる。** 物理の golden
  (`golden_tests.rs`。T0333 の `colliding_chain_matches_recorded_hash` を含む) を記録し直す。リリース前なので、前の版と同じビット列を保つ必要は無い
  (利用者の方針)。1 割に満たなければ入れず、測った数をメモに書いて終える (案 B と同じ結果)。
- 案 B: 入れない。ビット列を保つ。物理そのものは 1 フレームの評価のうち小さい (T0193・T0334 の段の内訳で、生成のモデルの `update` 約 54µs の
  うち約 5µs) ので、全体では数 % にしかならない。
- 案 C: `point_segment_distance` を使う粒子の判定 (`blocks_from` の 1 つ目の条件と `push_out` の前の判定) だけを 2 乗にし、
  `segment_segment_distance` (区間どうしの距離) はそのまま残す。変える式を減らすが、ビット列が変わりうるのは A と同じ。

推奨は A。決まった物差し (物理そのものの時間) で入れるかを決め、効かなければ入れない。入れるなら、判定の式がどこで平方根を取らないかを
仕様に書く必要は無い (数学的に同じ判定なので) が、境目の丸めが実装で決まることは docs/spec/physics.md#衝突体 に書く。

## 受け入れ条件

- [ ] 判断をメモに書いた
- [ ] (A・C) 2 乗で比べる形にした。`radius + margin` が 0 以下・NaN・無限大のときに、今の判定 (`d < reach` が偽) と同じ結果になる
      (2 乗にすると負の `reach` でも真になりうるので、先に弾く)。単体テストでこれらの値と、境目から離れた位置の判定が前と同じことを確かめた
- [ ] (A・C) `physics_timing` (`dynamic_hair` の段の内訳) と `puppet_timing` の `physics stages` で前と後を同じ条件で測り、物理そのものの時間をメモに書いた
      (wasm は T0334 の docs/testing.md#1-フレームの評価 の「物理の段の内訳の調べ方」で測る)。1 割に満たなければ入れずに戻し、数だけをメモに書く
- [ ] (A・C で入れたとき) 物理の golden を記録し直し、変わったのが衝突体のあるケースだけであることを確かめた。`just conformance` が通る
      (参照は同じ Rust のコードで作るので一致する)。docs/spec/physics.md#衝突体 と、対応する英語の公開仕様 (spec/) の節を直した
- [ ] (A・C で入れたとき) `just wasm-check` と `just nostd-check` が通る
- [ ] `just ci` が通る

## 参考

- T0333 のメモ (結果・速さ・起票の候補)、T0193・T0334 のメモ、docs/spec/physics.md#衝突体、ADR 0031、docs/testing.md#1-フレームの評価
- crates/ugoe-physics/src/collider.rs (`WorldCollider::blocks_from`・`push_out`・`point_segment_distance`・`segment_segment_distance`・`SweepBuffer`)、
  crates/ugoe-physics/src/golden_tests.rs、crates/ugoe-physics/examples/physics_timing.rs、crates/ugoe/examples/puppet_timing.rs

## メモ

- T0081 で起票した (元の候補: T0333 (衝突の判定で距離を平方根を取らず 2 乗で比べる。ビット列が変わるので golden の記録し直しと仕様の判断が要る。急がない))。
- M7 の後続 (docs/roadmap.md の M7 の性能の行)。M7 の完了の条件には含めない。
- 手元の計測は、ほかのワーカーのビルドが並行していると揺れる。前と後を続けて交互に測り、load average と時刻をメモに書く (T0333 と同じ)。
- git stash は使わない (前と後は一時的なコミットか別の worktree で比べる)。

- 判断 (利用者、2026-10-09): 案 A (推奨) に決まった。
