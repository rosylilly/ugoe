---
id: T0009
title: PNG のデコードと大きさの上限を 1 か所にまとめる (ugoe-headless と ugoe-wasm)
status: done
milestone: M9
crates: [ugoe-format, ugoe-headless, ugoe-wasm, docs]
depends_on: []
assignee: claude-worker-T0009
---

# T0009: PNG のデコードと大きさの上限を 1 か所にまとめる (ugoe-headless と ugoe-wasm)

## 目的

M9 のコードレビューの横断の所見 (T0212・T0424。まとめは docs/workflow/code-review.md#m9-の結果)。
`crates/ugoe-wasm/src/load.rs` の `decode_png`・`decode_png_premultiplied` は `crates/ugoe-headless/src/image.rs` の写しで、
「見出しの大きさだけで数 GB を確保する」誤りが両方にあり、T0212 と T0424 で別々に直した (一辺の上限 16384 も 2 か所に定数がある)。
次に直すときも片方だけになりやすい。

ランタイム層で使えるクレート (`ugoe-format` は既に `png` に依存し、バイト列の処理はランタイム層の規則の例外) に、
上限つきのデコード (色の正規化・乗算済みアルファの有無) を 1 つ置き、両方から使う。

## 受け入れ条件

- [x] PNG のデコード (8bit RGBA への正規化・乗算済みアルファ・一辺の上限) を 1 か所に置き、`ugoe-headless` と `ugoe-wasm` がそれを使う
- [x] 置き場所が公開 API になるなら doc コメントと README を書き、上限を差し替えて試せる形 (T0138 の `payload_within` のように) にする
- [x] 見出しが巨大な PNG・グレースケール・パレット・16bit の既存のテストが、まとめた側で通る
- [x] `just ci` と `just wasm-check` が通る

## 参考

- docs/workflow/code-review.md#m9-の結果
- T0212・T0424 のメモ

## メモ

### やったこと

- `ugoe_format::image` (feature `png`、既定で無効) に、上限つきの PNG のデコードを 1 つ置いた:
  `decode_png(bytes, ImageLimits)` (ストレート)・`decode_png_premultiplied(bytes, ImageLimits)`・`premultiply`・`Rgba8`・`ImageError { TooLarge, Decode }`。
  上限は T0150 の方針どおり読む側の設定で、`ReadLimits` / `ImportLimits` と同じ形 (`#[non_exhaustive]`・`Default`・`with_max_side`)。
  既定は `DEFAULT_MAX_IMAGE_SIDE` = 16384 (これまでの 2 か所の定数と同じ)。超える画像は画素を確保する前に `TooLarge`。
- `ugoe-headless`: `image::decode_png` / `decode_png_premultiplied` / `premultiply` / `Rgba8` は共有の関数・型を使う
  (関数の形は変えないので、スタジオ・適合テストは変更なし)。`Error::PngDecode` は `ImageError` をそのまま包む形に変えた (`#[error(transparent)]`)。
  既定の上限が出力の `MAX_SIZE` と同じことをテストで確かめる。
- `ugoe-wasm`: `load::Rgba8` は共有の型、`MAX_IMAGE_SIZE` は `DEFAULT_MAX_IMAGE_SIDE` の別名。`Rgba8::placeholder()` は
  外の型に inherent の関数を足せないので `load::placeholder()` にした。`png` は dev-dependencies に移した (テストの PNG の作成だけ)。
- テスト (`ugoe-format` の `image::tests`): 見出しが巨大な PNG・上限の差し替え (ちょうど上限は読め、1 つ超えるとエラー。高さだけ超える場合も)・
  グレースケール (8bit・アルファ付き・1bit)・パレット・16bit・RGB・壊れた入力・乗算の丸め。グレースケール・パレット・16bit のテストは
  これまで無かったので新しく書いた。wasm の巨大な見出しのテストは読み込みの経路の確認として残した。
- 文書: `docs/spec/file-formats.md#読み込みの上限` に PNG のデコードの上限を足し、公開仕様 `spec/bundle.md` の Resource limits に informative の段落を足した。
  `ugoe-format`・`ugoe-headless`・`ugoe-wasm` の README を更新。
- 範囲の外の小さな変更: `scripts/ci-groups.mjs` の runtime グループの features に `ugoe-format/png` を足した
  (`-p` で選ぶグループでは headless・wasm が feature を有効にしないので、足さないと `image` のテストが CI で走らない)。

### やらなかったこと

- headless・wasm の呼び出し側 (CLI・MCP・Web) から上限を変える口は作っていない (既定値で読む)。SDK・wasm から上限を指定する口は T0152 の範囲。
- `ugoe-editor-core/src/texture.rs` と `ugoe-format/tests/spec_bundles.rs` にも 8bit RGBA への展開の写しがある (前者は上限なし)。範囲の外なので触っていない。

### 起票の候補

- `ugoe-editor-core::texture` の PNG のデコードを `ugoe_format::image` に寄せる (上限が無い写し。エディタは外から来た PNG を読む)。
- `ugoe-format/tests/spec_bundles.rs` のテスト用のデコーダを `ugoe_format::image` に置き換える (dev-dependency で自分の `png` feature を有効にする)。

### 検査

- `just ci` が通った (rebase の後)。`just wasm-check`・`just publish-check` も通った。

