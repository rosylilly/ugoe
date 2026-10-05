---
id: T0166
title: 光源の型を GPU に依存しない場所へ移し、C ABI と GDExtension から wgpu を外す (判断が要る)
status: done
milestone: M9
crates: [ugoe-render, ugoe, ugoe-ffi, bindings/godot, docs]
depends_on: []
assignee: claude-worker-T0166
---

# T0166: 光源の型を GPU に依存しない場所へ移し、C ABI と GDExtension から wgpu を外す (判断が要る)

## 目的

T0156 で `ugoe` クレートの参照レンダラを `render` feature に分けたが、光源の型 (`MAX_LIGHTS`・`Light`・`Lighting`、`crates/ugoe-render/src/lighting.rs`) が
`ugoe-render` にあるため、`ugoe-ffi` と `bindings/godot` は `default-features = false` にできず、C ABI の共有ライブラリと GDExtension に wgpu が入る
(T0156 の候補)。光源の型を GPU に依存しない場所へ移せば、Unity・Godot の配布物が小さくなる。

**判断が要る:** 置き場所 (`ugoe-core` か、`ugoe` の `lighting` モジュールか、別の場所か)。`ugoe-render` は再エクスポートで互換を保つ。
ランタイム層の公開 API の移動なので、決めたことを docs/architecture.md か ADR に書く。

## 受け入れ条件

- [x] 置き場所を決め、理由を書いた (利用者の判断を待つなら blocked にしてメモに案を書く)
- [x] 光源の型を移し、`ugoe-render` からは再エクスポートで同じ名前で使える
- [x] `ugoe-ffi` と `bindings/godot` が `ugoe` を `default-features = false` で使い、`cargo tree` に wgpu が出ない
- [x] `just conformance` (C ABI・Godot) と `just ci`・`just wasm-check` が通る

## 参考

- T0156 のメモ (起票の候補)、docs/portability.md

## メモ

- T0026 で起票した (元の候補: T0156)。
- 判断 (2026-10-05、利用者の判断): 置き場所は `ugoe` クレートの `lighting` モジュール。ugoe-render から ugoe へ依存できない (依存の向き) なら ugoe-core に置く (その理由をメモに書く)。ugoe-render は再エクスポートで互換を保つ。
- やったこと (claude-worker-T0166):
  - **置き場所:** `ugoe-core` の `lighting` モジュール (`Light`・`Lighting`・`MAX_LIGHTS`)。`ugoe` は `ugoe::lighting` (既存の
    `uses_lighting`・`bundle_normal_maps` と同じモジュール) で、`ugoe-render` は同じ名前 (`ugoe_render::Lighting` など) で再エクスポートする。
    利用者の判断では `ugoe` の `lighting` だが、`ugoe` が (`render` feature で) `ugoe-render` に依存しているので `ugoe-render` から `ugoe` へは
    依存できず、両方が依存できる最も低い層の `ugoe-core` に置いた (光源の色が `Rgb`。モデルのデータではないので `ugoe-model` には置かない)。
    理由は docs/architecture.md (レイヤーと依存ルール) と crates/ugoe-core/README.md に書いた。シェーダのユニフォームへの変換 (`LightingRaw`) は ugoe-render に残した。
  - ルートの `[workspace.dependencies]` の `ugoe` を `default-features = false` にした (メンバーの `default-features = false` は、
    ワークスペースの依存が既定の feature を有効にしていると効かないため)。参照レンダラを使う `ugoe-wasm`・`ugoe-cli`・`apps/studio` は
    `features = ["render"]` を書く。crates.io の利用者から見た `ugoe` の既定 (`render` 有効) は変えていない。
  - `ugoe-ffi`・`bindings/godot` は `ugoe::lighting` を使い、`cargo tree` に wgpu・naga が出ない。`just sdk-no-render-check` に ugoe-ffi の検査を、
    `bindings/godot/build.mjs --check` に GDExtension の検査を足した (bindings/godot/Cargo.lock から wgpu などが消えた)。
  - テスト: `ugoe-core` の既定値、`ugoe` の `render` 有りで `ugoe::lighting` と `ugoe::render` が同じ型であること。
  - 範囲外の小さな変更: ルートの Cargo.toml・justfile・crates/ugoe-core・crates/ugoe-wasm・crates/ugoe-cli・apps/studio/src-tauri の Cargo.toml、
    docs/spec/rendering.md の `MAX_LIGHTS` の参照。
  - 確かめたこと: `just ci`・`just wasm-check`・`just sdk-no-render-check`・`just conformance` (c・wasm・wasm-webgpu・unity・godot ほか全部 ok)・
    `just godot-check` (smoke と適合 63/63)・bindings/godot の clippy とテスト。
- やらなかったこと: `ugoe-wasm`・`ugoe-headless`・`ugoe-cli`・スタジオの `ugoe::render::{Light, Lighting}`・`ugoe_render::{...}` の参照は
  再エクスポートでそのまま動くので書き換えていない。
- 起票の候補: Unity の配布物 (C ABI の共有ライブラリ) の大きさが減ったかをリリースビルドで測り、docs/portability.md に書く。
