---
id: T0324
title: モーションのカーブの評価でセグメントを線形に探さない
status: done
milestone: M9
crates: [ugoe-motion, docs]
depends_on: [T0317]
assignee: claude-worker-T0324
---

# T0324: モーションのカーブの評価でセグメントを線形に探さない

## 目的

`ugoe_motion::curve::evaluate` は、毎フレーム、カーブの先頭からセグメントを線形に探す (未知のセグメントの有無も毎回先頭から調べる)。
フレームごとにキーを打ったモーション (30fps で 10 秒なら 1 カーブ 300 セグメント) を多くのカーブで再生すると、1 体 1ms の目標
(roadmap.md の M7) に対して無視できない。T0317 のレビューで見つけた (マネージャで同じカーブを 4 回評価していた分は T0317 で直した)。

## 受け入れ条件

- [x] `MotionPlayer` が初回の適用でカーブごとに前処理する (未知のセグメントの有無、2 つ目の `start` の位置、区間の終わりの時刻の列) か、前回の区間を覚えて次の区間から探すようにする
- [x] 時刻が減るセグメントを含むカーブ (書き出しの規則に反するもの) の結果を、今の実装と変えない (変えるなら仕様に書く)
- [x] 長いカーブでの評価の計測を `ugoe` の性能のテスト (docs/testing.md) に足し、改善を確かめる
- [x] `just ci` が通る

## 参考

- docs/spec/motion.md#カーブ
- crates/ugoe-motion/src/curve.rs

## メモ

- やったこと:
  - `ugoe_motion::curve::SegmentIndex` を足した。カーブごとに、値を持つか (空・先頭が `start` でない・未知のセグメント)、2 つ目の `start` の位置、
    区間の終わりの時刻の累積最大値 (NaN は飛ばす) を前もって作り、`partition_point` で区間を探す (`O(log n)`)。
    線形の探索が選ぶ「`t <= to.t` となる最初の区間」は、累積最大値が初めて `t` 以上になる位置と同じなので、時刻が減るカーブでも結果は変わらない。
    NaN の `t` も線形と同じく最後の点の値になる。
  - `MotionPlayer::new` で全カーブの `SegmentIndex` を作り、`apply`・`curve_values` (マネージャ経由も) はそれを使う。
    `curve::evaluate` (線形) は公開 API として残した (`ugoe-wasm`・`ugoe-editor-core` のテストが使う。1 回だけ評価するならこちらで足りる)。
    区間の値の計算は両者で共有する関数にまとめた。
  - テスト: 既存の例のカーブ・時刻が減るカーブ・乱数で作ったカーブ 500 本 (時刻の後戻り・NaN の点・2 つ目の `start`・未知のセグメントを含む) で、
    線形の探索とビット単位で一致することを確かめる。累積最大を取らない誤った実装でテストが落ちることも確かめた。
  - 計測: `cargo run --release -p ugoe-motion --example curve_timing` (300 セグメント × 100 カーブ、60fps で 10 秒)。WSL2、ほかのワーカーと並行で、
    線形 18.79 µs/フレーム → 前処理 1.73 µs/フレーム (約 11 倍)。`MotionPlayer` の advance + 評価は 2.58 µs/フレーム。
  - 仕様の振る舞いは変えていないので `docs/spec/motion.md` は直していない。
- 受け入れ条件の「`ugoe` の性能のテスト (docs/testing.md)」は、既存の仕組みが無かったので、`compose_timing` と同じ形の計測用 example を
  `ugoe-motion` に置き、`docs/testing.md` に「性能の計測」の節を足して両方の example を載せた (`crates/ugoe` は範囲外なので触っていない)。
  計測は機械に左右されるので `just ci` には入れていない。
- `just ci` は通った (exit 0)。ランタイム層を触ったので `just wasm-check` も通した。
- 起票の候補:
  - `ugoe-wasm` の `animation.rs` (カーブの値そのものを返す API) も毎回線形の `curve::evaluate` を使う。毎フレーム呼ぶ使い方なら `SegmentIndex` を持たせる。
