---
id: T0448
title: "Godot: KTX2 のテクスチャのアルファの扱いを決めて合わせる"
status: done
milestone: M9
crates: [bindings/godot, docs]
depends_on: [T0427]
assignee: claude-worker-T0448
---

# T0448: Godot: KTX2 のテクスチャのアルファの扱いを決めて合わせる

## 目的

`bindings/godot/src/render.rs` の `load_texture` は、PNG だけを乗算済みアルファに変換し、KTX2 はデコードした画像をそのまま
(乗算済みとみなして) 使っている。`docs/spec/rendering.md#前提` はテクスチャを乗算済みとして扱うと書くが、バンドルの KTX2 が
乗算済みかどうか (取り込み時に変換して入れるのか、読み込み時に変換するのか) は決まっていない (`docs/spec/file-formats.md`)。
ほかの組み込み先 (wasm・C ABI) は KTX2 をデコードしない (白で代用・警告) ので、今は Godot だけが KTX2 を描く。

T0427 のレビューで、古いタスク ID の `TODO(T0406)` を `TODO(T0448)` に付け替えた。

## 受け入れ条件

- [x] KTX2 のテクスチャのアルファ (乗算済みか、乗算していないか。圧縮形式のときの扱い) を仕様に書く (英語の公開仕様の対応する節も)
- [x] Godot の `load_texture` を仕様に合わせ (圧縮していない KTX2 を乗算するなど)、`TODO(T0448)` を消す
- [x] KTX2 のテクスチャを持つバンドルを Godot で描いて確かめる (適合テストのケースか smoke)
- [x] `just ci` と `node bindings/godot/build.mjs --check` が通る

## 参考

- docs/spec/rendering.md#前提、docs/spec/file-formats.md
- bindings/godot/src/render.rs (`load_texture`)

## メモ

### 完了時 (claude-worker-T0448)

やったこと:

- 仕様: KTX2 のテクスチャも PNG と同じく乗算していないアルファと決めた (英語版は `spec/bundle.md` と `spec/rendering.md#textures` で既に
  「テクスチャの画素は非乗算」としていたので、それを KTX2 について明確にした)。`docs/spec/rendering.md#前提`・`docs/spec/file-formats.md`
  (ペイロードのエンコード)・`spec/rendering.md#textures`・`spec/bundle.md#textures-and-normal-maps` に書いた:
  - データ形式の記述の `KHR_DF_FLAG_ALPHA_PREMULTIPLIED` は見ない (書き出しは立てない)。
  - 画素の値は `_SRGB` / `_UNORM`・転送関数によらず保存された値のまま使う (sRGB からリニアに変換しない。PNG と同じ)。
    法線マップも同じ (英語版は既に "not sRGB-decoded")。
  - 圧縮した形式 (BCn・ETC2・ASTC・Basis Universal) は展開した画素を乗算する (圧縮したまま読んで後から乗算するのでは、
    乗算済みの値のバイリニアにならない)。ミップマップは最初の段だけを使う。
- Godot (`bindings/godot/src/render.rs`): KTX2 も RGBA8 にして乗算する。圧縮した形式は `Image.decompress` してから乗算する
  (展開できなければ警告して乗算せずに描く)。ミップマップは捨てる。`TODO(T0448)` を消した。
- Godot 4.7.1 の KTX2 の読み込みは `R8G8B8(A8)_SRGB` を 8 ビットのままリニアに変換し (戻せない)、`R8G8B8(A8)_UNORM` を読めず、
  転送関数が sRGB の Basis Universal をリニアに変換する。仕様どおりに値をそのまま使うため、読む前にヘッダを書き換える
  `ktx2_as_stored` (`src/model.rs`。8 ビットの非圧縮の形式を同じ大きさの `*_UINT` に、データ形式の記述の転送関数を linear に) を足し、
  単体テストを書いた。法線マップも同じ書き換えを通す。
- 描いて確かめる: `example/tests/ktx2.gd` (新規)。見本の `avatar.ugoe` のテクスチャを、同じ画素の KTX2 (`R8G8B8A8_SRGB`・
  `R8G8B8A8_UNORM`・`Image.compress` で作る `BC3_UNORM_BLOCK`) に差し替えたバンドルを描き、PNG のバンドル (BC3 は展開した画素の PNG)
  と画素ごとに比べる (3 つとも 0 画素の違い)。直す前のコードでは `rgba8` が約 52 万画素、`bc3` が読めずに白になって失敗した。
  `build.mjs --conformance` (= `just godot-check` の一部、CI の `godot` ジョブ) から実行する (`--only` のときは飛ばす)。
  適合テストのケースにしなかったのは、参照の `ugoe render` が KTX2 を読めないため。
  GPU での圧縮 (betsy) を使うと Godot が終了の処理で落ちたので、テストでは `compress_with_gpu` を切っている。
- `bindings/godot/Cargo.lock` が main (T0136 で `ugoe-core` から `thiserror` を外した) と合っておらず `build.mjs` の `--locked` で
  ビルドできなかったので、同期した (別のコミット)。
- 確かめた: `node bindings/godot/build.mjs --check` (smoke)、`node bindings/godot/build.mjs --conformance` (適合テスト 53 ケースと
  ktx2.gd)、`cargo test` / `cargo clippy` (bindings/godot)、`just ci`。

やらなかったこと・確かめていないこと:

- Basis Universal (ETC1S・UASTC)・ETC2・ASTC の KTX2 は確かめていない (テストで作る道具が無い)。Basis の転送関数の書き換えは
  Godot のソースを読んで決めた。zstd で超圧縮した KTX2 も確かめていない (libktx が展開するはず)。
- Godot が読めない形式 (`B8G8R8A8_*`、`ETC2_R8G8B8A1` など) は今までどおり白で描く (警告あり)。
- `ugoe render`・wasm・C ABI は KTX2 をデコードしない (白で代用・警告) ままで、変えていない。

起票の候補:

- `ugoe-headless` (`ugoe render`) と wasm で、少なくとも圧縮していない KTX2 をデコードする (仕様の扱いで乗算する)。できれば適合テストに
  KTX2 のケースを足し、Godot の `ktx2.gd` をそちらへ移す。
- 書き出し側 (エディタ・`ugoe-format`) で KTX2 を書くなら、`KHR_DF_FLAG_ALPHA_PREMULTIPLIED` を立てず、非乗算で書くことを検査する。

