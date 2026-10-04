---
id: T0462
title: モーションのイベントを発火したモーション (名前と番号) を C ABI・wasm・Unity・Godot から読む (判断が要る)
status: todo
milestone: M9
crates: [ugoe-ffi, ugoe-wasm, bindings/web, bindings/unity, bindings/godot, docs]
depends_on: [T0461]
assignee:
---

# T0462: モーションのイベントを発火したモーション (名前と番号) を C ABI・wasm・Unity・Godot から読む (判断が要る)

## 目的

T0821 で SDK のモーションのイベントに出どころ (どのモーションか) を持たせたが、組み込み先 (C ABI・wasm の `motion_events`・Unity・Godot) からは
イベントの名前しか読めない (T0821 の候補)。

**判断が要る:** 口の形。wasm は `motion_events` の戻り値を変える (互換を壊す) か、別の関数 (`motion_events_json` など) を足すか。
C ABI は `ugoe_puppet_motion_event_*` に添字で読む関数を足す形か。先に形を決めてから、4 つの組み込み先を揃える。

## 受け入れ条件

- [ ] 口の形を決め、C ABI・wasm の README に書いた (利用者の判断を待つなら blocked にしてメモに案を書く)
- [ ] C ABI・wasm・Unity・Godot からイベントのモーションの名前と番号を読め、それぞれテストがある (`@ugoe/runtime` の型も)
- [ ] 適合テストにイベントの出どころを比べるケースがある
- [ ] `just ci`・`just wasm-check`・`just unity-check`・`just godot-check`・`just conformance` が通る

## 参考

- T0821 のメモ (起票の候補)
- crates/ugoe-ffi/src/animation.rs、crates/ugoe-wasm/src/animation.rs

## メモ

- T0026 で起票した (元の候補: T0821)。C ABI・wasm の `animation.rs` を触る T0461 の後に着手する。大きければ「口の形 (C ABI・wasm)」と「Unity・Godot」に分ける。
