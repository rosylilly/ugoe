---
id: T0454
title: 指揮役の側の値 (物理の直前の値・当てたリップシンク・パーツの不透明度) を C ABI・wasm・Godot から読む
status: todo
milestone: M9
crates: [ugoe-ffi, ugoe-wasm, bindings/godot, docs]
depends_on: []
assignee:
---

# T0454: 指揮役の側の値 (物理の直前の値・当てたリップシンク・パーツの不透明度) を C ABI・wasm・Godot から読む

## 目的

スタジオの指揮役と受け手 (ADR 0028) では、指揮役が `Puppet::values_before_physics` などを受け手へ配る。SDK (`ugoe` の `follow.rs`) には
読む口があるが、C ABI・wasm・Godot には無い (受け手の `follow` だけがある。T0449 で Godot を SDK に揃えたときに分かった)。
組み込み先のエンジンでも指揮役になれるよう、3 つで同じ名前と形で足す。

**判断が要る:** C ABI・wasm・Godot の公開 API の追加。名前 (`get_values_before_physics`・`get_lip_sync_applied`・`get_part_opacities_applied` など) と、
配列の返し方 (C ABI は呼び出し側の領域へ写すか、ポインタと長さか) を、既存の口 (パラメータの値の読み出し) に揃えて決める。

## 受け入れ条件

- [ ] 名前と形を決め、3 つの組み込み先に足した (doc コメント・README)
- [ ] 適合テスト (`tests/conformance`) に、指揮役の値を読んで別の Puppet に `follow` させると参照と一致するケースがある (または既存の `*-follow` のケースを指揮役の側の口で組み直す)
- [ ] `just ci`・`just wasm-check`・`just ffi-check`・`just godot-check`・`just conformance` が通る

## 参考

- T0449 のメモ (起票の候補)
- crates/ugoe/src/follow.rs、ADR 0028

## メモ

- T0019 で起票した (元の候補: T0449 の指揮役の口)。
