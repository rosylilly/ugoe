---
id: T0177
title: "spec-check: iCCP と APNG のフレームを持つ PNG の例を足し、描画の例のテクスチャでも PNG の記述を使う"
status: done
milestone: M9
crates: [spec, tools/spec-check, ugoe-format, ugoe-render, docs]
depends_on: []
assignee: claude-worker-T0177
---

# T0177: spec-check: iCCP と APNG のフレームを持つ PNG の例を足し、描画の例のテクスチャでも PNG の記述を使う

## 目的

T0175 で、ファイルの例 (`spec/examples` の `png-*.ugoe`) に PNG の読み方を確かめる例を足し、目録の `png` の記述から spec-check が PNG を作るようにした。
残ったもの (T0175 の候補):

- `iCCP` と APNG のフレーム (`fcTL`・`fdAT`) は、spec/rendering.md#textures で「読み飛ばす」と決めているが、それを確かめる例が無い。
- 描画の例 (`render.json`) のテクスチャは `png` の記述を使えないので、16 ビットやパレットの画像を乗算済みのアルファに変えるところを描画では確かめていない。

## 受け入れ条件

- [x] ファイルの例に `iCCP` を持つ PNG と、APNG のフレームを持つ PNG (既定の画像と違う色のフレーム) の例がある。spec-check の `encodePng` が作り、
      読み手が既定の画像 (`IDAT`) だけを色を変えずに読むことを目録の `rgba` で確かめる
- [x] `iCCP` で色を変換する・APNG のフレームを読む、の誤った読み方を spec-check の読み込みに一時的に入れると落ちることを確かめ、メモに書いた
- [x] 描画の例のテクスチャに `png` の記述を使え、16 ビットとパレット (`tRNS` 付き) の画像を使う描画の例がある。参照の画像を作り直し、
      `crates/ugoe-render/tests/spec_render.rs` と spec-check の両方で一致する
- [x] `crates/ugoe-format/tests/spec_bundles.rs`・`spec_examples.rs` が新しい例を読み、参照実装が満たす (`KNOWN_MISMATCHES` を増やさない)
- [x] spec/examples/README.md・spec/rendering.md#textures (例への参照)・tools/spec-check/README.md を直した
- [x] `just ci` が通る

## 参考

- T0175 のメモ (やったこと・起票の候補)
- spec/rendering.md#textures、docs/spec/rendering.md#前提、tools/spec-check/src/png.ts・render-examples.ts

## メモ

- T0041 で起票した (元の候補: T0175 (`iCCP` と APNG の例)・T0175 (描画の例の `png` の記述))。ugoe-format と ugoe-render はテストだけを触る。

### 作業の結果 (claude-worker-T0177)

やったこと:

- ファイルの例に `png-iccp` (RGB 8 ビット。線形のトーンカーブ (ガンマ 1.0) のプロファイルの `iCCP`) と `png-apng` (RGBA 8 ビット。
  既定の画像はアニメーションに含めず、色の違う 2 つのフレームを `fcTL`・`fdAT` で持つ) を足した。期待する画素は既定の画像のサンプルそのまま。
- 目録の `png` に `icc_profile` (`name`・`gamma`。sRGB の原色 (D50 に順応)・白色点 D50・3 チャンネル同じ `curv` の ICC v2 の
  ディスプレイのプロファイルを deflate で圧縮) と `frames` (APNG のフレームの `samples`。位置 0・遅れ 1/10 秒・破棄と合成 0・番号は 0 から通し) を足した。
  spec-check の `encodePng` が作る (`iccProfile` を足し、走査線の圧縮を `imageData` に切り出した。既存の例のバイトは変わらない)。
- 作った PNG の確かめ: ImageMagick がプロファイルを読み (`icc:description: Linear RGB`)、ガンマ 2.2 のプロファイルへの変換で
  値が変わる (128 → 186) ので正しいプロファイルである。APNG はチャンクの並び・CRC・番号・`fdAT` の中身を Python で確かめた
  (手元の ImageMagick には APNG の読み込みが無い)。
- 誤った読み方を `png.ts` の `decodePng` に一時的に入れて `test/bundles.test.ts` を走らせ、落ちることを確かめた:
  `iCCP` があると色を変換する (線形 → ガンマ 2.2) → `png-iccp` だけが落ちる。APNG の最後のフレームを読む・最初のフレームを読む → `png-apng` だけが落ちる。
- 描画の例のテクスチャ (と法線マップ) に `png` の記述を使えるようにした (`rgba` はそれを読んだ結果)。spec-check は `encodePng` で作った PNG を
  `decodePng` で読んだもので描き、それが `rgba` と同じことを `test/render.test.ts` で確かめる。`render-reference.ts` も `png` の画像を渡す。
- 描画の例 `render-png` を足した: 4×1 のテクスチャを 3 つ (パレット 2 ビットで `PLTE` より短い `tRNS`・RGB 16 ビットで 16 ビットの `tRNS`・
  グレースケールとアルファ 16 ビット)、テクスチャ座標を 1/128 ずらしてテクセルの中心を調べる。期待値は読んだ色 × alpha / 255 を手で求めた実数。
- 参照実装: `crates/ugoe-render/tests/spec_render.rs` が `png` の記述から `png` クレートで PNG を書き、`ugoe_format::image::decode_png` で読んで
  `rgba` と同じことを確かめ、`ugoe_format::image::premultiply` で乗算済みにして描く (そのため `ugoe-render` の dev-dependency の
  `ugoe-format` に `png` の機能を足した。テストだけの変更)。`render-png` を含めて一致し `KNOWN_MISMATCHES` は空のまま。
  `tRNS` を外すと `render-png` の画素で落ちることも確かめた。`spec_bundles.rs`・`spec_examples.rs` は変更なしで新しい例を読み、満たす。
- `render-reference.ts` で参照実装の `ugoe render` と比べ、`render-png` を含めて比べられる画素は食い違いなし。
- 文書: spec/examples/README.md (ファイルの例の表・`png` の欄とチャンクの並び・描画の例の表・`textures` の欄・1 色の規則の例外)、
  spec/rendering.md#textures (例への参照)、docs/spec/rendering.md#前提、docs/testing.md、tools/spec-check/README.md。
- `just spec`・`just ci` が通る。

やらなかったこと:

- 既定の画像がアニメーションの最初のフレームを兼ねる APNG (`IDAT` の前に `fcTL`) の例は作っていない (そのときは最初のフレームを読む誤りを区別できない)。
- `iCCP` の例は 1 つ (線形のプロファイル) だけ。
- 注: spec-check のコードは仕様だけから書き、その後で `docs/spec/rendering.md` と `crates/` のテストを触った。

起票の候補:

- なし。
