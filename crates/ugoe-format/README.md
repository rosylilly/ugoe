# ugoe-format

**層:** L2 入出力 (依存してよいクレートは `scripts/check-layers.mjs` を参照)

`.ugoe` バンドル・`.ugoemod` (着せ替えのモジュール。同じコンテナ) と JSON 形式の読み書き。

## 守ること

- 読み込み時には必ずスキーマのバージョンを検査する
- 壊れた入力でパニックしない (lint と `tests/broken_input.rs`。[docs/portability.md](../../docs/portability.md#パニックしない))
- 外から来る入力で資源を使い切らない。展開の上限は形式でなく読む側の設定 (`ReadLimits`。既定は 1 チャンク 256 MiB・合計 1 GiB)。超えたら展開の途中でやめてエラーにする
- ファイルシステムに触れない (バイト列のスライスを読み、`Vec<u8>` に書く。`std::io` も使わない)
- `no_std` + `alloc` でビルドできる状態を保つ (`just nostd-check`。[docs/portability.md](../../docs/portability.md#ランタイム層が守る制約-移植性のため))
- 画像 (PNG・KTX2) のデコードの上限も読む側の設定 (`ImageLimits`。既定は一辺 16384)。見出しの大きさだけで確保しない
- 知らない必須の機能は拒否する (`features` モジュール)。未知の値 (`Unknown`) は警告にして読み、書き出さない (`ugoe_model::unknown`)
- モーション・表情の数値の検査は `check_motion`・`check_expression` の 1 か所に置く (`FormatError::InvalidMotion`・`InvalidExpression`)。
  JSON・バンドルの読み込みのほか、エディタのプロジェクトの読み込み (`ugoe-editor-core`) も同じ関数を使い、写しを作らない

## MessagePack

チャンクのペイロードの MessagePack は、このクレートの `msgpack` モジュール (非公開) で読み書きする。`rmp-serde` は `std` を前提にするので
置き換えた ([ADR 0031](../../docs/adr/0031-no-std-runtime-and-host-neutral-c-abi.md))。書き出すバイト列と読み込みの結果 (値か誤りか・
入れ子の深さの上限) は `rmp-serde` 1.x の `to_vec_named` と同じで、単体テストが `rmp-serde` (開発時の依存) と `assets/` のファイルで比べる。
失敗は `FormatError::MsgPackEncode` (`MsgPackEncodeError`)・`FormatError::MsgPackDecode` (`MsgPackDecodeError`)。

## feature

- `std` (既定で有効): `std` を使う。外しても公開 API は変わらず、バンドル・モジュール・JSON の読み書き、zstd の展開、
  読み込みの上限 (`ReadLimits`) は同じに働く (`no_std` + `alloc`)。
- `png` (既定で無効。`std` を含む): テクスチャの PNG とブロック圧縮の KTX2 のデコード。`image` モジュールのうち次はこの feature が要る。
  - `decode_png`・`decode_png_premultiplied`・`decode_image`・`decode_image_premultiplied` (無ければ関数が無い)
  - `decode_ktx2`・`decode_ktx2_premultiplied` のブロック圧縮 (BC1〜BC5・BC7) の展開 (無ければ `ImageError::UnsupportedKtx2`)

  feature が無くても、`image` モジュールは圧縮していない 8bit の RGB / RGBA の KTX2 (超圧縮は無しか zstd) を読む
  (`std` の無い環境では、画像のデコードはもともとホストの役目)。

## 画像 (`image`)

テクスチャの PNG と KTX2 のデコード。ヘッドレス描画 (`ugoe-headless`)・WebAssembly (`ugoe-wasm`)・C ABI (`ugoe-ffi` の `ugoe_image_decode`) が共有する
(`docs/spec/file-formats.md#読み込みの上限`)。KTX2 は圧縮していない 8bit の RGB / RGBA と、ブロック圧縮の BC1〜BC5・BC7
(`_UNORM`・`_SRGB`。超圧縮は無しか zstd。BCn の展開は依存の `bcdec_rs`。[ADR 0030](../../docs/adr/0030-compressed-ktx2-decoding.md)) を
保存された値のまま読み、レベル 0 だけを使う。ほかの形式 (ETC2・ASTC・Basis Universal など) は `ImageError::UnsupportedKtx2`
(代用するか誤りにするかは呼ぶ側が決める)。
形式で選ぶときは `decode_image` / `decode_image_premultiplied` (feature `png`)。

```rust
use ugoe_format::image::{ImageLimits, decode_png_premultiplied};

// 8bit RGBA に正規化し (グレースケール・パレット・16bit を展開)、アルファを乗算する。
let texture = decode_png_premultiplied(&bytes, ImageLimits::default())?;
// 上限は読む側が決める。一辺が上限を超えると、画素を確保する前に ImageError::TooLarge。
let small = decode_png_premultiplied(&bytes, ImageLimits::default().with_max_side(4096))?;
```

## 仕様

- [docs/spec/file-formats.md](../../docs/spec/file-formats.md)

## 関連タスク

- T0105 (`docs/tasks/T0105-*.md`)
- T0109 (前方互換: 機能の宣言と未知の値)
- T0113 (着せ替えのモジュールのファイル形式)
- T0115 (互換性テストの資料 `assets/compat/` と `tests/compat.rs`。手順は `docs/testing.md#互換性テスト`)
- T0150 (読み込みの上限 `ReadLimits`。`docs/spec/file-formats.md#読み込みの上限`)
- T0009 (PNG のデコードと上限 `ImageLimits` を ugoe-headless と ugoe-wasm から 1 か所にまとめた `image`)
- T0222 (圧縮していない KTX2 のデコード)
- T0170 (`check_motion`・`check_expression` を公開し、editor-core の写しを置き換えた)
- T0223 (ブロック圧縮 (BC1〜BC5・BC7) の KTX2 のデコード。ADR 0030)
- T0182 (`no_std` + `alloc`。MessagePack を rmp-serde から自前の `msgpack` に替え、PNG と BCn を feature `png` (`std` を含む) の側に置いた。ADR 0031)
