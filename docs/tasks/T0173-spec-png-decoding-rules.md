---
id: T0173
title: "仕様: PNG のテクスチャの読み方 (16 ビット・パレット・ガンマや色のチャンク) を書く"
status: todo
milestone: M9
crates: [docs, spec]
depends_on: []
assignee:
---

# T0173: 仕様: PNG のテクスチャの読み方 (16 ビット・パレット・ガンマや色のチャンク) を書く

## 目的

参照の実装 (`ugoe_format::image`) は、PNG のテクスチャを次のように読む: 16 ビットは上位の 8 ビットを使う・パレット (`PLTE` + `tRNS`) と
グレースケールは RGBA に展開する・`gAMA`・`sRGB`・`iCCP`・`cHRM` などのガンマや色のチャンクは無視して保存された値をそのまま使う。
これは今は実装の決まりで、仕様 (docs/spec/rendering.md#前提) には KTX2 の「保存された値のまま」しか書いていない (T0473 の候補)。
ほかの実装 (Godot の PNG の読み込みなど。T0477) が揃えるべき基準として、仕様に書く。

## 受け入れ条件

- [ ] docs/spec/rendering.md#前提 に、PNG の読み方 (ビット深度・カラータイプごとの展開・ガンマや色のチャンクを無視すること・乗算済みアルファにする時点) を書いた
- [ ] 英語の公開仕様 spec/rendering.md の対応する節を同じ内容で直した (docs/spec/public-spec.md の対応)
- [ ] 書いた規則が `ugoe_format::image` の実装とテスト (`sixteen_bit_is_reduced_to_eight_bit` など) と食い違わないことを確かめ、根拠のテストをメモに書いた
      (食い違えば実装を変えずにメモに書いて止まる)
- [ ] `just refs`・`just ci` が通る

## 参考

- T0473 のメモ (起票の候補・`png-16bit`・`png-palette` のケース)
- crates/ugoe-format/src/image.rs、docs/spec/rendering.md、spec/rendering.md、docs/spec/public-spec.md

## メモ

- T0038 で起票した (元の候補: T0473 (仕様))。
