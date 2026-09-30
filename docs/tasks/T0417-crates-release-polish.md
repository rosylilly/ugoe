---
id: T0417
title: crates.io: 公開向けの README と英語の description・trusted publishing・版上げの一本化・docs.rs
status: todo
milestone: M7
crates: [ugoe-core, ugoe-model, ugoe-runtime, ugoe-format, ugoe-physics, ugoe-motion, ugoe-render, ugoe, ugoe-cli, scripts, justfile, .github/workflows, docs]
depends_on: [T0403]
assignee:
---

# T0417: crates.io の公開の仕上げ

## 目的

T0403 の後続の候補をまとめる。

## 受け入れ条件

- [ ] crates.io 向けの README (いまの各クレートの README は、リポジトリで作業するエージェント向けの内部の説明)。`readme` を公開向けのファイルに向けるか、README を分ける
- [ ] 英語の description (crates.io の一覧で読まれる)
- [ ] trusted publishing (OIDC) への切り替えを検討し、docs/release.md に書く
- [ ] crates と npm の版上げを 1 コマンドにまとめる (scripts/crates.mjs sync と bindings/web の release.mjs sync)
- [ ] ugoe-cli を `cargo install` で配るかを決める (ADR 0027 の範囲の見直し)
- [ ] docs.rs の設定 (features・GPU の無いビルド)
- [ ] `just ci`・`just publish-dry-run` が通る

## 参考

- `docs/tasks/T0403-crates-publish.md` のメモ、`docs/adr/0027-*.md`、`docs/release.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
