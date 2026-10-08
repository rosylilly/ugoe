---
id: T0333
title: "性能: 物理の衝突の判定で、掃引の間は変わらない親と衝突体の距離を 1 度だけ求める"
status: done
milestone: M7
crates: [ugoe-physics, docs]
depends_on: []
assignee: claude-worker-T0333
---

# T0333: 性能: 物理の衝突の判定で、掃引の間は変わらない親と衝突体の距離を 1 度だけ求める

## 目的

T0193 のプロファイルで、物理そのもの (`update_instance` の 1 段目の評価を除いた部分) の時間の大半は衝突の判定 (`collider.rs`) の
`WorldCollider::blocks` と掃引の算術だった。`constrain_segment` は区間を親のまわりに回しながら標本と二分法で `blocked` を何度も呼ぶが、
そのたびに `near()` の `out_of_reach` と `blocks` の 2 つ目の条件で、親と衝突体の距離 (`point_segment_distance(parent, a, b)`。平方根を含む) を
求め直している。親 (`parent`) と衝突体は掃引の間変わらないので、`constrain_segment` の始めに衝突体ごとに 1 度だけ求めれば、
結果のビット列を変えずに判定 1 回あたりの距離の計算が減る。ネイティブと wasm の両方で物理が 1〜2 割速くなる見込み (T0193 のメモ)。急がない。

## 受け入れ条件

- [x] `constrain_segment` の始めに、衝突体ごとの親からの距離 (と届く範囲にあるか) を 1 度だけ求め、`blocks`・`out_of_reach` はそれを使う。
      `constrain_segment` の呼び出しのたびのヒープの確保を増やさない (呼び出し側が持つ作業用のバッファを渡すか、衝突体の数が少ないことを前提に
      確保しない形にする。どちらにしたかと理由をコメントに書く)
- [x] 結果のビット列が変わらない: 物理の golden (`golden_tests.rs`) と `ugoe-physics` の既存のテストがそのまま通る。加えて、衝突体のあるフィクスチャを
      数百フレーム進めた粒子の位置が、直す前と後で完全に一致する (`to_bits` で比べる) テストを足した
- [x] `physics_timing` (と `puppet_timing`) で前と後を測り、衝突体のある設定で速くなったか (変わらなかったか) をメモに書いた。
      目に見えて速くなれば docs/testing.md#1-フレームの評価 の表を直す (wasm は bindings/web/examples/timing の `measure.mjs` で測る)
- [x] `just wasm-check` が通る
- [x] `just ci` が通る

## 参考

- T0193 のメモ (結果・測り方・起票の候補)、docs/testing.md#1-フレームの評価、docs/spec/physics.md (衝突の判定)
- crates/ugoe-physics/src/collider.rs (`constrain_segment`・`WorldCollider::blocks`・`out_of_reach`・`point_segment_distance`)、
  crates/ugoe-physics/examples/physics_timing.rs、crates/ugoe/examples/puppet_timing.rs

## メモ

- T0080 で起票した (元の候補: T0193 (物理の衝突の判定で、掃引の間は変わらない親の距離を 1 度だけ求める。結果のビット列は変わらず 1〜2 割速くなる見込み))。
- M7 の後続 (docs/roadmap.md の M7 の性能の行)。M7 の完了の条件には含めない。
- 結果のビット列を変える変更 (判定の式を変える・近似を使う) は仕様の判断になる。その場合は案をメモに書いて止まる。
- 手元の計測は、ほかのワーカーのビルドが並行していると揺れる。load average が下がってから測り、測った時の条件をメモに書く (T0193 と同じ)。

### 結果 (claude-worker-T0333)

**やったこと:**

- `collider::constrain_segment` の始めに、衝突体ごとに親から芯までの距離を 1 度だけ求め、届く範囲にある衝突体と距離の組を
  作業用のバッファ (`collider::SweepBuffer`) に入れる。掃引と二分法の判定 (`WorldCollider::blocks_from`) と押し出しはこの組を使う。
  `out_of_reach` は無くし、同じ式 (`距離 >= length + radius + margin` なら除く。NaN の扱いも同じ) をバッファを作るところに書いた。
