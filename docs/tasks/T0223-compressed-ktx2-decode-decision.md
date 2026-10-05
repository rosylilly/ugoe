---
id: T0223
title: 参照 (ugoe render) と wasm で圧縮した KTX2 をデコードするか決める (依存の選定と ADR。判断が要る)
status: done
milestone: M9
crates: [docs/adr, ugoe-format, ugoe-headless, ugoe-wasm, tests/conformance]
depends_on: []
assignee: claude-worker-T0223
---

# T0223: 参照 (ugoe render) と wasm で圧縮した KTX2 をデコードするか決める (依存の選定と ADR。判断が要る)

## 目的

T0222 で、ヘッドレス (`ugoe render`) と wasm が非圧縮の KTX2 を読めるようにした。圧縮した KTX2 (BCn・ETC2・ASTC・Basis Universal) は
読めない形式として警告し白で描く (T0222 の候補)。Godot の `ktx2.gd` は BC3 を読むので、組み込み先で見え方が違う。

**判断が要る:** デコードに使う依存 (ライセンスは MIT のクレートに入れられるもの。wasm32 でビルドできるか・大きさ) と、対応する形式の範囲。
ランタイム層に重い依存を入れるので ADR を書く (AGENTS.md)。

## 受け入れ条件

- [x] 候補の依存 (ライセンス・wasm32・大きさ・保守) と、対応する形式の範囲を比べた ADR の案を書いた (利用者の判断を待つ間は blocked)
- [x] 判断の後: 決めた形式を参照と wasm でデコードし、適合テストに圧縮した KTX2 のケースがある (できれば Godot の `ktx2.gd` の BC3 も適合テストへ移す)。大きければ実装は別のタスクに分けて、その案をメモに書く
- [x] `just ci`・`just wasm-check`・`just conformance` が通る

## 参考

- T0222・T0448 のメモ、crates/ugoe-format/src/image.rs、tests/conformance/README.md

## メモ

- T0026 で起票した (元の候補: T0222 の圧縮した KTX2)。T0222 の残りの候補 (アプリの訳文・スタジオの仮想カメラ) は T0934・T0836 に入れた。

### 判断待ち (2026-10-05、claude-worker-T0223)

[ADR 0030 (Proposed)](../adr/0030-compressed-ktx2-decoding.md) に候補の比べ合わせと案を書いた。利用者の判断を待つ。

- **判断が要る点:** (1) 対応する形式の範囲 (案 A: BCn だけ / 案 B: + ETC2・EAC・ASTC / 案 C: + Basis Universal)、
  (2) 使う依存、(3) 案 B・C なら wasm の既定に含めるか (cargo の feature で分けるか)。
- **推奨:** 案 A を `bcdec_rs` (MIT、依存なし、wasm で約 14 KB) で。ETC2・ASTC・Basis Universal は需要が出たら別の ADR で。
- 調べたこと (一時ディレクトリで wasm32-unknown-unknown に試しにビルド): `bcdec_rs`・`texture2ddecoder`・`astc-decode`・`basisu` は
  ビルドでき、`basis-universal` (C++) はビルドできない。`texture2ddecoder` は BC3 の色を Direct3D 10 以降の決まりと違って
  (`color0 <= color1` で 3 色のモードに) 読むので、BCn には使わない。`basisu` の出力は道具が無く確かめていない。
- 判断の後の実装の要点は ADR の「決定 (案)」に書いた。案 A なら 1 つのタスク (デコード・仕様・適合テスト・`ktx2.gd` の BC3 の移動) に
  収まる見込み。B・C を採るなら、形式ごとにタスクを分ける (ETC2 / ASTC のテスト用のファイルを作る道具も要る)。

### 利用者の決定 (2026-10-05)

推奨どおり「BCn (BC1〜BC5・BC7) だけを `bcdec_rs` で読む」。ETC2・ASTC・Basis Universal は必要になったら別の ADR で決める。
ADR 0030 を Accepted にした。

### やったこと (2026-10-05、claude-worker-T0223)

