---
id: T0473
title: "Unity: PNG のテクスチャも C ABI のデコーダで読み、参照と画素を揃える"
status: done
milestone: M9
crates: [bindings/unity, tests/conformance, docs]
depends_on: []
assignee: claude-worker-T0473
---

# T0473: Unity: PNG のテクスチャも C ABI のデコーダで読み、参照と画素を揃える

## 目的

T0466 で Unity は KTX2 を C ABI の `ugoe_image_decode` で読むようになったが、PNG は今までどおり Unity の `Texture2D.LoadImage` で読む。
16 ビットの PNG・パレット・ガンマのチャンクなどで、参照 (`ugoe_format::image`) と画素が違う可能性が残る (T0466 の候補)。
PNG も C ABI のデコーダで読めば、参照・wasm・Godot と同じ画素になり、乗算済みアルファの変換も C ABI で揃う。

## 受け入れ条件

- [x] `UgoeTextures` が PNG も `EncodedTexture.Decode` で読み、行を入れ替えて渡す。読めなければ今のとおり警告と白 (法線マップは平ら)
- [x] dotnet のテスト (`tests/ImageTests.cs` など) で、PNG の画素が C ABI の結果と一致することを確かめた
- [x] 適合テストの `textures` の比較 (T0466) に PNG のテクスチャを持つケースも入れた (Unity の実行器が書く)
- [x] 実際の Unity で `LoadImage` の場合と見え方を比べる項目を T0443 のメモに書き足した (手元に Unity が無ければ)
- [x] bindings/unity/README.md を直した
- [x] `just unity-check`・`just conformance --only unity`・`just ci` が通る

## 参考

- T0466 のメモ (起票の候補・確かめていないこと)
- bindings/unity/README.md、crates/ugoe-ffi/README.md (「テクスチャのデコード」)

## メモ

- T0036 で起票した (元の候補: T0466 (Unity の PNG))。

### やったこと (2026-10-05、claude-worker-T0473)

- `UgoeTextures` (`Runtime/Unity/UgoeTextures.cs`): PNG も KTX2 と同じ経路 (`EncodedTexture.Decode`・乗算は C ABI・`TextureConversion.FlipRows`・
  `LoadRawTextureData`) で読む。`LoadImage` と C# の乗算の処理を消した。読めなければ今までどおり警告と白 (法線マップは平ら)。
  `TextureConversion.Premultiply`・`PremultiplyRgba32` は公開 API なので残した。
- dotnet のテスト (`bindings/unity/tests/ImageTests.cs`): PNG を手で組み (zlib は `ZLibStream`、CRC は表なしの計算)、RGBA8 (上の行が先・乗算あり/なし・行の入れ替え)、
  16 ビット + `gAMA` 1.0 (上位の 8 ビットを使い、ガンマで値を変えない。ugoe-format の `sixteen_bit_is_reduced_to_eight_bit` と同じ画素)、
  パレット + `tRNS`・グレースケール + アルファの展開を、参照 (`ugoe_format::image`) の決まりの値と比べる。105 件が通る。
- 適合テスト: 参照 (`reference.rs`) と Unity の実行器 (`unity/Program.cs`) が、KTX2 だけでなく **テクスチャを持つすべてのバンドルのケース** で
  `textures` を書くようにした (見本の `avatar-*`・`masks-*`・`dress-up-*` (モジュールのテクスチャを含む)・`bundle-lit-quads` (法線マップ) など)。
  8 ビットの RGBA ではない PNG の生成モデルとケース `png-16bit` (16 ビットの RGBA。下位のバイトは位置で変わる) と `png-palette` (16 色の
  `PLTE` + `tRNS`) を足した (`bundles.rs`・`cases.json`。`png` を ugoe-conformance の依存に足した)。`ModelInstance` から作る `Frame` と
  `evaluate` を比べる単体テスト 2 つは、`textures` を除いて比べるようにした。
- T0443 のメモに、実際の Unity で確かめる項目 (LoadImage の版との見え方・png-16bit / png-palette と参照の画像・読み込みの時間・壊れた PNG) を書き足した。
- 文書: bindings/unity/README.md (テクスチャの節)、tests/conformance/README.md、docs/spec/rendering.md#前提 (Unity は PNG も C ABI で読む一文)。
- **範囲外の小さな変更:** crates/ugoe-ffi/README.md の 1 行 (テストの説明の「KTX2 のケース」を「テクスチャを持つバンドルのケース」に)。

### 確かめたこと

- rebase の後で `just ci`・`just unity-check` (dotnet test 105 件、UnityCompile の netstandard2.1 と net48)・`just conformance`
  (c・wasm・wasm-webgpu・avatar・queries・unity・godot。69 ケース) がすべて通った。WSL2。Unity の実行器の `textures` は
  `png-16bit`・`png-palette`・`avatar-default` などで参照と一致した (`png-16bit` は `masks-nested` と同じ画素)。

### やらなかったこと・確かめていないこと

- **実際の Unity エディタ・プレイヤーでは確かめていない** (手元に Unity が無い)。PNG を `LoadImage` から `LoadRawTextureData` に変えたので、
  上下の向き・色 (RGBA32・`linear: true`)・読み込みの時間とメモリ (`LoadImage` はネイティブ。今は C ABI のデコード + C# での行の入れ替えの
  コピーが 1 回増える) は画面と計測で見ていない。T0443 のメモに項目を書いた。
- `png-16bit` には `gAMA` を書かなかった。書くと (ガンマ 1.0) **Godot の画像が参照と合わない** (87.5% の画素が一致、最大 72/255。
  Godot の PNG の読み込みが `gAMA` で値を変える。`gAMA` を外すと 100% 一致)。Godot は範囲外なので直していない (起票の候補)。
  ガンマを無視することは C ABI について dotnet のテストで確かめた。

### 起票の候補

- Godot: PNG の `gAMA` (と、おそらく `sRGB`・`iCCP`・`cHRM`) で値が変わり、参照 (`ugoe_format::image` は無視する) と画素が違う。
  Godot も C ABI ではなく ugoe-format のデコーダで PNG を読むか、Godot の読み込みの前にガンマのチャンクを取り除く。直したら適合テストの
  `png-16bit` に `gAMA` を戻す (または `gAMA` のケースを足す)。
- 仕様: docs/spec/rendering.md#前提 (と英語の spec/rendering.md) に、PNG の 16 ビット (上位の 8 ビット)・パレットの展開・ガンマや色のチャンクを
  無視して値をそのまま使うことを書く (今は参照の実装の決まりで、仕様には KTX2 の「保存された値のまま」しか書いていない)。
