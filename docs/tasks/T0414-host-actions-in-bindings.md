---
id: T0414
title: バインディング: ポーズの表示の選択・効果の差し替えを C ABI と wasm に出す・CLI のビュー変換
status: todo
milestone: M7
crates: [ugoe-ffi, ugoe-wasm, ugoe-cli, spec, docs]
depends_on: [T0130, T0412]
assignee:
---

# T0414: ホストの操作をバインディングに揃える

## 目的

T0130 の後続の候補をまとめる。公開仕様のホストの操作 (spec/motion.md) のうち、参照実装の Rust の API にしかないものがある。

## 受け入れ条件

- [ ] ポーズの表示の選択 (`PosePlayer::show`、パーツ ID で指定) を C ABI と wasm に出す
- [ ] 効果の設定の差し替え (`Puppet::set_effects`) を C ABI に出す (wasm は set_effects_json がある)
- [ ] `ugoe render` にビュー変換 (6 数) の指定を足し、描画の例の view のケースを黒箱でも比べられるようにする (tools/spec-check/src/render-reference.ts はワーカーが spec/ だけを見て直す別の作業でもよい)
- [ ] ビュー変換が退化したとき (逆変換が無い) の点光源の扱いを仕様で決めるか、適合の対象外と書く (spec/rendering.md と docs/spec の同期)
- [ ] 視線 (look_at) を時間の例に入れるかを決める
- [ ] `just ci`・`just wasm-check`・`just ffi-check` が通る

## 参考

- `docs/tasks/T0130-spec-tracking-timeline.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