- バッファは呼び出し側が持つ形にした (`constrain_segment` に `&mut SweepBuffer` を足した。`PhysicsSystem` が 1 つ持って使い回す)。
  リグの衝突体の数はモデルで決まり上限が無いので、決まった大きさのスタックの配列では足りない場合の別の道が要る。`Vec` を使い回せば
  容量が足りた後は確保しない。理由は `SweepBuffer` の doc コメントに書いた。公開の `WorldCollider::blocks` は残した (中で同じ判定を呼ぶ)。
- テスト `golden_tests::colliding_chain_matches_recorded_hash`: 衝突体 7 つ (円・カプセル・半径 0.3 の細い円・親を包む大きな円) の間で
  6 粒子の鎖を大きく振って 600 ステップ進め、粒子の位置と速度の `to_bits` を FNV で畳んだハッシュを、直す前の実装で記録した
  (先にテストだけをコミットした)。一時的に数えて、掃引で止まる 379 回・始めの向きで当たる 427 回・どの向きでも当たって押し出す 88 回を
  通ることを確かめた (`to` が親と重なる場合は通らない)。
- crates/ugoe-physics/README.md にバッファのことを書いた。

**確かめたこと (結果が変わらない):**

- 物理の golden (`golden_tests.rs`) と `ugoe-physics` の既存のテスト、足したテストがそのまま通る (debug と release)。
- T0193 の作業用のクレートで、生成のモデルと `avatar.ugoe` を 1,200 フレーム進めた頂点とパラメータのハッシュが、直す前と後・ネイティブと
  wasm の 4 通りですべて T0193 と同じ (`e9b9cf0fb09082ce`・`ab157425c0847d11`)。
- `just conformance` は全部 ok。`just perf` も ok (ネイティブ: 生成のモデルの `update` 53.9µs 前後 / 上限 300µs。wasm の `update`
  の中央値は 101.6µs / 300µs。測った時は load average 5〜6 で揺れた)。`just nostd-check`・`just wasm-check` も通る。

**速さ (前と後を同じ条件で続けて交互に測った。AMD Ryzen 9 7900・WSL2、ほかのワーカーのビルドが並行して load average 5〜8、17:38〜17:40):**

- `physics_timing` の `dynamic_hair` (衝突体あり) は 3 回ずつ 0.79・0.80・0.78 → 0.75・0.75・0.75µs (約 5% 速い)。衝突体の無い設定は変わらない。
- T0193 の段ごとの計測 (生成のモデル。200 フレームの平均の最速): 物理そのもの (`update_instance` から 1 段目の評価を除いたもの) は
  ネイティブで 5.54・5.90 → 5.19・5.05µs (約 1 割)、wasm (ヘッドレスの Chrome 154、3 回の中央値) で 7.10 → 6.00µs (約 15%)。
  `avatar.ugoe` は物理そのものが 0.3µs ほどで変わらない。
- `puppet_timing` (ネイティブ) の生成のモデルの `update` は 53.8・53.9 → 53.6・53.4µs、計測のページ (wasm、`measure.mjs --runs 5`) の
  `update` は 56.6・57.5 → 58.2・58.8µs (最速)。物理の段は `update` 全体の 1〜2 割なので、改善は揺れに埋もれて見えない。
  目に見えて速くはならなかったので docs/testing.md#1-フレームの評価 の表は直していない。
- 見込み (1〜2 割) より小さかったのは、`blocks` 1 回の平方根のうち親の距離は 1 つで、残り (`q` の距離と、区間どうしの距離の
  `point_segment_distance` 4 回) は向きごとに変わるので減らせないため。

**やらなかったこと:**

- 判定の式そのものを変える改善 (距離を 2 乗のまま比べて平方根を省くなど) は、丸めが変わり結果のビット列が変わりうるので、していない。

**起票の候補:**

- 衝突の判定で、距離を平方根を取らずに 2 乗で比べる (`point_segment_distance(q, …) < radius + margin` を `d2 < (radius + margin)^2` に
  するなど)。境目の判定の丸めが変わるので結果のビット列が変わりうる (golden の記録し直しと仕様の判断が要る)。物理そのものの残りの大半が
  この算術なので、ネイティブ・wasm とも物理そのものがもう 1〜2 割速くなる見込み。急がない。
