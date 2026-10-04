---
id: T0319
title: 物理の毎フレームの確保とパラメータの名前の探索を減らす
status: done
milestone: M9
crates: [ugoe-physics]
depends_on: [T0316]
assignee: claude-worker-T0319
---

# T0319: 物理の毎フレームの確保とパラメータの名前の探索を減らす

## 目的

`PhysicsSystem::update` / `update_instance` は毎フレーム次を行っている (T0316 のレビューで見つけた)。

- `RigFrame` の `Vec` (リグごとに `references`・`flips`)、`targets`・`starts` の `Vec` を作る。ステップごと・リグごとに `outputs()` が `Vec` を作る。
- 中継があると `Relay::new` が `ParameterValues` を丸ごと複製する (パラメータの ID の `String` もすべて複製する)。
- 入力 (`root_pose`、ステップごと) と出力 (`write_parameters`・`Relay::new`) のたびに、`ParameterValues::index_of` でパラメータの名前を線形に探す
  (出力の数 × パラメータの数の文字列比較)。

ランタイムの目標 (1 体 1ms、roadmap.md の M7) に対して今すぐ問題になる量ではないが、物理の計測がないので、計測を足してから減らす。

## 受け入れ条件

- [x] 物理の 1 フレームの時間を測るベンチマーク (またはテスト内の計測) を足す (`fixtures::dynamic_hair`・`physics_relay`)
- [x] パラメータの添字を解決して持つ (渡された `ParameterValues` の並びが変わったら解決し直す)、写しと出力のバッファを使い回すなどで、毎フレームの確保と名前の探索をなくす
- [x] 結果がビット単位で変わらないことを、既存の決定性のテストと前後の比較で確かめる
- [x] `just ci` と `just wasm-check` が通る

## 参考

- `docs/tasks/T0316-review-physics.md` のメモ
- `docs/spec/physics.md`

## メモ

### やったこと

- 計測: `crates/ugoe-physics/examples/physics_timing.rs` (`cargo run --release -p ugoe-physics --example physics_timing`)。
  `physics_hair`・使わないパラメータを 100 個足した `physics_hair`・`physics_relay` を `update` で、`dynamic_hair` を
  `update_instance` で進め、1000 フレームずつの平均の最小値を示す。criterion は入れていない (新しい依存を避けた)。
  この環境 (ほかのワーカーと並行、WSL2) での目安 (変更前 → 後、us/frame): hair 0.27 → 0.2、
  hair + 100 params 0.63〜0.82 → 0.35、relay 0.48〜0.70 → 0.3、dynamic_hair 1.05〜1.28 → 0.8〜0.9。
- ビット単位の確かめ: 変更の前に `src/golden_tests.rs` を足し、決まった入力列 (入力と `dt` を揺らす 240 フレーム) の
  パラメータの値・粒子・描画の頂点の FNV ハッシュを記録した。変更の後も同じハッシュ (既存の決定性のテストも通る)。
  場面は hair・relay・dynamic_hair・途中で並びの違う (逆順・パラメータが足りない) `ParameterValues` に切り替えるもの。
  記録したハッシュは `sin` などの結果がプラットフォームで違いうるので x86_64 Linux でだけ比べ、
  並びを逆にしても結果が同じことはどのプラットフォームでも確かめる。
- 実装 (`src/lib.rs`):
  - 入力と出力のパラメータの添字を `RigState::input_params`・`output_params` に解決して持つ。渡された
    `ParameterValues` の ID の並び (`ids()`) が前に解決したときと違うときだけ解決し直す (`resolve_parameters`)。
  - 中継の写し (`Relay`) は並びごとに 1 回作り、フレームの始めに `clone_from` で値を写す (ID の `String` を複製しない)。
  - `RigFrame` の列・ワールドの衝突体・`targets`・`starts` を `PhysicsSystem` に持って使い回し、`outputs()` は
    `RigState::current` に直接書く。`update`・`update_instance` の毎フレームの確保はなくなった
    (`rest_pose_collisions` は毎フレームではないので、作業用の `Vec` を作るまま)。
- `crates:` の外の小さな変更 (`ugoe-runtime` の `ParameterValues`):
  - `pub fn ids(&self) -> &[ParamId]` を足した (読むだけの accessor。並びの変化を見分けるため)。
  - `Clone` を手で実装し、`clone_from` がバッファを使い回すようにした (ID の並びが同じなら ID を写さない)。
    `clone` の意味は変わらない。テスト `clone_from_copies_values_ranges_and_ids` を足した。

### やらなかったこと

- 確保の数そのものを数えるテスト: グローバルアロケータの差し替えには `unsafe` が要るので入れていない。
  毎フレームの確保がないことはコードを読んで確かめた。
- 毎フレーム、並びの確かめ (`ids()` の比較) と中継の `clone_from` の中の比較で、パラメータの ID を 2 回比べる
  (O(パラメータ数) の比較で、探索ではない)。世代番号などで O(1) にするには `ParameterValues` の設計が変わるので見送った。

### 起票の候補

- ほかのクレートの毎フレームの名前の探索: `ugoe-motion` の `MotionPlayer`・`Expression` は添字を最初の `apply` で
  解決して以後は確かめない (並びの違う `ParameterValues` を渡すと別のパラメータに書く)。`tracking.rs` は毎回 `index_of` する。
  `ParameterValues::ids()` で同じ確かめができる。
- `ParameterValues` の並びの変化を O(1) で見分ける仕組み (ID の列を `Arc` で共有して `ptr_eq` を先に見るなど)。

### `just ci`

- `just ci` と `just wasm-check` は通った (main は b5722f1)。
