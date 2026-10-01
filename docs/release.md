# リリース

配布物の一覧と方針は [portability.md#配布](portability.md#配布)。すべての配布物で版を揃え、版の正はルートの `Cargo.toml` の
`[workspace.package] version` です。`v<版>` のタグ (例: `v0.1.0`) を打つと、配布物ごとのワークフローが動きます。

| 配布物 | ワークフロー | 手順 |
| --- | --- | --- |
| crates.io のクレート | `.github/workflows/crates-release.yml` | [crates.io](#cratesio) |
| npm の `@ugoe/runtime` | `.github/workflows/web-runtime-release.yml` | [bindings/web/README.md#リリース](../bindings/web/README.md#リリース) |
| エディタとビューアのインストーラ | `.github/workflows/editor-build.yml` | [apps/editor/README.md](../apps/editor/README.md) |
| Unity の UPM パッケージ (tarball) | `.github/workflows/unity.yml` (成果物に残すだけ) | [bindings/unity/README.md#リリース](../bindings/unity/README.md#リリース) |

公開するかどうか、いつ公開するかは、リポジトリを持つ人 (利用者) が決めて行います。

## crates.io

公開する範囲と理由は [ADR 0027](adr/0027-crates-io-publishing.md)。`ugoe` とその依存の 8 つのクレートを、依存の順に公開します。

```sh
node scripts/crates.mjs order
# ugoe-core → ugoe-model → ugoe-runtime → ugoe-format → ugoe-motion → ugoe-physics → ugoe-render → ugoe
```

### 公開の準備の検査

- `just publish-check` (`just ci` に含む): 公開するクレートのメタデータ (`description`・`readme`・`keywords`・`categories` など)、
  各クレートの `LICENSE` がルートの `LICENSE` の写しであること、内部の依存の版が揃っていること。
- `just publish-dry-run`: `cargo publish --workspace --dry-run`。パッケージを作り、crates.io の代わりに一時的なレジストリに置いて
  ビルドします (公開はしません)。crates.io の索引を読むのでネットワークが要り、時間もかかるので `just ci` には含めません。
  パッケージに入るファイルは `cargo package -p <クレート> --list` で確かめられます。

### 版を上げる

1. ルートの `Cargo.toml` の `[workspace.package] version` を変えます。
2. `node scripts/crates.mjs sync` で、`[workspace.dependencies]` の公開するクレートの `version` を揃えます。
3. `node bindings/web/scripts/release.mjs sync` と `node bindings/unity/release.mjs sync` で、npm のパッケージと
   Unity のパッケージの版も揃えます。
4. `cargo update --workspace` で `Cargo.lock` を更新し、コミットします。
5. `v<版>` のタグを打って push します。

### 公開できるようにする (初回だけ)

1. crates.io にログインし、[API トークン](https://crates.io/settings/tokens)を作ります。スコープは `publish-new` と `publish-update`、
   対象のクレートは `ugoe*` に絞ります。
2. GitHub のリポジトリの Settings → Environments で、environment `crates-io` を作ります。
   - **Required reviewers** に、公開を承認する人を加えます (承認されるまで公開のジョブは始まりません)。
   - **Deployment branches and tags** を「Selected branches and tags」にし、タグの規則 `v*` だけを許します。
   - **Environment secrets** に `CARGO_REGISTRY_TOKEN` (1. のトークン) を置きます。リポジトリの secret ではなく
     environment の secret にするのは、承認の前のジョブ (予行) からトークンが見えないようにするためです。
3. 最初の版を公開したら、crates.io のクレートの設定で **trusted publishing** (GitHub Actions の OIDC) に切り替えることもできます。
   切り替えるなら、ワークフローの Publish の前に `rust-lang/crates-io-auth-action` でトークンを得る手順を足し、`id-token: write`
   の権限を publish のジョブに付け、`CARGO_REGISTRY_TOKEN` の secret を消します。

### 公開する

- **タグから (通常):** `v<版>` のタグを push すると、`crates-release.yml` の予行 (`package` ジョブ) が動き、通れば
  `publish` のジョブが承認を待ちます。承認すると、依存の順に公開します。
- **手で起動する:** Actions の画面で `Crates release` を実行します。既定では予行だけです。`publish` を選び、ref にタグを
  選んだときだけ公開します (ブランチからは公開しません)。
- **手元から:** `cargo login` の後、タグをチェックアウトした clean な作業ツリーで `cargo publish --workspace --locked` を実行します。

途中のクレートで失敗したとき (ネットワークなど)、公開済みの版は取り消せません。残りのクレートだけを依存の順に公開します:

```sh
# 例: ugoe-motion まで公開できていたとき
cargo publish --locked -p ugoe-physics -p ugoe-render -p ugoe
```

誤って公開した版は削除できず、`cargo yank --version <版> <クレート>` で新しい依存の解決から外すことだけができます。

### README のリンク

各クレートの `README.md` がそのまま crates.io のページになります。README の中のワークスペースの文書へのリンク
(`../../docs/...`) は、crates.io がパッケージの `path_in_vcs` (`.cargo_vcs_info.json`) と `repository` から GitHub の URL に
直して表示します。`path_in_vcs` は Git のチェックアウトから公開したときにだけ記録されるので、公開はタグのチェックアウトから行います。
