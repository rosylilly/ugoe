---
id: T0481
title: "Godot: KTX2 のテクスチャも参照のデコーダで読む"
status: done
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

- [x] `bindings/godot/src/render.rs` の KTX2 の読み込みが参照の `decode_ktx2` を使い、`ktx2_as_stored`・`ktx2_is_bc1_rgb` とその単体テストを外した
      (BCn は CPU で展開する。今も乗算のために展開している)
- [x] 適合テストの KTX2 のケース (BC1・BC3・BC7 など) が `just godot-check` で今までどおり通る
- [x] 読み込みの時間とメモリが目に見えて増えないことを、見本の KTX2 で前と後を比べてメモに書いた
- [x] bindings/godot/README.md (描画のしかた) を直した
- [x] `just ci` と `just godot-check` が通る

## 参考

- T0477 のメモ (選んだ方法・起票の候補)
- bindings/godot/src/render.rs、crates/ugoe-format の `decode_ktx2`

## メモ

- T0040 で起票した (元の候補: T0477)。

### やったこと (2026-10-06、claude-worker-T0481)

- `bindings/godot/src/model.rs` の `decode_png` を `decode_image(format, bytes)` にし、PNG も KTX2 も参照の
  `ugoe::format::image::decode_image` (一辺の上限は読み込みのときに見出しで調べてあるので `u32::MAX`) で乗算していない RGBA8 にする。
  `render.rs` は `load_image` で `Image.create_from_data` にし、テクスチャは参照の `ugoe::format::image::premultiply` で乗算する
  (Godot 側の `premultiply` は式が同じなので外した)。`Image.load_ktx_from_buffer`・`decompress`・`clear_mipmaps`・`convert` は使わない。
- 外したもの: `ktx2_as_stored`・`ktx2_is_bc1_rgb` と表 (`KTX2_AS_STORED_FORMATS`・`KTX2_BC1_RGB_FORMATS`・DFD の定数) と単体テスト、
  `premultiply` とその単体テスト。代わりに `ktx2_keeps_stored_values` (`R8G8B8A8_SRGB` の値が変わらない・`BC1_RGB` の 3 色のモードの
  4 つ目の色が不透明で `BC1_RGBA` では透明・Basis Universal (`vkFormat` 0) はエラー) を足した。
- **振る舞いの変化:** 参照が読まない KTX2 (ETC2・ASTC・Basis Universal・`B8G8R8A8_*` など) は、Godot で読めた形式でも警告を出して白で描く
  (法線マップなら使わない)。wasm と同じ。これらは今まで確かめていなかった形式で、仕様の決まり (値をそのまま使う) を満たすかも分からなかった。
- 文書: bindings/godot/README.md (描画のしかた)、docs/spec/rendering.md#前提 (Godot の KTX2 も参照の関数)、`src/shader.rs` の冒頭のコメント。
  英語の公開仕様 (spec/rendering.md) の KTX2 の参考の段はホストを挙げていないので直していない。

### 読み込みの時間とメモリ (前と後)

適合テストが作る見本の KTX2 のバンドル (`avatar-ktx2`・`avatar-lit-ktx2`・`ktx2-mips`・`avatar-bc3`・`avatar-lit-bc`・`bc1-three-color`) を、
リリースビルドの拡張でヘッドレスでない Godot 4.7.1 (Compatibility、llvmpipe、xvfb) に 30 回ずつ `load_model_from_buffer` で読ませ、
前 (main) と後のライブラリを入れ替えながら 3 巡測った (`/usr/bin/time` の最大 RSS も)。ほかのワーカーと同じ機械で負荷が高い (load average 約 25)
ので揺れが大きいが、3 巡の最小値の範囲で前と後に目に見える差は無かった:

| バンドル | 前 (最小、ms) | 後 (最小、ms) | 前 (最大 RSS、MB) | 後 (最大 RSS、MB) |
| --- | --- | --- | --- | --- |
| avatar-ktx2 (`R8G8B8A8_SRGB`) | 10.5〜13.1 | 9.9〜10.3 | 271〜277 | 271〜275 |
| avatar-lit-ktx2 | 12.9〜13.6 | 9.7〜13.2 | 277〜278 | 278〜279 |
| ktx2-mips (zstd・ミップマップ) | 2.4〜3.7 | 3.0〜3.8 | 282〜324 | 279〜343 |
| avatar-bc3 (`BC3_UNORM_BLOCK`) | 12.0〜14.1 | 11.0〜12.4 | 266〜273 | 264〜269 |
| avatar-lit-bc (zstd の `BC7_SRGB_BLOCK`) | 20.4〜22.6 | 17.7〜24.0 | 270〜276 | 267〜269 |
| bc1-three-color (`BC1_RGB_UNORM_BLOCK`) | 2.7〜3.8 | 2.5〜3.8 | 280〜310 | 300〜328 |

(最大 RSS は Godot 全体で、ほとんどが Godot 自身。小さいバンドルでは回ごとの揺れが前後の差より大きい。) BCn の展開は前も
Godot の `Image.decompress` (bcdec) で CPU で行っていたので、展開の仕事は増えていない。

### 確かめたこと

- `just godot-check` (ライセンス・smoke・適合テスト 69 ケース。`ktx2-*` の 6 ケースはすべて 100% 一致) が Compatibility と
  Forward+ (`--renderer forward_plus`、lavapipe) の両方で通った。直す前の main でも `ktx2-*` の 6 ケースが通ることを確かめてから直した。
- rebase の後で `just conformance` (c・wasm・wasm-webgpu・avatar・queries・unity・godot。69 ケース)・`just ci` が通った。
- `cargo clippy --all-targets -D warnings` と `cargo test` (`--manifest-path bindings/godot/Cargo.toml`) が通る。WSL2、Godot 4.7.1。

### 起票の候補

- なし。
