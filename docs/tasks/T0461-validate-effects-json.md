---
id: T0461
title: エフェクトの設定を差し替える set_effects_json (C ABI・wasm・Godot) で設定を検証する
status: in-progress
milestone: M9
crates: [ugoe, ugoe-ffi, ugoe-wasm, bindings/godot, docs]
depends_on: []
assignee: claude-worker-T0461
---

# T0461: エフェクトの設定を差し替える set_effects_json (C ABI・wasm・Godot) で設定を検証する

## 目的

目パチ・呼吸・リップシンクの設定を差し替える `set_effects_json` (C ABI・wasm・Godot) は設定を検証しない。JSON の `1e39` は `f32` の無限大になる。
ランタイムは安全に扱う (docs/portability.md#ホストから来る数 の表) が、作者の誤りを見逃す (T0010 の候補)。モデルの読み込みと同じ検証を通す。

今まで受けていた入力を拒むようになるので、拒むときのエラー (種類と補足の値) を各組み込み先で揃える。

## 受け入れ条件

- [ ] SDK に設定の検証 (モデルの `validate` のエフェクトの検査と同じもの) を通す入口があり、C ABI・wasm・Godot がそれを使う
- [ ] 有限でない値・範囲の外の値を拒み、拒んだときに今の設定を変えないテストがある (SDK・C ABI・wasm)
- [ ] 各 README と docs/portability.md を更新した
- [ ] `just ci`・`just wasm-check`・`just godot-check` が通る

## 参考

- T0010 のメモ (T0010 の結果の起票の候補)
- crates/ugoe-ffi/src/animation.rs、crates/ugoe-wasm/src/animation.rs、bindings/godot/src/node/sdk.rs

## メモ

- T0026 で起票した (元の候補: T0010 の `set_effects_json`)。
