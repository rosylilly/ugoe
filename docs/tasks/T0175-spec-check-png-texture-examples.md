---
id: T0175
title: "spec-check: PNG のテクスチャの読み方を確かめる例のバンドルを足す"
status: done
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

- [x] 公開仕様の例 (`spec/examples/bundles/` の目録 `bundles.json` と `tools/spec-check/src/make-bundles.ts`) に、パレットの `tRNS`・グレースケールか RGB の `tRNS`・
      16 ビットの `tRNS` (下位を捨てる前に比べる)・`gAMA` (と `sRGB`) 付き・Adam7 のインターレースの PNG をテクスチャに持つバンドルがある
- [x] それぞれの例に、規則どおりに読んだときの期待 (参照の描画結果か、デコードした画素) があり、spec-check が参照実装で通る
- [x] 規則に反する読み方 (ガンマを補正する・16 ビットを丸めてから `tRNS` と比べるなど) をすると spec-check が落ちることを、一度確かめてメモに書いた
- [x] spec/ の例の説明と tools/spec-check/README.md を直した
- [x] `just ci` が通る

## 参考

- T0173 のメモ (起票の候補)
- docs/spec/rendering.md#前提、spec/rendering.md#textures、crates/ugoe-format/src/image.rs のテスト
- tools/spec-check (T0128 の例の書き出し)

## メモ

- T0040 で起票した (元の候補: T0173)。

### 作業の結果 (claude-worker-T0175)

やったこと:

- ファイルの例に `png-palette` (パレット 2 ビット・`PLTE` より短い `tRNS`・番号を広げない)、`png-gray-trns` (グレースケール 4 ビットの `tRNS`。広げる前の値で比べる)、
  `png-rgb-trns`、`png-16bit-trns` (上位のバイト・丸めない・16 ビットで比べる)、`png-gamma` (`gAMA` 1.0 と `cHRM`)、`png-srgb` (`sRGB` と `gAMA`)、
  `png-interlaced` (RGBA の Adam7、7×3 で 3 番目のパスが空)、`png-interlaced-gray` (1 ビットの Adam7) の 8 つを足した。モデルは `single-quad`。
- 目録 `bundles.json` のテクスチャに、PNG の中身の記述 `png` (色の種類・ビット深度・インターレース・`PLTE`・`tRNS`・`gAMA`・`cHRM`・`sRGB`・サンプル) を足し、
  `rgba` を「4 つなら全画素の色、そうでなければ各画素の色」にした。期待する画素は規則から手で書いた値。
- spec-check: `png.ts` に記述から PNG を作る `encodePng` (フィルタ 0〜4 を順に使い IDAT を 2 つに分ける) と、仕様の規則どおりの `decodePng` を足し、
  `decodeRgbaPng` を置き換えた (ファイルの例の照合と `render-reference.ts`)。spec-check のコードは `crates/` を読む前に仕様だけから書いた。
- 作った PNG を libpng (ImageMagick の `convert … -depth 16 txt:`) で読み、上位のバイトが期待値と一致することを確かめた。
- 参照実装: `crates/ugoe-format/tests/spec_bundles.rs` (範囲外。テストだけの小さな変更) が `png` を受け付け、4 つの `rgba` を全画素の色として比べるようにした。
  参照実装の `decode_png` は新しい例をすべて満たす (`KNOWN_MISMATCHES` は空のまま)。`roundtrip.ts` (`ugoe validate`・`ugoe build`) も 54 × 2 で食い違いなし。
- 規則に反する読み方を `png.ts` の読み込みに一時的に入れて `test/bundles.test.ts` を走らせ、落ちることを確かめた:
  `gAMA` で補正する (画面のガンマ 2.2) → `png-gamma`、`sRGB` を線形に変換する → `png-srgb`、16 ビットを丸める → `png-16bit-trns`、
  8 ビットにしてから `tRNS` と比べる → `png-16bit-trns`、広げた値を `tRNS` と比べる → `png-gray-trns` と `png-16bit-trns`、
  `tRNS` より後の番号の alpha を 0 にする → `png-palette`、パレットの番号を広げる → `png-palette`、インターレースを無視する → `png-interlaced`・`png-interlaced-gray`。
- 文書: spec/examples/README.md (表・`textures`・`png` の欄)、spec/rendering.md#textures と docs/spec/rendering.md#前提 (例への参照)、docs/testing.md、tools/spec-check/README.md。

やらなかったこと:

- `iCCP`・APNG (`fcTL`・`fdAT`) の例は作っていない (`iCCP` は圧縮したプロファイルが要る)。参照実装の描画 (`ugoe render`) での確かめはしていない
  (同じ `decode_png` を `spec_bundles.rs` で確かめた)。

起票の候補:

- `iCCP` と APNG のフレームを持つ PNG の例を足す (読み飛ばすことの確かめ)。
- 描画の例 (`render.json`) のテクスチャにも `png` の記述を使えるようにし、乗算済みへの変換を 16 ビットやパレットの画像で確かめる。
