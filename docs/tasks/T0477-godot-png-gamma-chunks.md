---
id: T0477
title: "Godot: PNG のガンマや色のチャンクで画素が参照と変わらないようにし、png-16bit に gAMA を戻す"
status: done
milestone: M9
crates: [bindings/godot, tests/conformance, docs]
depends_on: [T0173]
assignee: claude-worker-T0477
---

# T0477: Godot: PNG のガンマや色のチャンクで画素が参照と変わらないようにし、png-16bit に gAMA を戻す

## 目的

T0473 で、適合テストの `png-16bit` に `gAMA` (ガンマ 1.0) を書くと、Godot の画像が参照と合わなかった (87.5% の画素が一致、最大 72/255)。
Godot の PNG の読み込みが `gAMA` (おそらく `sRGB`・`iCCP`・`cHRM` も) で値を変えるため。参照 (`ugoe_format::image`) はこれらを無視する
(T0173 で仕様に書く)。そこで `gAMA` を外したケースで通している (T0473 の候補)。

## 受け入れ条件

- [x] Godot の PNG のテクスチャが、ガンマや色のチャンクを持つ PNG でも参照と同じ画素になる (C ABI のデコーダで読む・読み込みの前にチャンクを除く、など。選んだ方法と理由をメモと bindings/godot/README.md に書く)
- [x] 適合テストの `png-16bit` に `gAMA` を戻す (または `gAMA`・`sRGB` を持つケースを足す) と、Godot の実行器が参照と一致する
- [x] tests/conformance/README.md のケースの説明を直した
- [x] `just godot-check`・`just conformance`・`just ci` が通る

## 参考

- T0473 のメモ (やらなかったこと・起票の候補)、T0173
- bindings/godot/README.md、tests/conformance/src/bundles.rs・cases.json

## メモ

- T0038 で起票した (元の候補: T0473 (Godot の gAMA))。

### やったこと (2026-10-06、claude-worker-T0477)

- **選んだ方法: PNG を参照と同じデコーダで読む。** `bindings/godot/src/model.rs` の `decode_png` が `ugoe_format::image::decode_png`
  (feature `png` を有効にするため `ugoe-format` を直接の依存に足した) で乗算していない RGBA8 にし、`render.rs` の `decode` が
  `Image.create_from_data` で Godot の画像にする (乗算・法線マップの扱いは今までどおり)。KTX2 は今までどおり Godot で読む。
  - 理由: Godot の `Image.load_png_from_buffer` は libpng の簡易 API (`drivers/png/png_driver_common.cpp`) で、`gAMA` のガンマを
    sRGB に補正する。チャンクを除いてから Godot に読ませる案は、16 ビットの丸め・パレットの展開も libpng の決まりに頼ることになり、
    仕様 (T0173) と同じになることを Godot の版ごとに確かめ続ける必要がある。参照の関数は C ABI・wasm・Unity も使う。
  - 一辺の上限は読み込みのときに見出しで調べてあるので、デコードでは制限しない (`u32::MAX`)。
  - Godot のクレートの依存に png・bcdec_rs など (ルートのワークスペースと同じもの) が増えた。`check-licenses --godot` は通る。
- 単体テスト (`cargo test --manifest-path bindings/godot/Cargo.toml`。dev-dependency に png): 16 ビット + `gAMA` 1.0 + `sRGB` の PNG が
  上位の 8 ビットになる。
- 適合テスト: `png-16bit` にガンマ 1.0 の `gAMA` を戻した (`bundles.rs`)。生成した PNG が `gAMA` を持ち、8 ビットの元の PNG と
  同じ画素にデコードされることを `bundles.rs` のテストで確かめる。**直す前の Godot のコードでは `png-16bit` が失敗する** (87.511%、
  最大 72/255) ことを確かめてから直した。
- 文書: bindings/godot/README.md (描画のしかた・上限の節)、tests/conformance/README.md、docs/spec/rendering.md#前提 (Godot も参照の関数を使う)。
- **範囲外の小さな変更:** 英語の公開仕様 spec/rendering.md の参考の一文 (参照のデコーダを使うホストに Godot を足した。日本語版と同期)。

### 確かめたこと

- rebase の後で `just godot-check` (ライセンス・smoke・適合テスト 69 ケース)・`just conformance` (c・wasm・wasm-webgpu・avatar・queries・
  unity・godot。69 ケース)・`just ci` が通った。WSL2、Godot 4.7.1 (Compatibility、llvmpipe)。`png-16bit` は 100% 一致。
- Forward+ (`--renderer forward_plus`) では流していない (テクスチャの読み込みは描画の方法によらない)。

### 起票の候補

- Godot の KTX2 も参照のデコーダ (`decode_ktx2`) で読むと、`ktx2_as_stored`・`ktx2_is_bc1_rgb` のヘッダの書き換えと、Godot の版ごとの
  挙動の違い (sRGB の変換・BC1 のアルファ) への対処が要らなくなる (BCn は CPU で展開することになる。今も乗算のために展開している)。

