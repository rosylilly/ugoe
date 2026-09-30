---
id: T0131
title: 不具合: `ugoe build` が .ugoe を入力にすると法線マップを失って失敗する
status: in-progress
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

- [ ] `ugoe build` が法線マップ付きの .ugoe を入力にして、同じ内容の .ugoe を書ける。assets/compat/dev の全ファイルで往復のテスト
- [ ] `ugoe validate` (と必要なら `build`) が `.ugoemod` を受け付けるかを決め、受け付けるなら実装する。README に書く
- [ ] `just ci` が通る

## 参考

- `docs/tasks/T0128-spec-bundle-examples-writer.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
