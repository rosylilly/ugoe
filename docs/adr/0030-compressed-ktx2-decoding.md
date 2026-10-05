# 0030: 参照の実装と wasm で圧縮した KTX2 をデコードする (依存と形式の範囲)

- 状態: Accepted
- 日付: 2026-10-05 (提案と決定)
- 関連: T0223、T0222、T0448、ADR 0013 (前方互換と段階的な劣化)、ADR 0018 (ライセンス)、ADR 0026 (Godot のバインディング)、
  [rendering.md#前提](../spec/rendering.md#前提)、[file-formats.md#読み込みの上限](../spec/file-formats.md#読み込みの上限)

## 背景

バンドルのテクスチャと法線マップは PNG か KTX2 で、KTX2 も乗算していないアルファとして扱う ([rendering.md#前提](../spec/rendering.md#前提))。
T0222 で、参照の実装 (`ugoe render`・`ugoe-headless`) と wasm は `ugoe_format::image::decode_ktx2` で **圧縮していない** 8 ビットの
RGB / RGBA (超圧縮は無しか zstd) を読むようになった。依存は増やしていない (ヘッダは自前で読み、zstd は既にある `ruzstd`)。

圧縮した形式 (BCn・ETC2・ASTC・Basis Universal) は `ImageError::UnsupportedKtx2` で、`ugoe render` はエラー、wasm は白で代用して
警告する (`texture_ktx2`・`normal_map_ktx2`)。一方で Godot (`bindings/godot`) は Godot 自身のデコーダで圧縮した KTX2 も読むので、
組み込み先の間で見え方が違う。Godot の BC3 の確かめは `bindings/godot/example/tests/ktx2.gd` に残り、適合テスト
(`tests/conformance`) では比べられない。Unity は KTX2 をデコードしない (白で代用して警告する)。

エディタと `ugoe-format` の書き出しは KTX2 を作らない (T0222)。圧縮した KTX2 は、外部の道具 (Godot の `Image.compress`・
KTX-Software の `toktx` / `ktx create`・basisu など) で作ったバンドルに入る。

デコーダはランタイム層 (`ugoe-format`) に入るので、AGENTS.md の「ランタイム層に重い依存を入れるなら ADR を書く」に当たる。
条件は次のとおり。

- ライセンスが寛容 (`scripts/check-licenses.mjs` の集合。MIT・Apache-2.0 など) で、MIT のクレートに入れられる。
- **wasm32-unknown-unknown でビルドできる** (C / C++ のコードを含まない。ランタイム層の決まり)。
- wasm の大きさを大きく増やさない。
- 不正な入力でパニックしない、または事前の検査でパニックの条件を除ける (外から来るデータ。AGENTS.md の M9 の誤り)。
- 決定的 (CPU で、同じ入力から同じ画素)。参照の実装なので、GPU のデコーダには任せない。

## 調べたこと

2026-10-05 に crates.io の情報を見て、一時ディレクトリ (リポジトリの外) で wasm32-unknown-unknown 向けに試しにビルドした。
大きさは、各デコーダを呼ぶだけの cdylib を `lto = true`・`opt-level = 3`・`codegen-units = 1` でビルドし、カスタムセクションを除いた
大きさから、何も呼ばない cdylib (約 15.5 KB) を引いたもの。比べるため、今のアプリが同梱する `ugoe_wasm_bg.wasm` は約 8 MB。

| 候補 | 形式 | ライセンス | wasm32 | 増える大きさ (概算) | 保守 (2026-10) | 備考 |
| --- | --- | --- | --- | --- | --- | --- |
| [`bcdec_rs`](https://crates.io/crates/bcdec_rs) 0.2.0 | BC1〜BC7 (BC6H は浮動小数点) | MIT | ビルドできる | 約 14 KB | 最終 2025-01。[image_dds](https://github.com/ScanMountGoat/image_dds) の一部。DL 約 13 万 | [bcdec](https://github.com/iOrange/bcdec) (C) の移植。依存なし。Godot 4 も BCn の展開に bcdec を使う |
| [`texture2ddecoder`](https://crates.io/crates/texture2ddecoder) 0.1.2 | BC1〜BC7・ETC1・ETC2・EAC・ASTC (LDR)・PVRTC・ATC・crunch | MIT OR Apache-2.0 | ビルドできる (no_std) | BCn だけ約 17 KB、+ETC2/EAC 約 36 KB、+ASTC 約 81 KB | 最終 2025-03。DL 約 13 万 | 下の「BC3 の色」の違いがある。依存は `paste` だけ |
| [`astc-decode`](https://crates.io/crates/astc-decode) 0.3.1 | ASTC (LDR・HDR) | Apache-2.0 | ビルドできる | 約 23 KB | 最終 2021-06 (止まっている) | 依存なし |
| [`basisu`](https://crates.io/crates/basisu) 0.1.0 | Basis Universal (ETC1S・UASTC LDR / HDR) を RGBA や BCn / ASTC などへ変換 | Apache-2.0 | ビルドできる (純粋な Rust) | 約 265 KB (zstd なし)、約 355 KB (zstd あり) | 2026-07 に最初の版が出たばかり (版は 1 つ、作者は 1 人) | 参照の C++ の変換器の移植で、同じ出力と称する。ソースは約 68 万行 (多くは表)。`ruzstd` 0.7 に依存し、ワークスペースの 0.9 と二重になる |
| [`basis-universal`](https://crates.io/crates/basis-universal) 0.3.1 | Basis Universal | MIT OR Apache-2.0 | **ビルドできない** (C++ を `cc` でビルドし、wasm32-unknown-unknown では `stdlib.h` が無い) | — | 最終 2023-11 | 公式の C++ のバインディング |

確かめた振る舞い:

- **BC3 の色:** 乱数のブロック 2 万個で `texture2ddecoder` と `bcdec_rs` を比べた。BC7 はすべて一致した。BC1 は補間の丸めで最大 1 違う
  (`bcdec_rs` は `(2a + b + 1) / 3`、`texture2ddecoder` は切り捨て)。BC3 は `texture2ddecoder` が色のブロックを BC1 と同じく
  `color0 <= color1` のとき 3 色のモード (4 つ目は黒) で読み、最大 255 違う。Direct3D 10 以降の決まりでは、BC2 / BC3 の色のブロックは
  常に 4 色のモードで読む ([Microsoft: Texture Block Compression in Direct3D 11](https://learn.microsoft.com/windows/win32/direct3d11/texture-block-compression-in-direct3d-11)、
  [Khronos Data Format Specification の S3TC の節](https://registry.khronos.org/DataFormat/specs/1.3/dataformat.1.3.html#S3TC))。
  `bcdec_rs` はそのとおりに読む。
- **BC1 の透明:** `bcdec_rs::bc1` は 3 色のモードの 4 つ目を透明の黒にする。`VK_FORMAT_BC1_RGB_*` (アルファなし) ではアルファを 255 に
  戻す必要がある。
- **不正な入力:** `texture2ddecoder` は入力や出力の長さが足りないと `Err` を返す。`bcdec_rs` は 1 ブロックの関数だけで、長さが足りないと
  添字でパニックするので、呼ぶ側がレベルのバイト数 (`ceil(w / 4) * ceil(h / 4) * ブロックの大きさ`) を先に確かめる。
- `basisu` の出力が参照の C++ と同じかは、Basis のファイルを作る道具 (basisu・toktx) が手元に無く確かめていない。

ほかに考えて採らなかった案:

- **GPU にそのまま渡す** (wgpu の `TEXTURE_COMPRESSION_BC` など): 対応はアダプタ次第で (WebGPU のモバイルは BC を持たない)、
  GPU のデコーダの丸めは実装ごとに違うので参照にならない。乗算済みへの変換 (rendering.md#前提) もシェーダで行うことになる。
- **自前で書く:** BC1〜BC5 は小さいが、BC7・ETC2・ASTC・Basis は大きく、誤りの元になる。

## 決めること (利用者の判断)

1. **対応する形式の範囲**
   - 案 A: BCn (BC1〜BC5・BC7。BC6H は HDR なので対象外) だけ。
   - 案 B: 案 A + ETC2 / EAC + ASTC (LDR)。
   - 案 C: 案 B + Basis Universal。
2. **使う依存**
   - BCn: `bcdec_rs` (推奨) か `texture2ddecoder`。
   - ETC2 / ASTC (案 B 以上): `texture2ddecoder` の ETC / ASTC の関数 (BCn は使わない)。ASTC は `astc-decode` もあるが保守が止まっている。
   - Basis Universal (案 C): `basisu` (純粋な Rust) しか wasm32 で使えない。
3. **wasm での既定** (案 B・C のとき): 増える形式を cargo の feature に分け、wasm の既定に含めるか。

## 決定

利用者の決定 (2026-10-05): **案 A を `bcdec_rs` で入れる。ETC2・ASTC・Basis Universal は今は入れず、需要が出たら別の ADR で決める。**

理由 (提案のときの推奨のまま):

- 小さい (wasm で約 14 KB)・MIT・依存なし・wasm32 でビルドできる。
- Godot が展開に使う bcdec の移植なので、Godot と画素が揃いやすい。`ktx2.gd` の BC3 を適合テストへ移せる (T0222 の残り)。
- BCn はデスクトップの GPU 向けの形式で、組み込み先 (Godot・Unity のデスクトップ) で作られやすい。
- ETC2・ASTC はモバイル向けで、今の組み込み先と道具の流れ (エディタは PNG を書く) から入ってくる見込みが低い。
  `texture2ddecoder` は BC3 の読み方が決まりと違うので、BCn には使わない。
- Basis Universal は glTF (KHR_texture_basisu) では一般的だが、`basisu` は出たばかりで (版が 1 つ・作者が 1 人)、wasm で
  約 265〜355 KB 増え、`ruzstd` が二重になる。成熟を待つ。入れるときは cargo の feature に分け、出力を参照の C++ と比べるテストを持つ。

実装の要点 (T0223):

- 置き場所は `ugoe_format::image` (今の `png` feature の中。PNG・非圧縮の KTX2 と同じ `ImageLimits`)。`decode_ktx2` が
  `VK_FORMAT_BC1_RGB_*`・`BC1_RGBA_*`・`BC2_*`・`BC3_*`・`BC4_*`・`BC5_*`・`BC7_*` (`_UNORM`・`_SRGB`。BC4 / BC5 の `_SNORM` は対象外)
  を読み、展開した画素を RGBA8 にする (BC4 は `(R, 0, 0, 255)`、BC5 は `(R, G, 0, 255)`。GPU が 1・2 チャンネルの形式を読むときと同じ)。値は転送関数によらず保存されたまま使う (rendering.md#前提)。
- 確保の前に、一辺の上限と、レベルのバイト数がブロックの数と合うか (`checked_mul`) を確かめる。幅・高さが 4 の倍数でない端のブロックは
  一時の 4x4 に展開して切り取る。超圧縮 (zstd) を外してから展開する。
- 仕様: rendering.md#前提 と file-formats.md#読み込みの上限 の「参照がデコードする形式」に BCn を足し、英語の公開仕様 (`spec/`) も直す。
  読まない形式 (ETC2・ASTC・Basis など) の扱い (エラー・白で代用して警告) は今のまま。
- 適合テスト: `ugoe_headless::image::encode_ktx2` にテスト用の簡単な BC1・BC3・BC7 の書き出し (`Ktx2Compression`。外部の依存を
  足さない) を足し、`ktx2-bc*` のケースで wasm・Godot を参照と比べる。`ktx2.gd` の BC3 はそちらへ移す。
- `scripts/check-licenses.mjs` の集合で通る (MIT)。アプリの `third-party-notices.txt` は JavaScript の同梱物の表記で、Rust のクレートは載せていないので変えない。

## 検討した代替案

| 案 | 利点 | 欠点 |
| --- | --- | --- |
| A: BCn だけ (`bcdec_rs`) | 小さい (約 14 KB)・MIT・依存なし。Godot (bcdec) と揃う。`ktx2.gd` の BC3 を適合テストへ移せる | ETC2・ASTC・Basis Universal の KTX2 は今までどおり白 / エラー |
| B: A + ETC2 / EAC + ASTC (`texture2ddecoder`) | モバイル向けの形式も参照で描ける | 約 80 KB 増える。今は入ってくる見込みが低い。テスト用のファイルを作る道具が要る |
| C: B + Basis Universal (`basisu`) | glTF でよく使う KTX2 も描ける | 約 265〜355 KB 増える。依存が出たばかり (版が 1 つ・作者が 1 人)。`ruzstd` が二重になる |
| BCn を `texture2ddecoder` で | 1 つの依存で B まで広げられる | BC3 の色を Direct3D 10 以降の決まりと違って読む |
| Basis を `basis-universal` (C++) で | 公式の実装 | wasm32-unknown-unknown でビルドできない |
| GPU にそのまま渡す | 依存が要らない | アダプタ次第で、デコードの丸めも実装ごとに違い、参照にならない |
| 何もしない (今のまま) | 依存が増えない | 組み込み先 (Godot) と参照の見え方の違いが残り、BC3 を適合テストで比べられない |

## 結果

- BCn の KTX2 を参照 (`ugoe render`)・wasm・Godot で同じに描け、適合テストで比べられる。wasm は約 14 KB 増える。
- ETC2・ASTC・Basis Universal の KTX2 は今までどおり、`ugoe render` はエラー、wasm は白で代用して警告する。
- 見直すきっかけ: ETC2・ASTC・Basis Universal の KTX2 を描きたい需要が出たとき、`basisu` が成熟したとき (版が重なり、出力を参照の
  C++ と比べられる道具が揃ったとき)、`bcdec_rs` の保守が止まって不具合が見つかったとき。
