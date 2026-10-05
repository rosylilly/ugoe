---
id: T0466
title: "Unity: KTX2 のテクスチャを C ABI のデコーダで読む (判断が要る)"
status: done
milestone: M9
crates: [ugoe-ffi, bindings/unity, docs]
depends_on: [T0464]
assignee: claude-worker-T0466
---

# T0466: Unity: KTX2 のテクスチャを C ABI のデコーダで読む (判断が要る)

## 目的

T0223 (ADR 0030) で、参照 (`ugoe render`)・wasm・Godot は圧縮した KTX2 (BC1〜BC5・BC7) を `ugoe_format::image` でデコードするようになった。
Unity は KTX2 をまったくデコードせず、白で代用している (T0223 の候補)。C ABI に `ugoe_format::image` のデコードを出せば、Unity も PNG 以外を同じに読める。

**判断が要る:** C ABI に足す関数の形 (テクスチャの番号でデコード済みの RGBA を返す・バンドルの読み込みでデコードまで済ませる、など) と、
共有ライブラリの大きさへの影響。同じ C ABI と Unity を触る T0464 の後に着手する。

## 受け入れ条件

- [x] C ABI の関数の形の案と推奨をメモに書き、利用者の判断を得た
- [x] Unity が KTX2 (圧縮していないもの・BC1〜BC5・BC7) を参照と同じ画素で読む。読めない形式は今のとおり警告と白にする
- [x] C ABI のテスト (Rust) と Unity のテストがあり、ヘッダ・`just refs`・bindings/unity の README を揃えた
- [x] 適合テスト (`just conformance`) で Unity が使う C ABI の経路の KTX2 のケースを比べた (実際の Unity で確かめられなければメモに)
- [x] `just ci` が通る

## 参考

- T0223 のメモ、ADR 0030
- crates/ugoe-ffi/README.md、bindings/unity/README.md

## メモ

- T0032 で起票した (元の候補: T0223 (Unity))。

### 関数の形 (2026-10-05、claude-worker-T0466)

オーケストレーターの指示 (「ABI の版の規則・T0152・T0223・T0464 から形が自然に決まるなら、根拠をメモに書いて実装する」) に従い、
次の根拠で形が決まると判断して実装した。**受け入れ条件の 1 つ目 (利用者の判断) は得ていないのでチェックしていない。** 形を変えたい場合は、
足しただけの関数なのでこのブランチの中で差し替えられる。

- **形:** バイト列と形式を受け取る単独の関数 `ugoe_image_decode(format, data, len, const UgoeImageLimits *limits, bool premultiply, UgoeImage **out)` と、
  結果の不透明なオブジェクト `UgoeImage` (`ugoe_image_size`・`ugoe_image_pixels`・`ugoe_image_destroy`)、上限 `UgoeImageLimits { max_side }`・
  `ugoe_image_limits_default()`。失敗の種類は `image_limit_exceeded` (`UGOE_ERROR_LIMIT_EXCEEDED`、`width`・`height`・`max`)・
  `unsupported_texture` / `invalid_texture` (`UGOE_ERROR_FORMAT`)。新しいエラーコードは足していない。
- **根拠:**
  - ABI の版の規則 (足すだけ): 既存の `ugoe_puppet_texture` (エンコードされたまま) と読み込みの意味を変えない。`ABI_VERSION` は上げない。
  - T0152 (C ABI は画像をデコードせず、デコードと上限はホストの責務): 読み込みでデコードまで済ませる案は、PNG を自分のエンジンで読む
    ホストにもデコードの時間とメモリを払わせ、読み込みの関数に画像の上限を足すことになるので採らない。上限はホストが関数に渡す
    (`UgoeReadLimits` と同じ「既定値の関数 + NULL なら既定値」の形)。
  - テクスチャの番号でデコードする案は、法線マップ・モジュールのテクスチャに別の関数が要り、`UgoePuppet` に結びつく。バイト列を受け取る形なら
    1 つで足りる (wasm・参照の `decode_image(format, bytes, limits)` と同じ形)。
  - 結果: 大きさを先に知る関数が `ugoe_format::image` に無いので、ホストのバッファに書く形 (`capacity` で聞いて呼び直す) では 2 度デコードする。
    `UgoeModule` と同じ不透明なオブジェクトと破棄の関数にした。
  - T0223 (ADR 0030): デコーダは `ugoe_format::image` (feature `png`) をそのまま使い、参照・wasm と同じ画素になる。
  - T0464: 失敗の種類と補足の値は `ugoe_last_error_kind` と列挙 (`ugoe_last_error_field_*`) でそのまま読める。種類の名前は wasm の
    `image_limit_exceeded`・`invalid_texture` に合わせ、wasm では警告の「読めない KTX2」は `unsupported_texture` とした。
