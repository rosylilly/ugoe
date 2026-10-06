---
id: T0481
title: "Godot: KTX2 のテクスチャも参照のデコーダで読む"
status: in-progress
milestone: M9
crates: [bindings/godot, tests/conformance, docs]
depends_on: []
assignee: claude-worker-T0481
---

# T0481: Godot: KTX2 のテクスチャも参照のデコーダで読む

## 目的

T0477 で、Godot のバインディングは PNG を参照のデコーダ (`ugoe_format::image::decode_png`) で読むようにした。KTX2 は今も Godot で読み、
そのために `ktx2_as_stored` (sRGB の変換を避けるヘッダの書き換え)・`ktx2_is_bc1_rgb` (BC1 のアルファの扱い) と、Godot の版ごとの挙動の違いへの対処を持っている
(T0477 の候補)。KTX2 も参照の `decode_ktx2` で RGBA8 にして `Image.create_from_data` に渡し、これらを外す。

## 受け入れ条件

- [ ] `bindings/godot/src/render.rs` の KTX2 の読み込みが参照の `decode_ktx2` を使い、`ktx2_as_stored`・`ktx2_is_bc1_rgb` とその単体テストを外した
      (BCn は CPU で展開する。今も乗算のために展開している)
- [ ] 適合テストの KTX2 のケース (BC1・BC3・BC7 など) が `just godot-check` で今までどおり通る
- [ ] 読み込みの時間とメモリが目に見えて増えないことを、見本の KTX2 で前と後を比べてメモに書いた
- [ ] bindings/godot/README.md (描画のしかた) を直した
- [ ] `just ci` と `just godot-check` が通る

## 参考

- T0477 のメモ (選んだ方法・起票の候補)
- bindings/godot/src/render.rs、crates/ugoe-format の `decode_ktx2`

## メモ

- T0040 で起票した (元の候補: T0477)。
