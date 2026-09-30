# 0027: crates.io に公開するクレートの範囲と公開の方法

- 状態: Accepted
- 日付: 2026-10-01
- 関連: T0403、ADR 0017 (単一ランタイムと 1 行導入)、ADR 0018 (ライセンス)、ADR 0025 (Web のランタイムのパッケージ)、[docs/release.md](../release.md#cratesio)

## 背景

ADR 0017 で、Rust 向けの配布物を crates.io の `ugoe` (`cargo add ugoe`) と決めた。`ugoe` は下位のクレート
(`ugoe-core` から `ugoe-render` まで) に依存するので、それらも crates.io に置く必要がある。
一方、ワークスペースには 20 近いクレートがあり、ツール層 (エディタの中核・AI・CLI など) の API はまだ動いている。
公開したクレートは取り消せず (yank しかできない)、名前と版を持ち続けることになるので、公開する範囲を決める。

あわせて、次を決める必要がある。

- ワークスペースの内部の依存は `path` だけで書いており、そのままでは公開できない (crates.io のパッケージには `version` が要る)。
- ライセンスの条文 (`LICENSE`) はリポジトリのルートにしかなく、パッケージに入らない。MIT は再配布に条文を含めることを求める。
- タグで公開する CI の形 (承認・秘密の置き場所・版の検査)。

## 決定

1. **公開するのは、`ugoe` とその依存のクレートだけ** (`ugoe-core`・`ugoe-model`・`ugoe-runtime`・`ugoe-format`・
   `ugoe-physics`・`ugoe-motion`・`ugoe-render`・`ugoe` の 8 つ)。`cargo add ugoe` に必要な最小の集合である。
   - `ugoe-ffi` と `ugoe-wasm` は公開しない。前者はビルド済みの共有ライブラリとヘッダ (C / Unity / Unreal)、後者は npm の
     `@ugoe/runtime` (ADR 0025) として配り、Rust の利用者は `ugoe` を直接使えばよいので、crates.io に置く利点が小さい。
     どちらもバインディングの都合で API が動いている。要望があれば、この範囲を広げる ADR を書く。
   - ツール層 (`ugoe-psd`・`ugoe-mesher`・`ugoe-rigging`・`ugoe-headless`・`ugoe-editor-core`・`ugoe-ai`・`ugoe-cli`・`ugoe-mcp`) と
     アプリは公開しない。`cargo install` で CLI を配るかは、別に決める。
2. **既定は非公開のままにし、公開するクレートだけ `publish = true` を書く。** ルートの `[workspace.package]` の `publish = false`
   を残すので、新しいクレートは明示しない限り公開されない。
3. **公開するクレートの内部の依存は、`[workspace.dependencies]` で `path` と `version` を併記する。** `version` は
   `[workspace.package]` の `version` と同じ値にする (すべての配布物で版を揃える。ADR 0017)。版を上げるときは
   `node scripts/crates.mjs sync` で揃える。公開しないクレートは `path` だけのままにする (dev の依存なら、パッケージを作るときに
   cargo が取り除く)。
4. **`LICENSE` は、公開する各クレートのディレクトリに写しを置く。** `just publish-check` (`just ci` に含む) が、ルートの `LICENSE` と同じ内容かを検査する。
5. **公開は `v*` のタグで動く CI (`.github/workflows/crates-release.yml`) か、利用者が手元で行う。** CI は予行
   (`cargo publish --workspace --dry-run`) を通してから、required reviewers を設定した environment `crates-io` で公開する。
   トークンはその environment の secret `CARGO_REGISTRY_TOKEN` に置く。タグと版が食い違えば公開しない。
6. **公開する範囲とメタデータは `scripts/crates.mjs` で検査する。** `publish = true` のクレートが一覧 (`PUBLISHED`) と一致すること、
   `description`・`readme`・`keywords`・`categories`・`repository`・`license` があること、公開するクレートが公開しないクレートに
   (dev 以外で) 依存しないこと、内部の依存の `version` が揃っていること。

## 検討した代替案

| 案 | 利点 | 欠点 |
| --- | --- | --- |
| ランタイム層をすべて (`ugoe-ffi`・`ugoe-wasm` を含む) 公開する | 範囲の説明が単純 (「ランタイム層」) | 利用者の少ない 2 つのクレートの名前と版を持ち続ける。`ugoe-wasm` はテストがリポジトリの資料を読み、ffi は API が動いている |
| `ugoe` 1 つにまとめて公開する (下位のクレートを内部のモジュールにする) | 公開するクレートが 1 つで済む | ワークスペースの構成と依存方向の検査 (`just layers`) を作り直すことになる。下位のクレート (モデルだけを使うツールなど) を単独で使えない |
| LICENSE をシンボリックリンクにする | 写しの同期が要らない | Windows で `core.symlinks` が無効なチェックアウトでは、リンクの中身 (パス) がパッケージに入る |
| `license-file` でルートの LICENSE を指す | 写しが要らない | `license` (SPDX の式) と併用すると crates.io で「非標準のライセンス」と表示される。`license` を外すと `just licenses` の検査が効かない |
| crates.io の trusted publishing (OIDC) を使う | 長く生きるトークンが要らない | 最初の版はトークンで公開する必要がある。後から切り替えられる (docs/release.md) |

## 結果

- `cargo add ugoe` で導入できる状態になる (実際の公開は利用者が行う)。
- 版を上げるときに、`[workspace.package]` の `version` のほか、`[workspace.dependencies]` の 8 つの `version` も変える
  (`node scripts/crates.mjs sync`)。食い違えば cargo がワークスペースを解決できないので、気づかずに進むことはない。
- 公開したクレートの README は、ワークスペースの中の文書へ相対パスでリンクしている。crates.io はパッケージの
  `.cargo_vcs_info.json` の `path_in_vcs` と `repository` から GitHub の URL に直して表示する。そのためタグ (Git のチェックアウト) から公開する。
- 公開する範囲を広げるときは、この ADR を置き換える ADR を書き、`scripts/crates.mjs` の `PUBLISHED` を直す。