- `ugoe_format::image::decode_ktx2` が BC1 (RGB / RGBA)・BC2・BC3・BC4 (UNORM)・BC5 (UNORM)・BC7 (`_UNORM`・`_SRGB`) を
  `bcdec_rs` (MIT、workspace の依存に追加。feature `png` に含めた) で展開する。BC4 は `(R, 0, 0, 255)`、BC5 は `(R, G, 0, 255)`、
  BC1 のアルファなしはアルファ 255。段の大きさはブロック数 × ブロックのバイト数と `checked_mul` で比べ、上限は展開の前に確かめる。
  端の欠けたブロックは画像に入る画素だけを使う。ETC2・ASTC・BC6H・SNORM・Basis は今までどおり `UnsupportedKtx2`。
  単体テスト: 手で組んだブロックの値 (BC3 の色は `color0 <= color1` でも 4 色のモード)・端のブロック・ミップマップと zstd・
  大きさの不一致・上限・乱れたブロックでパニックしないこと。
- `ugoe render` と wasm は `ugoe_format::image` を使うので、そのまま BCn を描く (wasm の単体テストを足した)。
- `ugoe_headless::image::encode_ktx2` に `Ktx2Options::compression` (`Ktx2Compression::{None, Bc1, Bc3, Bc7}`) を足した
  (新しい `bc` モジュール。簡単な圧縮で画質は求めない。BC7 はモード 6 だけ)。
- 適合テスト: 生成するモデル `avatar-bc3` (BC3_UNORM) と `avatar-lit-bc` (zstd の BC7_SRGB のテクスチャ + BC1_RGB_UNORM の法線マップ)、
  ケース `ktx2-bc3-avatar`・`ktx2-bc-avatar-lit` を足した。c・wasm・wasm-webgpu・unity・godot の 65 ケースが通った。
- **範囲外の変更 (小さなもの):**
  - `bindings/godot`: `example/tests/ktx2.gd` (BC3) を消して適合テストへ移した (`build.mjs`・README も)。Godot 4.7 の KTX2 の読み込みは
    `BC*_SRGB_BLOCK` を読めない (texture_loader_ktx.cpp を読んで確かめた) ので、`ktx2_as_stored` で `*_UNORM_BLOCK` に書き換えるようにした
    (これが無いと `ktx2-bc-avatar-lit` が白で描かれて落ちた)。テストを足した。
  - `apps/studio/src-tauri/src/virtual_camera.rs` のテスト (T0836) が「読めない KTX2」に BC3 を使っていたので ETC2 (151) に変えた。
  - `crates/ugoe/src/warning.rs` の doc コメント、`crates/ugoe-cli/README.md`・`crates/ugoe-format/README.md` の記述。
- 仕様: `docs/spec/rendering.md#前提`・`docs/spec/file-formats.md#読み込みの上限`・英語の `spec/rendering.md` (Informative の注) を直した。
- ADR 0030 を Accepted にし、実装の要点 (適合テストの作り方・third-party-notices は JS だけで変えないこと) を実際に合わせた。

### 確かめたこと

- `just ci`・`just wasm-check`・`just licenses`・`just conformance` (c・wasm・wasm-webgpu・avatar・queries・unity・godot。すべて通った)・
  `just godot-check`・bindings/godot の `cargo test`・`clippy`。WSL2 (llvmpipe・SwiftShader)。
- 実際の道具 (toktx・Compressonator など) で作った BC の KTX2 では確かめていない (自前の書き出しと、T0222 までの Godot の `Image.compress`)。

### やらなかったこと・起票の候補

- ETC2・ASTC・Basis Universal のデコード (ADR 0030 で後回し。需要が出たら別の ADR)。
- Unity は KTX2 をまったくデコードしない (白で代用)。C ABI に `ugoe_format::image` のデコードを出せば、Unity も PNG 以外を同じに読める (起票の候補)。
- Godot の BC1 のアルファなしの形式で、3 色のモードの 4 つ目の画素が透明になるはず (参照は不透明)。適合テストの書き出しは 3 色のモードを
  使わないので確かめていない (起票の候補: 3 色のモードを使う BC1 のケースを足し、違えば Godot 側でアルファを 255 にする)。
- エディタ・ビューア・packages/player の `texture_ktx2` の訳文 (「まだ表示できない」) は T0934 の範囲。
