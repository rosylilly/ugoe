---
id: T0105
title: `.ugoe` バンドルの読み書き
status: done
milestone: M1
crates: [ugoe-format]
depends_on: []
assignee: claude-worker-T0105
---

# T0105: `.ugoe` バンドルの読み書き

## 目的

docs/spec/file-formats.md#バンドル-ugoe のチャンク形式を実装する。バイナリのシリアライズ方式を決める。

## 受け入れ条件

- [x] `write_bundle` / `read_bundle` があり、モデル・物理・モーション・テクスチャを往復できるテストがある
- [x] 未知のチャンクを読み飛ばすテストがある
- [x] 壊れた入力 (切り詰め、不正なオフセット) でパニックせずにエラーを返すテストがある
- [x] 採用したシリアライズ方式を仕様書に書く (大きな判断なら ADR も書く)
- [x] wasm32 でビルドできる
- [x] `just ci` が通る

## 参考

- `docs/spec/file-formats.md`

## メモ

- ペイロードは MessagePack (フィールド名付き、`rmp-serde`) にした。postcard は `ugoe-model` の内部タグ付き enum (`Segment` `DeformerKind`) を読めない (`deserialize_any` が必要) ことを実際の往復テストで確認した。経緯は ADR 0008。
- zstd 圧縮 (flags bit0) は純 Rust の `ruzstd` で読み書きとも実装した。書き出しの既定は無圧縮 (`WriteOptions { compress: true }` で圧縮)。`TEXR` は圧縮しない。展開後の上限は 1 チャンク 256 MiB。
- `read_bundle` はスキーマバージョンの検査に加えて `ugoe_model::validate::validate` を通し、失敗したら `FormatError::Invalid` を返す。`model_from_json` は従来どおり検証しない (必要なら揃える)。
- テクスチャの形式は `TEXR` の先頭シグネチャ (PNG / KTX2) で判別する。チャンクに形式の欄は持たない。
- 後続候補: `ugoe-model` の enum を外部タグ付きに変えられるなら postcard に切り替えて小さく速くできる (コンテナの `version` を上げる)。`CompiledModel` のゼロコピー形式は計測してから判断する。
- 後続候補: CLI / エディタからのバンドル書き出し (`ugoe-cli` の `bundle` コマンドなど) と、`ugoe` SDK での `read_bundle` の利用。
