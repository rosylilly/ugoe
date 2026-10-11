# ugoe-physics

**層:** L3 機能 (依存してよいクレートは `scripts/check-layers.mjs` を参照)

振り子チェーンによる物理演算 (`PhysicsSystem`)。

- `PhysicsSystem`: パラメータの入出力、固定ステップ、出力の補間。
- 動的物理 (`PhysicsSystem::update_instance`): モデルの 1 段目の評価から根元・衝突体・重力の基準を求め、
  回転デフォーマの角度・原点、ワープの制御点、メッシュの頂点に出力する (2 段階の評価)。
  `PhysicsSystem::rest_pose_collisions` (`RestPoseCollision`) で静止姿勢の当たりを調べる。
- `Chain`・`Particle`: 振り子チェーン 1 本の粒子の状態。外からは `PhysicsSystem::chain` で読むだけ (作って進めるのは
  `PhysicsSystem` の中)。

公開するのは上の項目と `DEFAULT_FPS` だけ。次は crate の中の部品 (`pub(crate)`)。

- モジュール `chain`: 振り子チェーン 1 本の積分 (`Chain::step`・`Chain::settle`)。パラメータを知らないので、動的物理からも使う。
- モジュール `collider`: 衝突体 (円・カプセル。`WorldCollider`) と `constrain_segment`。区間の長さを保って親のまわりに回し、
  衝突体の手前で止める (すり抜けない)。親と衝突体の距離は掃引の始めに 1 度だけ求め、作業用のバッファ
  (`SweepBuffer`) は呼び出し側 (`PhysicsSystem`) が持って使い回す。

## 守ること

- `no_std` + `alloc` で書く。既定で有効な feature `std` を外しても公開 API は変わらない (`just nostd-check`。[規則](../../docs/portability.md#ランタイム層が守る制約-移植性のため))
- 決定的であること
- 固定ステップで積分する
- 有限でない粒子と出力を外に出さない (鎖を静止姿勢に置き直し、出力を捨てる。`docs/spec/physics.md#有限でない状態`)
- 毎フレームの確保とパラメータの名前の探索をしない (パラメータの添字は並びが変わったときだけ解決し直し、
  作業用のバッファは使い回す)。1 フレームの時間は `cargo run --release -p ugoe-physics --example physics_timing`
  で測る (`dynamic_hair` の段の内訳 (1 段目の評価・物理そのもの・2 段目の評価) も出す。大きなモデルの段の内訳は
  `ugoe` の例 `puppet_timing`。[docs/testing.md](../../docs/testing.md#1-フレームの評価))。結果をビット単位で保つことは `src/golden_tests.rs` で確かめる

## 仕様

- [docs/spec/physics.md](../../docs/spec/physics.md)

## 関連タスク

- T0301 (`docs/tasks/T0301-*.md`)
- T0305 (動的物理)
