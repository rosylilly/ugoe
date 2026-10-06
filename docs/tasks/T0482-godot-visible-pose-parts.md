---
id: T0482
title: "Godot: UgoePuppet に表示中のポーズの要素を読む関数を足す"
status: todo
milestone: M9
crates: [bindings/godot, docs]
depends_on: []
assignee:
---

# T0482: Godot: UgoePuppet に表示中のポーズの要素を読む関数を足す

## 目的

T0478 で、wasm・C ABI・Unity・`@ugoe/runtime` に表示中のポーズの要素を読む口が入った。Godot の `UgoePuppet` は `show_pose` を持つが、
読む口が無い (T0478 の候補)。ほかのバインディングと揃える。

## 受け入れ条件

- [ ] `UgoePuppet` に表示中の要素を読む関数 (`visible_pose_parts()` など。グループごとに 1 つ、要素の無いグループは空。並びは wasm の `pose_visible_json` と同じ) がある
- [ ] Godot の smoke (か単体テスト) で、`show_pose` の直後とモーションのカーブで切り替わった後の値を確かめる
- [ ] bindings/godot/README.md の API の表を直した
- [ ] `just ci` と `just godot-check` が通る

## 参考

- T0478 のメモ (wasm の `pose_visible_json` の並び・C ABI の口)
- bindings/godot/src (`show_pose`)、docs/spec/motion.md#ポーズ

## メモ

- T0040 で起票した (元の候補: T0478 (Godot))。
