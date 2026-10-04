---
id: T0325
title: "ugoe-motion: 解決した添字を並びの変化で解決し直し、毎フレームの探索と確保を減らす"
status: in-progress
milestone: M9
crates: [ugoe-motion, ugoe, docs]
depends_on: [T0160]
assignee: claude-worker-T0325
---

# T0325: ugoe-motion: 解決した添字を並びの変化で解決し直し、毎フレームの探索と確保を減らす

## 目的

T0319・T0146 で物理と SDK の ID の引き当てを毎フレームの線形探索でなくしたが、`ugoe-motion` に残りがある。

- **正しさ:** `MotionPlayer`・`Expression` はパラメータの添字を最初の `apply` で解決し、以後は確かめない。並びの違う `ParameterValues` を
  渡すと別のパラメータに書く。T0160 の口 (並びが同じかを定数時間で見分ける) で確かめ、違えば解決し直す。
- **探索:** `tracking.rs` は毎回 `index_of` で引く。
- **確保:** `Puppet::apply_part_opacities` のポーズの判定 (`PosePlayer::contains`) と、`MotionManager::part_opacities` のカーブの `Vec` を毎フレーム確保する。

## 受け入れ条件

- [ ] `MotionPlayer`・`Expression` に並びの違う `ParameterValues` を渡すと正しいパラメータに書くテスト (先に失敗するテスト)
- [ ] `tracking.rs` の毎回の `index_of` を、解決した添字の使い回しにした
- [ ] `PosePlayer::contains` と `part_opacities` の毎フレームの確保をなくした (振る舞いは変えない。既存のテストが通る)
- [ ] `just ci` と `just wasm-check` が通る

## 参考

- T0319・T0146 のメモ (起票の候補)
- crates/ugoe-motion/src/player.rs・expression.rs・tracking.rs・pose.rs、crates/ugoe/src/lib.rs (`Puppet::apply_part_opacities`)

## メモ

- T0019 で起票した (元の候補: T0319 の添字の確かめと `index_of`、T0146 の `PosePlayer::contains` と `part_opacities`)。
