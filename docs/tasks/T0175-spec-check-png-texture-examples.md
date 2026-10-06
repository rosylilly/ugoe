---
id: T0175
title: "spec-check: PNG のテクスチャの読み方を確かめる例のバンドルを足す"
status: in-progress
milestone: M9
crates: [spec, tools/spec-check, docs]
depends_on: []
assignee: claude-worker-T0175
---

# T0175: spec-check: PNG のテクスチャの読み方を確かめる例のバンドルを足す

## 目的

T0173 で、PNG のテクスチャの読み方 (`tRNS`・16 ビット・ガンマと色のチャンクを見ない・インターレース) を docs/spec/rendering.md#前提 と
spec/rendering.md#textures に書いた。根拠のテストは参照実装 (`crates/ugoe-format/src/image.rs`) の単体テストと、適合テストの `png-16bit`・`png-palette` だけで、
ほかの実装が規則を確かめる手段が無い (T0173 の候補)。spec-check (黒箱の適合の検査) の例に、それぞれの規則を確かめるバンドルを足す。

## 受け入れ条件

- [ ] 公開仕様の例 (`spec/examples/bundles/` の目録 `bundles.json` と `tools/spec-check/src/make-bundles.ts`) に、パレットの `tRNS`・グレースケールか RGB の `tRNS`・
      16 ビットの `tRNS` (下位を捨てる前に比べる)・`gAMA` (と `sRGB`) 付き・Adam7 のインターレースの PNG をテクスチャに持つバンドルがある
- [ ] それぞれの例に、規則どおりに読んだときの期待 (参照の描画結果か、デコードした画素) があり、spec-check が参照実装で通る
- [ ] 規則に反する読み方 (ガンマを補正する・16 ビットを丸めてから `tRNS` と比べるなど) をすると spec-check が落ちることを、一度確かめてメモに書いた
- [ ] spec/ の例の説明と tools/spec-check/README.md を直した
- [ ] `just ci` が通る

## 参考

- T0173 のメモ (起票の候補)
- docs/spec/rendering.md#前提、spec/rendering.md#textures、crates/ugoe-format/src/image.rs のテスト
- tools/spec-check (T0128 の例の書き出し)

## メモ

- T0040 で起票した (元の候補: T0173)。
