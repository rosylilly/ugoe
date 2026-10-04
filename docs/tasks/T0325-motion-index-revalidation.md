---
id: T0325
title: "ugoe-motion: 解決した添字を並びの変化で解決し直し、毎フレームの探索と確保を減らす"
status: done
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

- [x] `MotionPlayer`・`Expression` に並びの違う `ParameterValues` を渡すと正しいパラメータに書くテスト (先に失敗するテスト)
- [x] `tracking.rs` の毎回の `index_of` を、解決した添字の使い回しにした
- [x] `PosePlayer::contains` と `part_opacities` の毎フレームの確保をなくした (振る舞いは変えない。既存のテストが通る)
- [x] `just ci` と `just wasm-check` が通る

## 参考

- T0319・T0146 のメモ (起票の候補)
- crates/ugoe-motion/src/player.rs・expression.rs・tracking.rs・pose.rs、crates/ugoe/src/lib.rs (`Puppet::apply_part_opacities`)

## メモ

- T0019 で起票した (元の候補: T0319 の添字の確かめと `index_of`、T0146 の `PosePlayer::contains` と `part_opacities`)。

### やったこと

- 解決した添字の控え `ResolvedParams` (`crates/ugoe-motion/src/resolve.rs`) を足した。ugoe-physics の `resolve_parameters` と同じ形で、
  並びの手札が同じなら定数時間でそのまま使い、手札が違っても ID の列が同じなら手札だけ取り替え、違えば解決し直す。
  `MotionPlayer`・`ExpressionPlayer` に加えて、同じ問題があった `EyeBlinkPlayer`・`BreathPlayer`・`LipSyncPlayer` もこれにした。
  先に失敗するテスト (`player`・`expression` の `resolves_parameters_again_when_layout_changes`) をコミットしてから直した。
- `tracking.rs`: 起票の時点の記述と違い、`TrackingMap` は作るときに添字を解決してあり、毎フレームの `index_of` はもう無かった。
  ただし作ったときと並びの違う `ParameterValues` に `apply` すると別のパラメータに書く同じ問題があったので、作ったときの並びの手札を持ち、
  並びが違うときだけ書き込み先ごとに ID で引き直して (`index_of` は定数時間) 同じ ID のパラメータに書くようにした (`apply` は `&self`
  のままなので控えは更新しない。`apply_where` の `filter` には書き込む先の並びの添字を渡す)。`PartialEq` は並びを ID の列で比べる手書きにした。
- パーツの不透明度:
  - `MotionManager::part_opacities_in(&self, &mut PartOpacityBuffer)` を足した。重ねた結果をパーツの ID を複製せずに
    `(&PartId, 値, 重み)` で返す (パーツは最初に現れたカーブの (プレイヤー, カーブ) の番号で指す)。`part_opacities` はこれを使って作る (値は同じ)。
  - `PosePlayer::update_by(dt, request)` を足した (要求を列でなく関数で引く)。`update` はこれを使う。
  - `Puppet` は作業領域 `part_opacity_buffer` と、パーツの添字ごとに「ポーズの要素か」を `set_pose` で作った `pose_members` を持ち、
    `apply_part_opacities` を確保なしの関数にした (`PosePlayer::contains` の線形探索もなくした)。受け手の `follow` もパーツの ID を複製しない。
    同じパーツが複数あるときの振る舞い (上書きは後、ポーズへの要求は先) と、モデルにないポーズの要素への要求も前と同じ。
- `crates/ugoe-motion/README.md` の「守ること」に添字の解決し直しを書いた。仕様書 (`docs/spec/motion.md`・`spec/`) にはこの内部の記述がないので変えていない。
- `just ci` と `just wasm-check` は通った (main は 35213ba9 に rebase)。

### やらなかったこと

- 確保の数を数えるテスト (グローバルアロケータの差し替えに `unsafe` が要る)。毎フレームの確保がないことはコードを読んで確かめ、
  `PartOpacityBuffer` の容量が使い回されることだけテストした。
- `MotionManager::update` のイベントの `cloned()` (イベントの文字列の複製) は、イベントが発火したフレームだけなので触っていない。

### 起票の候補

- `TrackingMap` が並びの違う `ParameterValues` を毎フレーム受けると、書き込み先ごとに `index_of` で引き直す (定数時間だが毎回)。
  `apply` を `&mut self` にして控えを取り替えれば避けられるが、公開 API の変更になるので見送った。今の呼び出し側 (Puppet・FFI・wasm・Godot・スタジオ) は
  同じインスタンスのパラメータに使うので起きない。

