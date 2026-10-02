# リリース

配布物の一覧と方針は [portability.md#配布](portability.md#配布)。すべての配布物で版を揃え、版の正はルートの `Cargo.toml` の
`[workspace.package] version` です。`v<版>` のタグ (例: `v0.1.0`) を打つと、配布物ごとのワークフローが動きます。

| 配布物 | ワークフロー | 手順 |
| --- | --- | --- |
| crates.io のクレート | `.github/workflows/crates-release.yml` | [crates.io](#cratesio) |
| npm の `@ugoe/runtime` | `.github/workflows/web-runtime-release.yml` | [bindings/web/README.md#リリース](../bindings/web/README.md#リリース) |
| エディタ・ビューア・スタジオのインストーラ | `.github/workflows/editor-build.yml` (`just app::build` を OS ごとのランナーで実行) | [apps/editor/README.md#各-os-でのビルド](../apps/editor/README.md#各-os-でのビルド) |
| Unity の UPM パッケージ (tarball) | `.github/workflows/unity.yml` (成果物に残すだけ) | [bindings/unity/README.md#リリース](../bindings/unity/README.md#リリース) |
| スタジオの macOS の仮想カメラ (Camera Extension) | まだ無い (手で作る。配布物に入れるのは Release のマイルストーン) | [macOS の仮想カメラ](#macos-の仮想カメラ) |

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

## macOS の仮想カメラ

スタジオの macOS の仮想カメラ (Camera Extension、[studio.md](studio.md#macos-camera-extension)・ADR 0028 の 8.) を入れたスタジオの、
署名・公証・entitlement の手順です。**いまは配布しません。** 配布物に入れるのは、利用者が Apple Developer Program のアカウントを
用意し、署名の名義を Release のマイルストーンで決めてからです。それまでの配布物は `tauri.conf.json` のまま (拡張も
System Extension の権限も入れない) で作り、スタジオは「この OS ではまだ仮想カメラを出せません」を出します。

この手順は macOS の実機で確かめていません (T0809 は WSL の上で作った)。初めて行うときに、違っていたらここを直してください。

### 用意するもの (初回だけ)

Apple Developer の「Certificates, Identifiers & Profiles」で次を作ります。

| もの | 中身 |
| --- | --- |
| 証明書 | 配布は **Developer ID Application** (開発中は Apple Development)。キーチェーンに入れておく |
| App ID `app.ugoe.studio` (スタジオ) | Capabilities で **System Extension** を有効にする |
| App ID `app.ugoe.studio.camera` (拡張) | Capabilities で **App Groups** を有効にし、下のグループを付ける |
| App Group `<チーム ID>.app.ugoe.studio.camera` | 拡張の `CMIOExtensionMachServiceName` と同じ名前 (Camera Extension の決まり) |
| プロビジョニングプロファイル 2 つ | 上の 2 つの App ID それぞれに、**Developer ID** の種類 (開発中は macOS App Development) で作って落とす |

System Extension の権限 (`com.apple.developer.system-extension.install`) とアプリグループは、プロビジョニングプロファイルが無いと
署名しても起動できない (macOS がアプリを止める) ので、プロファイルは必ず束に入れます。

### entitlement

| 対象 | ファイル | entitlement |
| --- | --- | --- |
| スタジオ | `apps/studio/src-tauri/Entitlements.vcam.plist` | `Entitlements.plist` の 2 つ (カメラ・マイク) と `com.apple.developer.system-extension.install` |
| 拡張 | `apps/studio/vcam/macos/Extension.entitlements` | `com.apple.security.app-sandbox` (Camera Extension はサンドボックスの中で動く決まり) と `com.apple.security.application-groups` (`<チーム ID>.app.ugoe.studio.camera`) |

拡張の `Info.plist` と entitlement の `${TEAM_ID}` は、ビルドのスクリプトが環境変数から埋めます。

### 作る

macOS で、Xcode (またはコマンドラインツール) と、エディタと同じ Rust・Node.js・pnpm を用意してから行います。

```sh
export UGOE_APPLE_TEAM_ID=XXXXXXXXXX                                   # 10 文字のチーム ID
export APPLE_SIGNING_IDENTITY="Developer ID Application: <名義> (XXXXXXXXXX)"
export UGOE_VCAM_PROVISIONING_PROFILE=/path/to/ugoe_studio_camera.provisionprofile
export UGOE_STUDIO_PROVISIONING_PROFILE=/path/to/ugoe_studio.provisionprofile
just app::build TARGET=studio PLATFORM=macos VCAM=1
```

- `VCAM=1` は、`--target universal-apple-darwin` (macOS の既定) に `--config src-tauri/tauri.macos-vcam.conf.json` を重ねて
  `tauri build` を実行します (`scripts/app-build.mjs`。上の 4 つの環境変数が無ければ、何も作らずに止まります)。
  できた `.dmg` は `target/dist/studio/macos/` にも写します。

- `tauri.macos-vcam.conf.json` の `beforeBuildCommand` が `pnpm vcam:macos` (`scripts/build-vcam-macos.mjs`) を呼びます。
  スクリプトは Swift の拡張を arm64 と x86_64 で作って束ね、`target/vcam/macos/app.ugoe.studio.camera.systemextension` に組み、
  プロファイルを `Contents/embedded.provisionprofile` に入れて、拡張の entitlement と Hardened Runtime (`--options runtime`)・
  タイムスタンプ付きで署名します。スタジオのプロファイルは `target/vcam/macos/studio.provisionprofile` に写します。
- tauri は `bundle.macOS.files` で拡張を `Contents/Library/SystemExtensions/` に、プロファイルを `Contents/embedded.provisionprofile`
  に入れ、`APPLE_SIGNING_IDENTITY` と `Entitlements.vcam.plist` でアプリを署名します。拡張は先に署名してあるので、中の署名はそのまま残ります。
- 署名を確かめる:

  ```sh
  APP="target/universal-apple-darwin/release/bundle/macos/ugoe-studio.app"
  codesign --verify --deep --strict --verbose=2 "$APP"
  codesign -d --entitlements - "$APP"
  codesign -d --entitlements - "$APP/Contents/Library/SystemExtensions/app.ugoe.studio.camera.systemextension"
  ```

### 公証する

Developer ID で配るものは公証 (notarization) が要ります。tauri は `APPLE_ID`・`APPLE_PASSWORD` (App 用パスワード)・`APPLE_TEAM_ID`
(または App Store Connect の API キーの `APPLE_API_ISSUER`・`APPLE_API_KEY`・`APPLE_API_KEY_PATH`) が設定されていれば、
`tauri build` の中で公証とステープルまで行います。手で行うときは次のとおりです。

```sh
ditto -c -k --keepParent "$APP" ugoe-studio.zip
xcrun notarytool submit ugoe-studio.zip --apple-id <Apple ID> --team-id "$UGOE_APPLE_TEAM_ID" --password <App 用パスワード> --wait
xcrun stapler staple "$APP"
spctl --assess --type execute --verbose "$APP"
```

`.dmg` で配るときは、`.dmg` も公証してステープルします。

### 利用者の手順 (配布物に入れたとき)

1. アプリを「アプリケーション」フォルダに入れて開く (System Extension はほかの場所のアプリからは有効にできない)。
2. 仮想カメラを入れると macOS が許可を求めるので、「システム設定」で許可し、もう一度入れる
   (操作ウィンドウが案内する。[studio.md の macOS](studio.md#macos-camera-extension))。
3. アンインストールはアプリをゴミ箱に入れる。macOS が拡張も外す (外れるのは再起動の後のことがある)。
