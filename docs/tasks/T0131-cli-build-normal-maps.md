---
id: T0131
title: 不具合: `ugoe build` が .ugoe を入力にすると法線マップを失って失敗する
status: done
milestone: M4
crates: [ugoe-cli, ugoe-format, docs]
depends_on: []
assignee: claude-worker-T0131
---

# T0131: 不具合: `ugoe build` が法線マップ付きの .ugoe を作り直せない

## 目的

T0128 で見つかった参照実装の不具合。`ugoe build <in>.ugoe -o <out>.ugoe` が、法線マップのあるバンドルで必ず失敗する。
再現: `ugoe build assets/compat/dev/dress-up-base.ugoe -o /tmp/x.ugoe` → `Error: normal map `body_normal` has no image data`。
`ugoe validate` は同じファイルを ok にするので、.ugoe を入力にしたときに NRML の画像を引き継いでいないとみられる。
あわせて、CLI が `.ugoemod` を読めない (validate が MODU を拒否する) ので、モジュールのファイルを CLI で検査・作り直しできるようにするかを決める。

## 受け入れ条件

- [x] `ugoe build` が法線マップ付きの .ugoe を入力にして、同じ内容の .ugoe を書ける。assets/compat/dev の全ファイルで往復のテスト
- [x] `ugoe validate` (と必要なら `build`) が `.ugoemod` を受け付けるかを決め、受け付けるなら実装する。README に書く
- [x] `just ci` が通る

## 参考

- `docs/tasks/T0128-spec-bundle-examples-writer.md` のメモ

## メモ

- 原因: `run_build` がバンドルをプロジェクトにするとき、テクスチャだけを `Project::textures` に入れ、`NRML` の画像を捨てていた。
  書き出しは法線マップをテクスチャと同じ `textures` から `normal_map` の名前で引くので、必ず失敗していた。
  `project_from_bundle` で法線マップも名前で入れるようにした (`ugoe_editor_core::project` の本体の読み込みと同じやり方)。
  KTX2 の法線マップはテクスチャと同じくエラー。`--atlas` では法線マップのあるテクスチャはまとめずに残る (従来どおり)。
- `.ugoemod` は **受け付ける** ことにした。バンドルと同じコンテナなので中身で判別する (バンドルとして読んで `MODU` があれば
  モジュールとして読み直す)。
  - `ugoe validate <module>.ugoemod`: 単体の検査 (`validate_module`)。`--profile` `--physics` `--module` とは一緒に使えない。
  - `ugoe validate <base>.ugoe --module <m>.ugoemod ...`: `render --module` と同じ規則で付けた結果のモデルと物理を検査する。
    `--physics` は付ける前の本体の物理を差し替える (モジュールのリグはそれに加わる)。
  - `ugoe build <m>.ugoemod -o <out>.ugoemod`: 読み直して書き直す (`--atlas` はエラー)。
  - `info` と `render` の入力にモジュールを渡すと「モジュールなので `--module` で付ける」というエラー。
  - 検証に通らないバンドル・モジュールは、各エラーを `error: ...` で標準エラーに書くようにした (以前は件数だけ)。
- `ugoe build --compress` を足した (互換性の資料は圧縮したものがあり、バイト列まで同じに作り直せるように)。
- テスト (`crates/ugoe-cli/tests/cli.rs`): `build_round_trips_every_compat_file` が assets/compat/dev の全ファイル (バンドル 10・
  モジュール 2) を `--compress` の有無で作り直し、読んだ中身が一致すること、どちらかとはバイト列まで一致することを確かめる。
  `validate_accepts_modules` がモジュールの検査と `--module` を確かめる。
- ファイル形式・互換性の資料・spec/ は変えていない。docs/spec/modules.md#検証 にも CLI のことは書いていない (実装の話なので README だけ)。
- 後続の候補: `ugoe info` でモジュールの概要 (スロット・条件・メッシュ数) を出す。`ugoe build` のアトラス化をモジュールにも。