- **共有ライブラリの大きさ:** `png` と `bcdec_rs` が入る。Linux x86_64 の release の `libugoe_ffi.so` で 6,302,904 → 6,496,256 バイト
  (約 189 KiB、3% 増)。cargo の feature で分けてはいない (分けるなら起票の候補)。

### やったこと

- C ABI (`crates/ugoe-ffi/src/image.rs`): 上の関数と型。`ugoe-format` (feature `png`) に直接依存した (`ugoe` は `png` を有効にしない)。
  ヘッダを作り直し、C の見本 (`examples/c/sample.c`) でテクスチャをすべてデコードし、上限の失敗を確かめる。Rust のテスト: PNG (乗算あり・なし)、
  手で組んだ BC1 の KTX2 (上の行が先・SDK と同じ画素)、上限 (種類と補足の値)、壊れた画像・ETC2 の種類、知らない形式・NULL。
- Unity: `EncodedTexture.Decode(premultiply[, UgoeImageLimits])` → `DecodedImage` (上の行が先の RGBA8。`UgoeImage` はすぐ解放)、
  `UgoeImageLimits`、`TextureConversion.FlipRows`、`Native.cs` の 5 つの宣言。`UgoeTextures` は PNG を今までどおり `LoadImage` で、KTX2 を
  C ABI でデコードし (乗算は C ABI で行う)、行を入れ替えて `LoadRawTextureData` で渡す。読めなければ今までどおり警告と白 (法線マップは平ら)。
  テスト `tests/ImageTests.cs` (BC1・非圧縮の KTX2・見本の PNG・種類・上限・行の入れ替え)。`UnityStubs.cs` に `LoadRawTextureData` を足した。
- 適合テスト: KTX2 のテクスチャを持つバンドルのケースで、参照 (`ugoe_format::image`) と Unity の実行器 (`EncodedTexture.Decode`) が
  テクスチャと法線マップを番号ごとにデコードし、`"<幅>x<高さ>:<FNV-1a 64>"` (失敗なら種類) を結果の `textures` に書き、完全に一致することを比べる
  (`frame.rs` の `TextureFrame`・`compare.rs` の `textures`・`reference.rs` の `textures`。書かないバインディングとは比べない)。
  `ktx2-avatar`・`ktx2-avatar-lit`・`ktx2-mips`・`ktx2-bc3-avatar`・`ktx2-bc-avatar-lit` の 5 ケースで一致した。
- 文書: crates/ugoe-ffi/README.md (「テクスチャのデコード (T0466)」の節・形を決めた理由・大きさ、ほかの節の「C ABI はデコードしない」の記述)、
  bindings/unity/README.md、tests/conformance/README.md、docs/spec/rendering.md#前提 (C ABI と Unity の一文)。
- **範囲外の変更 (小さなもの):** `tests/conformance` (受け入れ条件の適合テストのため。`ugoe-format` の feature `png` を明示した)、
  `crates/ugoe-format/README.md` の 1 行 (`image` を共有するクレートに C ABI を足した)。

### 確かめたこと

- rebase の後で `just ci`・`just wasm-check`・`just ffi-check`・`just refs`・`just unity-check` (dotnet test 102 件、UnityCompile の netstandard2.1 と net48)・
  `just conformance` (c・wasm・wasm-webgpu・avatar・queries・unity・godot。66 ケース) がすべて通った。WSL2。

### やらなかったこと・確かめていないこと

- 実際の Unity エディタ・プレイヤーでは確かめていない (手元に Unity が無い)。特に `LoadRawTextureData` に渡す行の向き (Unity の `Texture2D` は
  下の行が先で、`LoadImage` で読んだ PNG と同じ向きになるように行を入れ替えた) と、`Texture2D` の作り方 (RGBA32・`linear: true`) は、
  UV の v を `1 - v` にしている既存のコードと Unity の文書から決めたもので、画面では見ていない。
- PNG は今までどおり Unity の `LoadImage` で読む (C ABI のデコーダに変えていない)。16 ビットの PNG・パレットなどで参照と画素が違う可能性は残る。
- C の適合テストの実行器 (`c/runner.c`) は画像をデコードしない (C ABI の関数そのものは Unity の経路と Rust のテストで確かめた)。

### 起票の候補

- Unity の PNG も C ABI のデコーダで読む (参照と画素を揃える。`LoadImage` との違いを適合テストで比べる)。実際の Unity で見る必要がある。
- 実際の Unity で、BCn の KTX2 のバンドル (適合テストの `avatar-bc3`・`avatar-lit-bc`) の見え方を参照の画像と比べる。
- C ABI の画像のデコードを cargo の feature で外せるようにする (共有ライブラリを約 189 KiB 小さくしたい組み込み先向け。需要が出たら)。
- 判断 (2026-10-05、利用者の判断): 関数の形はこのブランチの `ugoe_image_decode` と不透明な `UgoeImage` のまま入れる。
