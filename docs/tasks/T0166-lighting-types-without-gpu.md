---
id: T0166
title: 光源の型を GPU に依存しない場所へ移し、C ABI と GDExtension から wgpu を外す (判断が要る)
status: todo
milestone: M9
crates: [ugoe-render, ugoe, ugoe-ffi, bindings/godot, docs]
depends_on: []
assignee:
---

# T0166: 光源の型を GPU に依存しない場所へ移し、C ABI と GDExtension から wgpu を外す (判断が要る)

## 目的

T0156 で `ugoe` クレートの参照レンダラを `render` feature に分けたが、光源の型 (`MAX_LIGHTS`・`Light`・`Lighting`、`crates/ugoe-render/src/lighting.rs`) が
`ugoe-render` にあるため、`ugoe-ffi` と `bindings/godot` は `default-features = false` にできず、C ABI の共有ライブラリと GDExtension に wgpu が入る
(T0156 の候補)。光源の型を GPU に依存しない場所へ移せば、Unity・Godot の配布物が小さくなる。

**判断が要る:** 置き場所 (`ugoe-core` か、`ugoe` の `lighting` モジュールか、別の場所か)。`ugoe-render` は再エクスポートで互換を保つ。
ランタイム層の公開 API の移動なので、決めたことを docs/architecture.md か ADR に書く。

## 受け入れ条件

- [ ] 置き場所を決め、理由を書いた (利用者の判断を待つなら blocked にしてメモに案を書く)
- [ ] 光源の型を移し、`ugoe-render` からは再エクスポートで同じ名前で使える
- [ ] `ugoe-ffi` と `bindings/godot` が `ugoe` を `default-features = false` で使い、`cargo tree` に wgpu が出ない
- [ ] `just conformance` (C ABI・Godot) と `just ci`・`just wasm-check` が通る

## 参考

- T0156 のメモ (起票の候補)、docs/portability.md

## メモ

- T0026 で起票した (元の候補: T0156)。
