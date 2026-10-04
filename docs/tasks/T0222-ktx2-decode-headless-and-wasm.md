---
id: T0222
title: ヘッドレスと wasm で非圧縮の KTX2 を読み、適合テストに KTX2 のケースを足す
status: done
milestone: M9
crates: [ugoe-format, ugoe-headless, ugoe-wasm, tests/conformance, bindings/godot, docs]
depends_on: []
assignee: claude-worker-T0222
---

# T0222: ヘッドレスと wasm で非圧縮の KTX2 を読み、適合テストに KTX2 のケースを足す

## 目的

T0448 で Godot が KTX2 のテクスチャのアルファを仕様どおり (非乗算として読み、乗算する) に扱うようにした。確かめは Godot の
`bindings/godot/example/tests/ktx2.gd` だけで、参照の実装 (`ugoe render`・wasm) は KTX2 を読めないので、組み込み先の間で比べられない。

- `ugoe-headless` と wasm で、少なくとも圧縮していない KTX2 をデコードする (仕様の扱いで乗算する)。デコーダの置き場所は PNG (T0009 の `ugoe_format::image`) に揃える。
- 適合テスト (`tests/conformance`) に KTX2 のケースを足し、Godot の `ktx2.gd` の確かめをそちらへ移す。
- 書き出し側 (エディタ・`ugoe-format`) は今 KTX2 を書かないことを確かめる。書くなら `KHR_DF_FLAG_ALPHA_PREMULTIPLIED` を立てず非乗算で書くことを検査する。

## 受け入れ条件

- [x] 非圧縮の KTX2 (RGBA8・sRGB) のデコードがあり、上限 (大きさ) を PNG と同じく確かめる。不正なファイルのテストがある
- [x] `ugoe render` と wasm が KTX2 のテクスチャを持つ `.ugoe` を描ける
- [x] 適合テストに KTX2 のケースがあり、C ABI・wasm・Godot で参照と比べる。Godot の `ktx2.gd` を外すか、残す理由を書いた
- [x] 書き出し側の扱いを確かめてメモに書いた
- [x] `docs/spec/rendering.md` (または file-formats.md) と公開仕様の対応する節を、必要なら更新した
- [x] `just ci`・`just wasm-check`・`just conformance`・`just godot-check` が通る

## 参考

- T0448 のメモ (起票の候補)
- crates/ugoe-format/src/image.rs、tests/conformance/README.md

## メモ

- T0019 で起票した (元の候補: T0448 の 2 つ)。

### 完了時 (claude-worker-T0222)

やったこと:

- `ugoe_format::image` (PNG と同じ場所・同じ `ImageLimits`) に KTX2 のデコードを足した: `decode_ktx2` / `decode_ktx2_premultiplied`、
  形式で選ぶ `decode_image` / `decode_image_premultiplied`、識別子 `KTX2_IDENTIFIER`。新しい依存は無い (ヘッダは自前で読み、zstd は
  既に依存している ruzstd)。
  - 読む形式: 圧縮していない `VK_FORMAT_R8G8B8_*` / `VK_FORMAT_R8G8B8A8_*` (`_UNORM`・`_SRGB`) の 2D テクスチャ、超圧縮は無しか zstd。
    値は保存されたまま、レベル 0 だけ、データ形式の記述 (`KHR_DF_FLAG_ALPHA_PREMULTIPLIED`) は見ない (`docs/spec/rendering.md#前提`)。
  - 一辺の上限は PNG と同じく、画素を確保する前に `ImageError::TooLarge` (文面は "PNG image is" → "image is" に変えた)。
  - `ImageError` に `InvalidKtx2` (壊れている: 見出し・識別子・レベルの範囲・大きさの不一致・zstd) と `UnsupportedKtx2` (BCn・Basis・
    配列・キューブマップ・3D・ほかの超圧縮) を足した (`#[non_exhaustive]` なので既存の match は壊れない)。不正なファイルのテストあり。
- `ugoe render` (`ugoe-headless`): バンドルの KTX2 を描く。読めない形式のテクスチャは今までどおりエラー (`--texture` の案内つき)、
  法線マップは警告して使わない。`--texture` にも KTX2 を渡せる (シグネチャで判別)。`Error::PngDecode` を `Error::ImageDecode` に改名。
  適合テスト用の KTX2 の書き出し `image::encode_ktx2` (RGBA8、`_SRGB` / `_UNORM`、ミップマップ、zstd) を足し、
  `KHR_DF_FLAG_ALPHA_PREMULTIPLIED` を立てないことを単体テストで確かめた。依存に ruzstd (ワークスペースに既にある) を足した。
- wasm: テクスチャと法線マップの KTX2 をデコードする。読めない形式だけ今までどおり白 / 法線マップなしで代用して `texture_ktx2` /
  `normal_map_ktx2` の警告。壊れた KTX2 は `LoadError::Texture`、上限超えは `image_limit_exceeded`。
