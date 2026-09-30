---
id: T0122
title: 形式: 読み込みで schema_version を先に確かめる
status: in-progress
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

- [ ] `MODL` / `MODU` の `schema_version` だけを先に読み、新しい版なら `UnsupportedVersion` を返す
- [ ] 形の違う新しい版のモデルを模したファイルを `assets/compat/future/` に足し、`UnsupportedVersion` になるテスト
- [ ] `docs/spec/file-formats.md` を新しい振る舞いに合わせる
- [ ] `just ci`・`just wasm-check` が通る

## 参考

- `docs/tasks/T0116-public-spec-model.md` のメモ
- `spec/bundle.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
