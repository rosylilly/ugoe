---
id: T1003
title: "リポジトリの URL (SECURITY.md・CODE_OF_CONDUCT.md・Issue のテンプレート・README など) が Cargo.toml の repository と食い違わないかを検査する"
status: todo
milestone: Release
crates: [scripts, justfile, docs]
depends_on: []
assignee:
---

# T1003: リポジトリの URL の食い違いの検査

## 目的

T0096 で SECURITY.md・Issue のテンプレートなどに `https://github.com/rosylilly/ugoe` を固定で書いた (`Cargo.toml` の `repository` と同じ)。公開する場所を変えたとき、どこかが古いまま残らないようにする。

## 受け入れ条件

- [ ] ルートの `Cargo.toml` の `workspace.package.repository` (無ければ決めた 1 か所) を正とし、リポジトリの中の `https://github.com/<owner>/<repo>` の形の URL のうち、このリポジトリを指すもの (owner か repo が違うだけのもの) が正と一致するかを検査する軽いスクリプトを足した。ほかのリポジトリへのリンク (依存・出典) は対象にしない (見分け方を決めて書く)
- [ ] 各パッケージ (npm の package.json の `repository`・Unity の package.json・Godot・Tauri の設定など) の URL も対象にした
- [ ] `just ci` (軽い検査の 1 つ) に入れ、単体テストを書いた。わざと食い違えた URL で落ちることを確かめた
- [ ] docs/release.md の公開の前の点検の「リポジトリの URL」の項を、この検査に合わせて直した
- [ ] `just ci` が通る

## 参考

- T0096 のメモ、docs/release.md (公開の前の点検)、scripts/check-local-paths.mjs (同じ種類の軽い検査)

## メモ

- 元の候補: T0096。オーケストレーターが起票した (利用者の指示「後続の候補を起票して着手」、2026-10-12)。
