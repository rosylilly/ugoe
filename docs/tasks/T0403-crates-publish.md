---
id: T0403
title: crates.io への公開準備
status: todo
milestone: M4
crates: [全クレートの Cargo.toml, CI]
depends_on: []
assignee:
---

# T0403: crates.io への公開準備

## 目的

ランタイム層のクレートを crates.io に公開できる状態にする (メタデータ、ドキュメント、リリースの自動化)。

## 受け入れ条件

- [ ] プロジェクトのライセンスが決まっている (決まっていなければ blocked にする)
- [ ] 公開するクレートの `publish`・`license`・`description`・`readme` を整える
- [ ] `cargo publish --dry-run` が通る
- [ ] タグで公開する CI のワークフローがある (手動の承認付き)
- [ ] `just ci` が通る

## 参考

- `docs/adr/0017-single-runtime-distribution.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