- **範囲外の小さな変更:** `crates/ugoe/src/warning.rs` の `TextureKtx2` / `NormalMapKtx2` の英語の文を「KTX2 in a format that is not
  supported yet」に変えた (KTX2 すべてが未対応ではなくなったため。code は変えていない)。`crates/ugoe-cli/README.md` の KTX2 の記述を直した。
- 適合テスト: 生成するモデルに `avatar-ktx2` / `avatar-lit-ktx2` (見本のテクスチャと法線マップを `R8G8B8A8_SRGB` の KTX2 に) と
  `ktx2-mips` (`nested-masks` のテクスチャを zstd・ミップマップ付きの `R8G8B8A8_UNORM` に。2 段目からはマゼンタ) を足し、ケース
  `ktx2-avatar`・`ktx2-avatar-lit`・`ktx2-mips` を足した。単体テストで、KTX2 のデコード結果が元の PNG と画素ごとに同じことを確かめる
  (手でも `ugoe render` の出力が PNG のバンドルと同じバイト列の画素になることを見た)。C ABI・Unity は頂点だけ、wasm (WebGL2・WebGPU)・
  Godot は画像も比べる。
- Godot の `ktx2.gd` から `rgba8-srgb` / `rgba8-unorm` を外し、`bc3` だけ残した。残す理由: 参照 (`ugoe render`) と wasm は圧縮した
  KTX2 をデコードしないので、適合テストでは BC3 を比べられない。`bindings/godot/README.md`・`build.mjs` のコメントを直した。
- 仕様: `docs/spec/rendering.md#前提` に参照の実装が読む範囲 (圧縮した形式は未対応で、仕様の決まりを満たさないこと) を、
  `docs/spec/file-formats.md#読み込みの上限` に API・上限・読む形式・書き出し側の扱いを書いた。公開仕様は `spec/bundle.md` の
  Informative の注 (参照の実装の画像の上限) を KTX2 にも広げただけ (決まり自体は T0448 で書かれていて変えていない)。
- 確かめた: `just ci`・`just wasm-check`・`just conformance` (60 ケース、c・wasm・wasm-webgpu・avatar・unity・godot すべて ok)・
  `just godot-check` (60 ケースと ktx2.gd の bc3 が ok)。いずれも WSL2 (llvmpipe・SwiftShader)。

- T0434 (wasm の読み込みの失敗のコード) の後に rebase した: 壊れた KTX2 は既存の `invalid_texture`、上限超えは `image_limit_exceeded` に
  なる (テストで確かめた)。読まない形式の KTX2 は失敗でなく警告 (`texture_ktx2`・`normal_map_ktx2`) なので、新しいコードは足していない。
  `invalid_texture` の説明 (`load.rs` の `info` と README の表) を「画像 (PNG・KTX2)」に直した。`packages/player` の訳は形式に触れていないので変えていない。

書き出し側の扱い (確かめた):

- `ugoe-format` の `write_bundle` / `write_module` は KTX2 を作らない。`TextureData` のバイト列をそのまま `TEXR` / `NRML` に書き、
  シグネチャと `format` が合うかだけを検査する (`push_images`)。読んだバンドルの KTX2 は、乗算済みの印も含めて書き戻すだけ。
- エディタ (`ugoe-editor-core`) はテクスチャを PNG で持ち・書き出す (`encode_rgba_png` など)。KTX2 を書く経路は無い
  (`bundle_project_tests.rs` の "ktx" は名前だけ)。バンドルを読み直す `ugoe build` もテクスチャは PNG だけを扱う (KTX2 はエラー。`crates/ugoe-cli/README.md`)。
- KTX2 を書くのは、今回足したテスト用の `ugoe_headless::image::encode_ktx2` だけで、`KHR_DF_FLAG_ALPHA_PREMULTIPLIED` を立てない
  (単体テストで検査)。

やらなかったこと・確かめていないこと:

- ブロック圧縮・Basis Universal などの KTX2 のデコード (参照と wasm)。デコーダの依存が要り、ランタイム層に入るので ADR が要る。
- `B8G8R8A8_*` など、ほかの非圧縮の形式は読まない (Godot も読まない)。
- 実際の KTX2 の道具 (`toktx` など) で作ったファイルでは確かめていない (自前の書き出しと Godot の読み込みで確かめた)。
- Windows・macOS、CI の上では実行していない。

起票の候補:

- 参照 (`ugoe render`) と wasm で、圧縮した KTX2 (BCn・ETC2・ASTC・Basis Universal) をデコードする (依存の選定と ADR)。
  できれば Godot の `ktx2.gd` の BC3 も適合テストへ移す。
- エディタ・ビューア・packages/player の `texture_ktx2` / `normal_map_ktx2` の翻訳文 (「KTX2 の画像で、まだ表示できない」) を、
  「読めない形式の KTX2」に合わせて直す (`apps/editor/locales/*.json`・`packages/player/locales/*.json`)。
- スタジオの仮想カメラ (`apps/studio/src-tauri/src/virtual_camera.rs`) は PNG だけをデコードしている。`ugoe_headless::image::decode_image*`
  で KTX2 も読めるようにする。
