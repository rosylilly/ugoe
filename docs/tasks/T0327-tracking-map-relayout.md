---
id: T0327
title: TrackingMap が並びの違う ParameterValues を受け続けるときの毎フレームの引き直しをなくす (判断が要る)
status: done
milestone: M9
crates: [ugoe-motion, ugoe, docs]
depends_on: []
assignee: claude-worker-T0327
---

# T0327: TrackingMap が並びの違う ParameterValues を受け続けるときの毎フレームの引き直しをなくす (判断が要る)

## 目的

T0325 で、モーション・表情が解決した添字を並びの変化 (T0160 の `ParameterLayout`) で解決し直すようにした。`TrackingMap` は
`apply(&self, ...)` なので控えを取り替えられず、並びの違う `ParameterValues` を毎フレーム受けると書き込み先ごとに `index_of` で引き直す (T0325 の候補)。
今の呼び出し側 (Puppet・FFI・wasm・Godot・スタジオ) は同じインスタンスのパラメータに使うので起きない。

**判断が要る:** `apply` を `&mut self` にするのは公開 API の変更。内部可変性 (`Cell` など) で `&self` のまま控えを取り替える案もある。どちらにするか。

## 受け入れ条件

- [x] 方針を決めた (利用者の判断を待つなら blocked にしてメモに案を書く)
- [x] 並びの違う `ParameterValues` を続けて渡しても、2 回目からは引き直さないことを確かめるテストがある
- [x] 呼び出し側 (`ugoe` の Puppet など) を合わせ、`just ci` と `just wasm-check` が通る

## 参考

- T0325 のメモ (起票の候補)、T0160
- crates/ugoe-motion/src/tracking.rs

## メモ

- T0026 で起票した (元の候補: T0325)。

### やったこと

- 方針: 公開 API を壊さない内部可変性にした (オーケストレーターの指示どおり)。`apply` / `apply_where` は `&self` のまま。
  - `TrackingMap` に、作ったときと違う並びで引き直した添字の控え `Mutex<ResolvedParams>` (T0325 の `crates/ugoe-motion/src/resolve.rs`) を足した。
    並びの手札が作ったときと同じなら控えに触らない (今の呼び出し側はすべてこれ。ロックも取らない)。違えば控えを使い、同じ並びが続けば 2 回目からは引き直さない。
  - `RefCell` / `Cell` だと `TrackingMap` と `Puppet` が `Sync` でなくなる (Bevy のコンポーネントなどで困る) ので `Mutex` にした。`try_lock` で取り、
    ほかのスレッドが使っていて取れないときは、そのフレームだけ以前どおり書き込み先ごとに引き直す (待たない)。テストで `Send + Sync` を確かめる。
  - 毎フレームのメモリの確保はしない (控えの `Vec` は解決し直すときだけ詰め直す)。
  - `Clone` は控えを写さない (手で実装)。`PartialEq` は以前どおり控えを比べない。
- テスト `keeps_indices_resolved_for_a_different_layout` (tracking.rs): 同じ並びでは控えを作らない、違う並びの 1 回目で控えを作り 2 回目はそのまま使う、
  元の並びでは控えを使わない、控えが取れないときも同じ値を書く、写しは控えを持たない。
- 呼び出し側 (`ugoe` の Puppet・FFI・wasm・Godot・スタジオ) は API が変わらないので直していない。crates/ugoe-motion/README.md に一文足した。
- `just ci` と `just wasm-check` が通った。

### やらなかったこと

- `apply(&mut self)` への変更 (公開 API を壊すので見送った)。

### 起票の候補

- なし。
