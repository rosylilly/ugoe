---
id: T1003
title: "リポジトリの URL (SECURITY.md・CODE_OF_CONDUCT.md・Issue のテンプレート・README など) が Cargo.toml の repository と食い違わないかを検査する"
status: done
milestone: Release
crates: [scripts, justfile, docs]
depends_on: []
assignee: claude-worker-T1003
---

# T1003: リポジトリの URL の食い違いの検査

## 目的

T0096 で SECURITY.md・Issue のテンプレートなどに `https://github.com/rosylilly/ugoe` を固定で書いた (`Cargo.toml` の `repository` と同じ)。公開する場所を変えたとき、どこかが古いまま残らないようにする。

## 受け入れ条件

- [x] ルートの `Cargo.toml` の `workspace.package.repository` (無ければ決めた 1 か所) を正とし、リポジトリの中の `https://github.com/<owner>/<repo>` の形の URL のうち、このリポジトリを指すもの (owner か repo が違うだけのもの) が正と一致するかを検査する軽いスクリプトを足した。ほかのリポジトリへのリンク (依存・出典) は対象にしない (見分け方を決めて書く)
- [x] 各パッケージ (npm の package.json の `repository`・Unity の package.json・Godot・Tauri の設定など) の URL も対象にした
- [x] `just ci` (軽い検査の 1 つ) に入れ、単体テストを書いた。わざと食い違えた URL で落ちることを確かめた
- [x] docs/release.md の公開の前の点検の「リポジトリの URL」の項を、この検査に合わせて直した
- [x] `just ci` が通る

## 参考

- T0096 のメモ、docs/release.md (公開の前の点検)、scripts/check-local-paths.mjs (同じ種類の軽い検査)

## メモ

- 元の候補: T0096。オーケストレーターが起票した (利用者の指示「後続の候補を起票して着手」、2026-10-12)。
- 完了 (claude-worker-T1003、2026-10-12):
  - `scripts/check-repo-url.mjs` と単体テスト (`scripts/check-repo-url.test.mjs`) を足し、`just repo-url` として `just ci` と CI の
    `rust-checks` (`.github/workflows/ci.yml`。`crates:` の外だが 1 段の追加なのでここで行った) に入れた。正はルートの `Cargo.toml` の
    `[workspace.package]` の `repository`。
  - 見分け方: GitHub の owner と repo の組 (`github.com/`・`github.com:` (SSH)・`raw.githubusercontent.com/`・npm の `github:`・`gh` の
    `--repo`) を拾い、owner か repo の片方だけが正と同じものを誤りにする。両方が違うものはほかのリポジトリとして見ない。
    正と同じ名前を持つほかのリポジトリは `OTHER_REPOS` に足す (今は空)。パッケージの設定の `repository` (package.json と各 Cargo.toml
    の `repository = "…"`) は自身の置き場なので、`"o/r"` の短い形も含め、両方が違っても誤りにする。
  - 確かめたこと: `.github/ISSUE_TEMPLATE/config.yml` の owner と Unity の package.json の repo をわざと変えると、その 2 か所で落ちる。
    `Cargo.toml` の owner を変えると、SECURITY.md・CODE_OF_CONDUCT.md・Issue のテンプレート・README・Unity の package.json・
    Godot の Cargo.toml・エディタの Tauri の capabilities・MCPB の manifest (`crates/ugoe-mcp/src/mcpb.rs`)・JSON Schema の `$id`・
    cliff.toml など 40 ほどのファイルで落ちる (古いタスクファイルの CI の実行へのリンクも含む)。
  - docs/release.md の「自動の検査」に項を足し、「人がすること」の「リポジトリの URL」を `Cargo.toml` を直してから検査に従う形にした。
    docs/testing.md の表と `rust-checks` の説明にも足した。
  - やらなかったこと: owner と repo を一度に両方変えたときの古い URL は拾えない (文書とスクリプトの説明に、片方ずつ変えるか
    `git grep` で探すと書いた)。pre-commit (lefthook) には入れていない (全ファイルを見る検査で、1 つのファイルでは食い違いを見逃さないため
    `just ci` と CI で足りると判断)。`@ugoe/runtime` (bindings/web/package.json) には `repository` が無い (npm に出すときに要るかは
    T1003 の範囲の外)。
  - 起票の候補: npm に出す `@ugoe/runtime` の package.json に `repository`・`homepage` を足すか (公開の準備の検査で要求するか) を決める。
