---
id: T0451
title: "wasm: カーブの値を返す API でもセグメントの索引 (SegmentIndex) を使う"
status: in-progress
milestone: M9
crates: [ugoe-wasm]
depends_on: []
assignee: claude-worker-T0451
---

# T0451: wasm: カーブの値を返す API でもセグメントの索引 (SegmentIndex) を使う

## 目的

T0324 でモーションの再生はカーブのセグメントを索引 (`SegmentIndex`) で引くようになったが、`ugoe-wasm` の `animation.rs`
(フェードなどを掛けないカーブの値そのものを返す API。エディタのタイムラインのプレビューが使う) は、毎回線形の `curve::evaluate` を使う。
毎フレーム呼ぶ使い方なので、カーブごとに索引を持たせる。

## 受け入れ条件

- [ ] `animation.rs` のカーブの値の評価が索引を使い、結果が `curve::evaluate` と一致するテスト (ネイティブのテスト) がある
- [ ] モーションを差し替えたときに索引を作り直す
- [ ] `just ci` と `just wasm-check` が通る

## 参考

- T0324 のメモ (起票の候補)
- crates/ugoe-wasm/src/animation.rs、crates/ugoe-motion/src/curve.rs

## メモ

- T0019 で起票した (元の候補: T0324)。
