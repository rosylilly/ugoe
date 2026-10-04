---
id: T0448
title: "Godot: KTX2 のテクスチャのアルファの扱いを決めて合わせる"
status: todo
milestone: M9
crates: [bindings/godot, docs]
depends_on: [T0427]
assignee:
---

# T0448: Godot: KTX2 のテクスチャのアルファの扱いを決めて合わせる

## 目的

`bindings/godot/src/render.rs` の `load_texture` は、PNG だけを乗算済みアルファに変換し、KTX2 はデコードした画像をそのまま
(乗算済みとみなして) 使っている。`docs/spec/rendering.md#前提` はテクスチャを乗算済みとして扱うと書くが、バンドルの KTX2 が
乗算済みかどうか (取り込み時に変換して入れるのか、読み込み時に変換するのか) は決まっていない (`docs/spec/file-formats.md`)。
ほかの組み込み先 (wasm・C ABI) は KTX2 をデコードしない (白で代用・警告) ので、今は Godot だけが KTX2 を描く。

T0427 のレビューで、古いタスク ID の `TODO(T0406)` を `TODO(T0448)` に付け替えた。

## 受け入れ条件

- [ ] KTX2 のテクスチャのアルファ (乗算済みか、乗算していないか。圧縮形式のときの扱い) を仕様に書く (英語の公開仕様の対応する節も)
- [ ] Godot の `load_texture` を仕様に合わせ (圧縮していない KTX2 を乗算するなど)、`TODO(T0448)` を消す
- [ ] KTX2 のテクスチャを持つバンドルを Godot で描いて確かめる (適合テストのケースか smoke)
- [ ] `just ci` と `node bindings/godot/build.mjs --check` が通る

## 参考

- docs/spec/rendering.md#前提、docs/spec/file-formats.md
- bindings/godot/src/render.rs (`load_texture`)

## メモ

