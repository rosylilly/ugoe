# ugoe-format

**層:** L2 入出力 (依存してよいクレートは `scripts/check-layers.mjs` を参照)

`.ugoe` バンドル・`.ugoemod` (着せ替えのモジュール。同じコンテナ) と JSON 形式の読み書き。

## 守ること

- 読み込み時には必ずスキーマのバージョンを検査する
- 壊れた入力でパニックしない
- 外から来る入力で資源を使い切らない。展開の上限は形式でなく読む側の設定 (`ReadLimits`。既定は 1 チャンク 256 MiB・合計 1 GiB)。超えたら展開の途中でやめてエラーにする
- ファイルシステムに触れない (バイト列と `Read`/`Write` を扱う)
- 画像 (PNG・KTX2) のデコードの上限も読む側の設定 (`ImageLimits`。既定は一辺 16384)。見出しの大きさだけで確保しない
- 知らない必須の機能は拒否する (`features` モジュール)。未知の値 (`Unknown`) は警告にして読み、書き出さない (`ugoe_model::unknown`)
- モーション・表情の数値の検査は `check_motion`・`check_expression` の 1 か所に置く (`FormatError::InvalidMotion`・`InvalidExpression`)。
  JSON・バンドルの読み込みのほか、エディタのプロジェクトの読み込み (`ugoe-editor-core`) も同じ関数を使い、写しを作らない

## feature

- `png` (既定で無効): テクスチャの PNG と KTX2 のデコード `image` モジュール。ヘッドレス描画 (`ugoe-headless`) と WebAssembly (`ugoe-wasm`) が共有する
  (`docs/spec/file-formats.md#読み込みの上限`)。KTX2 は圧縮していない 8bit の RGB / RGBA (`_UNORM`・`_SRGB`。超圧縮は無しか zstd) を
  保存された値のまま読み、レベル 0 だけを使う。ほかの形式は `ImageError::UnsupportedKtx2` (代用するか誤りにするかは呼ぶ側が決める)。
  形式で選ぶときは `decode_image` / `decode_image_premultiplied`。

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
