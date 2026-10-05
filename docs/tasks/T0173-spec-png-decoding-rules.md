---
id: T0173
title: "仕様: PNG のテクスチャの読み方 (16 ビット・パレット・ガンマや色のチャンク) を書く"
status: done
milestone: M9
crates: [docs, spec]
depends_on: []
assignee: claude-worker-T0173
---

# T0173: 仕様: PNG のテクスチャの読み方 (16 ビット・パレット・ガンマや色のチャンク) を書く

## 目的

参照の実装 (`ugoe_format::image`) は、PNG のテクスチャを次のように読む: 16 ビットは上位の 8 ビットを使う・パレット (`PLTE` + `tRNS`) と
グレースケールは RGBA に展開する・`gAMA`・`sRGB`・`iCCP`・`cHRM` などのガンマや色のチャンクは無視して保存された値をそのまま使う。
これは今は実装の決まりで、仕様 (docs/spec/rendering.md#前提) には KTX2 の「保存された値のまま」しか書いていない (T0473 の候補)。
ほかの実装 (Godot の PNG の読み込みなど。T0477) が揃えるべき基準として、仕様に書く。

## 受け入れ条件

- [x] docs/spec/rendering.md#前提 に、PNG の読み方 (ビット深度・カラータイプごとの展開・ガンマや色のチャンクを無視すること・乗算済みアルファにする時点) を書いた
- [x] 英語の公開仕様 spec/rendering.md の対応する節を同じ内容で直した (docs/spec/public-spec.md の対応)
- [x] 書いた規則が `ugoe_format::image` の実装とテスト (`sixteen_bit_is_reduced_to_eight_bit` など) と食い違わないことを確かめ、根拠のテストをメモに書いた
      (食い違えば実装を変えずにメモに書いて止まる)
- [x] `just refs`・`just ci` が通る

## 参考

- T0473 のメモ (起票の候補・`png-16bit`・`png-palette` のケース)
- crates/ugoe-format/src/image.rs、docs/spec/rendering.md、spec/rendering.md、docs/spec/public-spec.md

## メモ

- T0038 で起票した (元の候補: T0473 (仕様))。
- やったこと:
  - docs/spec/rendering.md#前提 に PNG の読み方を書いた: 保存された値のまま (`gAMA`・`sRGB`・`iCCP`・`cHRM` を見ない)・16 ビットは上位の 8 ビット・
    8 ビット未満のグレースケールは `v × 255 / (2^d − 1)` (パレットの番号は広げない)・カラータイプごとの展開・パレットの `tRNS` はアルファ
    (無い番号は 255)・グレースケールと RGB の `tRNS` は元のビット深度の値 (16 ビットなら下位を捨てる前) で比べてアルファ 0・Adam7 は戻す・
    APNG は既定の画像 (`IDAT`) だけ。乗算済みにするのは 8 ビットの RGBA に展開した後 (法線マップは乗算しない)。Godot は今は `gAMA` で
    値が変わることも書いた (T0477)。
  - spec/rendering.md#textures に同じ内容を規範として書いた (乗算の時点は節の先頭の項目、PNG は新しい項目。参照実装の一文は Informative)。
    docs/spec/public-spec.md の対応 (Textures ↔ rendering.md#前提) は変わらないので直していない。
  - 根拠のテスト (`crates/ugoe-format/src/image.rs`。既存: `sixteen_bit_is_reduced_to_eight_bit`・`palette_is_expanded_to_rgba`・
    `grayscale_is_expanded_to_rgba`・`rgb_is_opaque`・`rgba_is_kept_and_premultiplied_on_request`)。仕様に書いた規則のうちテストの無かったものを
    足した (どれも今の実装のままで通った。実装は変えていない): `palette_transparency_becomes_alpha`・`color_key_transparency_becomes_alpha`・
    `sixteen_bit_color_key_is_compared_before_reduction`・`low_bit_depths_are_scaled_or_looked_up`・`gamma_and_color_chunks_are_ignored`
    (ガンマ 1.0 / 0.2 の `gAMA`・`sRGB`・`cHRM`・`iCCP` を手で組んだ PNG に挟んで、値が変わらないこと)・`interlaced_png_is_deinterlaced` (手で組んだ
    2x2 の Adam7)・`apng_uses_only_the_default_image` (既定の画像が枠に含まれる / 含まれない両方)。
  - `crates:` の外 (`ugoe-format`) の変更はテストの追加とモジュールの doc コメント (`tRNS` とガンマのチャンク) だけ。公開 API は変えていない。
  - tools/spec-check の PNG の読み込み (`decodeRgbaPng`) は、参照の描画結果と例の画像 (8 ビット RGBA・インターレースなしだけを受け付ける) を
    読むためのもので、テクスチャのデコードではないので直していない。
- やらなかったこと: Godot の PNG の読み込みを規則に合わせること (T0477 の範囲)。`cICP` など png クレートが読むほかのチャンクも
  「など」に含めて無視としたが、テストは上の 4 つだけ。
- 起票の候補: spec-check (黒箱の適合の検査) に、`tRNS`・16 ビットの `tRNS`・`gAMA` 付き・インターレースの PNG を持つバンドルの例を足し、
  ほかの実装でも上の規則を確かめられるようにする (今は適合テストの `png-16bit`・`png-palette` だけ)。

