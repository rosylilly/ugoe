---
id: T0473
title: "Unity: PNG のテクスチャも C ABI のデコーダで読み、参照と画素を揃える"
status: todo
milestone: M9
crates: [bindings/unity, tests/conformance, docs]
depends_on: []
assignee:
---

# T0473: Unity: PNG のテクスチャも C ABI のデコーダで読み、参照と画素を揃える

## 目的

T0466 で Unity は KTX2 を C ABI の `ugoe_image_decode` で読むようになったが、PNG は今までどおり Unity の `Texture2D.LoadImage` で読む。
16 ビットの PNG・パレット・ガンマのチャンクなどで、参照 (`ugoe_format::image`) と画素が違う可能性が残る (T0466 の候補)。
PNG も C ABI のデコーダで読めば、参照・wasm・Godot と同じ画素になり、乗算済みアルファの変換も C ABI で揃う。

## 受け入れ条件

- [ ] `UgoeTextures` が PNG も `EncodedTexture.Decode` で読み、行を入れ替えて渡す。読めなければ今のとおり警告と白 (法線マップは平ら)
- [ ] dotnet のテスト (`tests/ImageTests.cs` など) で、PNG の画素が C ABI の結果と一致することを確かめた
- [ ] 適合テストの `textures` の比較 (T0466) に PNG のテクスチャを持つケースも入れた (Unity の実行器が書く)
- [ ] 実際の Unity で `LoadImage` の場合と見え方を比べる項目を T0443 のメモに書き足した (手元に Unity が無ければ)
- [ ] bindings/unity/README.md を直した
- [ ] `just unity-check`・`just conformance --only unity`・`just ci` が通る

## 参考

- T0466 のメモ (起票の候補・確かめていないこと)
- bindings/unity/README.md、crates/ugoe-ffi/README.md (「テクスチャのデコード」)

## メモ

- T0036 で起票した (元の候補: T0466 (Unity の PNG))。
