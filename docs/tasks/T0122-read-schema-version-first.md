---
id: T0122
title: 形式: 読み込みで schema_version を先に確かめる
status: done
milestone: M5
crates: [ugoe-format, assets/compat, docs]
depends_on: [T0115, T0116]
assignee: claude-worker-T0122
---

# T0122: 形式: 読み込みで schema_version を先に確かめる

## 目的

`read_bundle` と `read_module` は `MODL` / `MODU` を丸ごとデコードしてから `schema_version` を調べる。形の違う新しい版のモデルを読むと、`UnsupportedVersion` ではなく MessagePack のデコードのエラーになる。
公開仕様 (`spec/bundle.md`) は、読み込み器がまず `schema_version` を確かめること (SHOULD) を求めている (T0116 のメモ)。

## 受け入れ条件

- [x] `MODL` / `MODU` の `schema_version` だけを先に読み、新しい版なら `UnsupportedVersion` を返す
- [x] 形の違う新しい版のモデルを模したファイルを `assets/compat/future/` に足し、`UnsupportedVersion` になるテスト
- [x] `docs/spec/file-formats.md` を新しい振る舞いに合わせる
- [x] `just ci`・`just wasm-check` が通る

## 参考

- `docs/tasks/T0116-public-spec-model.md` のメモ
- `spec/bundle.md`

## メモ

- `bundle::peek_schema_version`: `MODL` / `MODU` の MessagePack のマップのキーを順に見て、`schema_version` の値だけを読む
  (ほかの値は `IgnoredAny` で読み飛ばす。深さの上限 128 は `decode` と同じ)。`rmp_serde` はマップを最後まで読まないと
  `LengthMismatch` にするので、見つけた後も最後まで読み飛ばす (型に当てはめないので安い)。読めなければ `None`。
- `bundle::check_schema_version_first`: 機能の宣言を調べた直後、どのチャンクもデコードする前に、表で最初の `MODL` / `MODU` の
  版を確かめる。展開したペイロードを返して、デコードで使い回す (二度展開しない)。展開や先読みに失敗したらここではエラーにせず、
  今までどおりチャンクを順に読むところでエラーにする。全体をデコードした後の `schema_version` の検査も残した。
  このため、今の版以下のファイルの結果とエラーは変わらない。変わるのは、新しい版のファイルが `UnsupportedVersion` になることだけ
  (表で `MODL` より前にある壊れた `PHYS` などもデコードしない)。
- `assets/compat/future/newer-schema.ugoe` / `.ugoemod` を足した (`schema_version` が先頭でなく、ほかのフィールドの形も違う)。
  先読みを外すとデコードのエラーになることを確かめた。
- `spec/bundle.md` の「Reading」で、Schema version を Chunks の前 (4 番目) に移し、先読みの方法と、読めなかったときの扱いを書いた。
  `docs/spec/file-formats.md` (読み込み時の検査・バージョニング)、`docs/testing.md` (future/ の表)、`assets/compat/README.md` も合わせた。
- JSON (`model_from_json` / `module_from_json`) は今までどおり全体をデコードしてから版を調べる (T0122 の範囲外。必要なら後続で)。
