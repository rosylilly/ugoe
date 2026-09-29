---
id: T0113
title: 着せ替えモジュールのファイル形式
status: done
milestone: M4
crates: [ugoe-format]
depends_on: [T0111]
assignee: claude-worker-T0113
---

# T0113: 着せ替えモジュールのファイル形式

## 目的

モジュールを `.ugoemod` (`MODU` チャンク) として読み書きする。

## 受け入れ条件

- [x] `write_module` / `read_module` があり、往復できる
- [x] 壊れた入力でパニックしない
- [x] 機能 `modules` を宣言する (基準の機能なので `FEAT` には書かない。メモを参照)
- [x] `just ci` が通る

## 参考

- `docs/spec/file-formats.md#機能の宣言と段階的な劣化`
- `docs/spec/modules.md`

## メモ

- API: `ugoe_format::module_file` に `ModuleFile { module, textures, normal_maps, meta }` (`ModuleFile::new`・
  `ModuleFile::normal_map(index)`)、`write_module` / `write_module_with` / `read_module` / `read_module_with_report`。
  報告はバンドルと同じ `ReadReport`。JSON の `module_to_json` / `module_from_json` も足した (エディタのプロジェクト保存用)。
  `ModuleFile::textures` は `Module::textures` の順なので、T0121 は `compose` と同じ順 (本体の後ろ、モジュールの順) に
  テクスチャを並べればよい。
- コンテナはバンドルと共通化した (`bundle.rs` の `ChunkWriter`・`read_container`・`read_texture` / `read_normal_map`・
  テクスチャ数の検査)。バンドルの振る舞いは、`MODU` があれば `UnexpectedChunk` にする点だけ変えた。逆に `.ugoemod` に
  `MODL` `PHYS` `MOTN` `EXPR` があれば `UnexpectedChunk`。取り違えたファイルを黙って読み飛ばさないため。
- 読み込みは `validate_module` を通す (`FormatError::InvalidModule`)。本体との整合は合成が検査する。
- 機能 `modules` の宣言: ADR 0018 により `modules` は 1.0 の基準の機能で、基準の機能は宣言しない
  (`features::FEATURES` と仕様書の表には登録済み)。T0112 の `lighting.normal_map` と同じ判断で、`FEAT` には書かない。
  モジュールのファイルであることは `MODU` の存在で分かり、着せ替えを知らない読み手 (`read_bundle`) は `MODL` がないので読めない。
  1.0 以降の機能のために `features::detect_module` を用意した (今は常に空)。
- 仕様: `docs/spec/file-formats.md` に「モジュール (`.ugoemod`)」の節 (チャンク・書き出し順・読み込み時の検査・API) を足し、
  `modules.md` の「実装待ち」からファイル形式を外した。
- 後続の候補:
  - `ugoe info` (ugoe-cli) で `.ugoemod` を表示する。エディタでモジュールを書き出す。
  - モジュール単位のテクスチャのアトラス化 (T0639 のメモ)。
  - 互換性のコーパス (T0115) に `.ugoemod` を加える。
