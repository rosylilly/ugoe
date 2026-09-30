---
id: T0403
title: crates.io への公開準備
status: done
milestone: M7
crates: [全クレートの Cargo.toml, CI]
depends_on: []
assignee: claude-worker-T0403
---

# T0403: crates.io への公開準備

## 目的

ランタイム層のクレートを crates.io に公開できる状態にする (メタデータ、ドキュメント、リリースの自動化)。

## 受け入れ条件

- [x] ライセンス (MIT、ADR 0018) の表記と `LICENSE` をパッケージに含める
- [x] 公開するクレートの `publish`・`license`・`description`・`readme` を整える
- [x] `cargo publish --dry-run` が通る
- [x] タグで公開する CI のワークフローがある (手動の承認付き)
- [x] `just ci` が通る

## 参考

- `docs/adr/0017-single-runtime-distribution.md`

## メモ

- 範囲 (ADR 0027): `ugoe` とその依存の 8 クレート (core・model・runtime・format・physics・motion・render・ugoe) だけ公開する。
  `ugoe-ffi` (ビルド済みのライブラリとヘッダで配る)・`ugoe-wasm` (npm で配る。テストがリポジトリの資料を `include_bytes!` する)・
  ツール層は `publish = false` のまま。`[workspace.package]` の `publish = false` を既定に残し、公開するクレートだけ `publish = true`。
- 内部の依存: `[workspace.dependencies]` の公開する 8 つに `version = "0.0.0"` を併記した。版を上げるときは
  `node scripts/crates.mjs sync` で揃える (食い違うと cargo がワークスペースを解決できない)。
- LICENSE: 公開する各クレートに写しを置き、`just publish-check` (`scripts/crates.mjs check` と単体テスト。`just ci` と CI の rust ジョブに追加)
  でルートと同じ内容かを検査する。シンボリックリンクと `license-file` をやめた理由は ADR 0027。
- メタデータ: `publish`・`repository`・`readme`・`keywords`・`categories` だけを足した。`description` は既存の日本語のまま、依存と機能は変えていない。
- `cargo publish --workspace --locked --dry-run` が clean な作業ツリーで通る (8 つのパッケージを作り、一時的なレジストリでビルド)。
  順は ugoe-core → model → runtime → format → motion → physics → render → ugoe (`node scripts/crates.mjs order`)。
- ワークフロー `.github/workflows/crates-release.yml`: `v*` のタグで予行 (版の検査と dry-run) の後、environment `crates-io`
  (required reviewers を設定する) で `cargo publish --workspace --locked`。手で起動したときは予行だけ (タグを ref に選び `publish` を選んだときだけ公開)。
  トークンは environment の secret `CARGO_REGISTRY_TOKEN`。設定の手順と途中で失敗したときのやり直しは docs/release.md#cratesio。
  actionlint (shellcheck 付き) で検査した。
- 確かめていないこと: 実際の公開・crates.io での README の表示 (相対リンクは `path_in_vcs` から GitHub の URL に直される想定。
  パッケージの `.cargo_vcs_info.json` に `path_in_vcs` が入ることは確かめた)・categories が crates.io に受け付けられるか (dry-run では検査されない)・
  GitHub の environment の設定 (利用者が行う)。crates.io の名前 (`ugoe`・`ugoe-*`) は 2026-10-01 の `cargo search` では空いていた。
- パッケージに `tests/` も入る。リポジトリの資料 (`assets/`・`docs/`) を読むテストは、パッケージからは動かない (公開とビルドには影響しない)。
- 後続の候補: crates.io 向けの README (いまの README はエージェント向けの内部の説明。英語の説明と `cargo add ugoe` の使い方)、
  英語の `description`、trusted publishing (OIDC) への切り替え、`ugoe-cli` を `cargo install` で配るか、
  リリースの版上げを 1 コマンドにまとめる (crates と npm の sync)、docs.rs の設定 (`[package.metadata.docs.rs]`)。
