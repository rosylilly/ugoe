---
id: T0327
title: TrackingMap が並びの違う ParameterValues を受け続けるときの毎フレームの引き直しをなくす (判断が要る)
status: todo
milestone: M9
crates: [ugoe-motion, ugoe, docs]
depends_on: []
assignee:
---

# T0327: TrackingMap が並びの違う ParameterValues を受け続けるときの毎フレームの引き直しをなくす (判断が要る)

## 目的

T0325 で、モーション・表情が解決した添字を並びの変化 (T0160 の `ParameterLayout`) で解決し直すようにした。`TrackingMap` は
`apply(&self, ...)` なので控えを取り替えられず、並びの違う `ParameterValues` を毎フレーム受けると書き込み先ごとに `index_of` で引き直す (T0325 の候補)。
今の呼び出し側 (Puppet・FFI・wasm・Godot・スタジオ) は同じインスタンスのパラメータに使うので起きない。

**判断が要る:** `apply` を `&mut self` にするのは公開 API の変更。内部可変性 (`Cell` など) で `&self` のまま控えを取り替える案もある。どちらにするか。

## 受け入れ条件

- [ ] 方針を決めた (利用者の判断を待つなら blocked にしてメモに案を書く)
- [ ] 並びの違う `ParameterValues` を続けて渡しても、2 回目からは引き直さないことを確かめるテストがある
- [ ] 呼び出し側 (`ugoe` の Puppet など) を合わせ、`just ci` と `just wasm-check` が通る

## 参考

- T0325 のメモ (起票の候補)、T0160
- crates/ugoe-motion/src/tracking.rs

## メモ

- T0026 で起票した (元の候補: T0325)。
