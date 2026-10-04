---
id: T0430
title: Unity のバインディングで、C ABI の新しい関数 (受け手・トラッキング優先・本体の物理など) を使えるようにする
status: todo
milestone: M9
crates: [bindings/unity, docs]
depends_on: [T0423]
assignee:
---

# T0430: Unity のバインディングで、C ABI の新しい関数を使えるようにする

## 目的

T0423 (ugoe-ffi のレビュー) で、`include/ugoe.h` と `bindings/unity/com.ugoe.unity/Runtime/Core/Native.cs` の宣言を突き合わせた。
構造体の並びと型は一致していたが、次の 9 つの関数は C# に宣言が無く、Unity から使えない (README にも範囲外とは書いていない)。

- `ugoe_puppet_follow`・`ugoe_puppet_set_base_values` (受け手。T0133〜T0135)
- `ugoe_puppet_set_motion_over_tracking`・`ugoe_puppet_motion_over_tracking`・`ugoe_puppet_set_tracking_kept` (T0722)
- `ugoe_puppet_set_base_physics_json`・`ugoe_puppet_base_physics_json` (T0416)
- `ugoe_puppet_part_evaluated_opacity` (T0416)
- `ugoe_puppet_positions` (内部のバッファの借用。コピーの `ugoe_puppet_copy_positions` はある)

必要なものを `Native.cs` に宣言して `Puppet` などから使えるようにするか、使わないものは README に「範囲外」とその理由を書く。

## 受け入れ条件

- [ ] 上の 9 つのそれぞれについて、宣言して公開したか、README に範囲外として書いた
- [ ] 足した宣言の型 (`bool` は `byte`、`size_t` は `UIntPtr`) が `ugoe.h` と一致する
- [ ] `just ci` が通る

## 参考

- `crates/ugoe-ffi/README.md`・`crates/ugoe-ffi/include/ugoe.h`
- T0426 (Unity のバインディングのレビュー。重なっていればそちらで扱う)

## メモ
