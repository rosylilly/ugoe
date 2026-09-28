---
id: T0302
title: モーションカーブと再生
status: done
milestone: M3
crates: [ugoe-motion]
depends_on: []
assignee: claude-worker-T0302
---

# T0302: モーションカーブと再生

## 目的

ベジェ区間を含むカーブの評価と、`MotionPlayer` のフェード・ループ・イベントを実装する。

## 受け入れ条件

- [x] ベジェ区間の t→s の解決 (単調性を前提とした数値解法) とテストがある
- [x] フェードとループのテストがある
- [x] ループをまたいでもイベントを漏らさず発火するテストがある
- [x] `just ci` が通る

## 参考

- `docs/spec/motion.md#モーション`

## メモ

- ベジェ: `curve::solve_monotonic_bezier` (ニュートン法を二分法で守る。許容誤差 1e-6)。制御点の時刻は評価時にも区間内へクランプする。
- `MotionPlayer` (`src/player.rs`): 経過時間 `elapsed` と再生位置 `time` を分けて持つ。フェードは終了時刻 `end` に向けてかけ、ループしないモーションは `end = duration` で自然にフェードアウトする。`stop()` で「今から `fade_out` 秒後」を終了時刻にする (T0303 のクロスフェード用)。`advance` / `apply` / `update` を分け、`fired_events()` と `curve_values()` (対象・値・フェードの重み) を公開した。
- `update` の `weight` は「フェード込み」から「フェードに掛ける外部の重み」に意味を変えた (`Puppet` は 1.0 を渡しているので挙動は変わらない)。
- 細則は `docs/spec/motion.md#再生とフェード` に追記した。
- やらなかったこと (T0303): パーツの不透明度・エフェクトグループのカーブの適用 (`curve_values()` で取り出せるようにしただけ)、優先度・キュー、`Puppet::play` の置き換え。
- 長さ 0 のループしないモーションは開始時点で終わり扱いになり、何も適用しない。必要なら T0303 で扱いを決める。
- パラメータの添字は最初の `apply` で解決してキャッシュする。別のモデルの `ParameterValues` に使い回すことは想定していない。
