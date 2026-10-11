# リリース

配布物の一覧と方針は [portability.md#配布](portability.md#配布)。すべての配布物で版を揃え、版の正はルートの `Cargo.toml` の
`[workspace.package] version` です。`v<版>` のタグ (例: `v0.1.0`) を打つと、配布物ごとのワークフローが動きます。

| 配布物 | ワークフロー | 手順 |
| --- | --- | --- |
| crates.io のクレート | `.github/workflows/crates-release.yml` | [crates.io](#cratesio) |
| npm の `@ugoe/runtime` | `.github/workflows/web-runtime-release.yml` | [bindings/web/README.md#リリース](../bindings/web/README.md#リリース) |
| エディタ・ビューア・スタジオのインストーラ | `.github/workflows/editor-build.yml` (`just app::build` を OS ごとのランナーで実行。Secrets があれば署名し、タグでは [下書きの Release](#github-release-の下書き) に添付) | [apps/editor/README.md#各-os-でのビルド](../apps/editor/README.md#各-os-でのビルド)・[デスクトップアプリの署名](#デスクトップアプリの署名) |
| Unity の UPM パッケージ (tarball) | `.github/workflows/unity.yml` (成果物に残すだけ) | [bindings/unity/README.md#リリース](../bindings/unity/README.md#リリース) |
| スタジオの macOS の仮想カメラ (Camera Extension) | まだ無い (手で作る。配布物に入れるのは Release のマイルストーン) | [macOS の仮想カメラ](#macos-の仮想カメラ) |

公開するかどうか、いつ公開するかは、リポジトリを持つ人 (利用者) が決めて行います。1.0 (v1.0.0) は、[1.0 の範囲](roadmap.md#10-の範囲) (M1〜M11 の完了。
[ADR 0035](adr/0035-one-point-zero-scope-m1-to-m11.md)) を終えてから、Release のマイルストーンで公開します。

## 同梱物のライセンスの表記

エディタ・ビューア・スタジオは、同梱する第三者のもの (MediaPipe の wasm とモデル、バンドルに入る JS のライブラリ) のライセンスの
表記を `apps/<アプリ>/public/third-party-notices.txt` に持ち、アプリの「ライセンス」の画面で表示します (T0926)。この 3 つのファイルは
手で書かず、`scripts/third-party-notices.mjs` が次の 2 つから生成します。

- MediaPipe の節: `packages/face-tracking/third-party-notices.txt` の最初の節から終わりまで (Apache-2.0 の条文を含む)。
- JS のライブラリの節: アプリを書き出さずに `vite build` し、バンドルに入ったモジュールの node_modules のパッケージごとに、
  名前・版・ライセンス・ソースと、パッケージのライセンスの文 (LICENSE をそのまま) を並べる。版はロックファイルで決まる
  node_modules のもの。MIT と別のライセンスから選べるもの (Tauri の JS API など) は MIT の文を載せ、文のファイルが無い
  Tauri のプラグインは `LICENSE.spdx` の著作権者と MIT の文を載せる。仮想のモジュール (Vite・rolldown が足す小さな補助) は載せない。

`just licenses` (`just ci` に含まれる) が生成したものとコミットしたものを突き合わせ、食い違えば失敗します。

依存やモデルを更新したときの手順:

1. JS の依存を足す・上げる・外したら (`pnpm install` でロックファイルが変わったら)、`node scripts/third-party-notices.mjs` で
   3 つのファイルを生成し直し、差分 (載るパッケージ・版・ライセンスの文) を読んでからコミットする。
   ライセンスの文のファイルを持たないパッケージがバンドルに入ると生成が失敗するので、ライセンスを確かめてスクリプトに扱いを足す。
2. MediaPipe の wasm やモデルを更新したら (`packages/face-tracking/scripts/mediapipe-assets.mjs` の URL とハッシュ)、
   `packages/face-tracking/third-party-notices.txt` の節 (ファイル・取得元・モデルカード・ライセンス) を直してから、1 と同じく生成し直す。
3. 新しいアプリを足したら、スクリプトの `APPS` に加え、アプリの「ライセンス」の画面が `third-party-notices.txt` を読むようにする。

Rust のクレート (Tauri のバックエンドなど) のライセンスの表記は、ここではまだ扱っていません。

## crates.io

公開する範囲と理由は [ADR 0027](adr/0027-crates-io-publishing.md)。`ugoe` とその依存の 8 つのクレートを、依存の順に公開します。

```sh
node scripts/crates.mjs order
# ugoe-core → ugoe-model → ugoe-runtime → ugoe-format → ugoe-motion → ugoe-physics → ugoe-render → ugoe
```

### 公開の準備の検査

- `just publish-check` (`just ci` に含む): 公開するクレートのメタデータ (`description`・`readme`・`keywords`・`categories` など)、
  各クレートの `LICENSE` がルートの `LICENSE` の写しであること、内部の依存の版が揃っていること、
  各クレートの `lib.rs` に `#![warn(missing_docs)]` があること。
- doc コメントの不足: 公開するクレートは `#![warn(missing_docs)]` で、公開する項目 (型・関数・フィールド・enum の値・モジュール)
  に doc コメントが無いと警告を出します。CI の clippy は `-D warnings` なので、不足があると通りません。公開するクレートを
  増やしたら、その `lib.rs` にも入れます (`just publish-check` が確かめます)。doc コメントは中身のあるものにし、
  名前を言い換えただけの doc で埋めません。`ugoe-model` の型の doc コメントはエディタの TypeScript の型と JSON Schema
  にも入るので、変えたら `just gen` で生成物を作り直します。
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

## デスクトップアプリの署名

エディタ・ビューア・スタジオのインストーラの、コード署名と (macOS の) 公証の手順です (T0084)。
**証明書を用意して Secrets に置くのは人の手** です。Secrets が無ければ、ワークフローも手元のビルドも今までどおり署名せずに作ります
(フォークや、Secrets を置く前のリポジトリでも失敗しない)。署名しないときに利用者が見る警告と越え方は
[apps/editor/README.md#配布物の署名](../apps/editor/README.md#配布物の署名)。

署名するかどうかは `scripts/app-build.mjs` (`just app::build`) が環境変数から決めます (`scripts/lib/app-signing.mjs`)。

| OS | 署名する条件 (環境変数) | すること |
| --- | --- | --- |
| Windows | `UGOE_WINDOWS_SIGN_COMMAND` (`%1` が署名するファイル) | Tauri の `bundle.windows.signCommand` に渡す設定 (`target/app-signing/windows.conf.json`) を重ねる。スタジオの仮想カメラの DLL にも同じコマンドで署名する |
| macOS | `APPLE_SIGNING_IDENTITY` (と `APPLE_CERTIFICATE`・`APPLE_CERTIFICATE_PASSWORD`) | Tauri が `.app` に署名する。公証の資格情報が 1 組揃っていれば、Tauri が `.app` を公証してステープルし、スクリプトが `.dmg` も公証してステープルする |
| Linux | (無い) | 署名しない |

- 設定が半端なとき (公証の資格情報の組の一部だけ・署名の名義の無い証明書・`%1` の無いコマンド) は、黙って署名せずに作らず、何も作らずに止まります。
- 空の値は「無い」と同じに扱い、`tauri build` に渡す前に消します (GitHub Actions は置いていない Secrets を空の文字列で渡し、
  Tauri は空の値も設定されていると読むため)。
- 最後に、アプリと OS ごとに署名・公証したかを表示します。GitHub Actions では同じ表をジョブの要約 (`$GITHUB_STEP_SUMMARY`) に書きます。
  `DRY_RUN=1` では、署名するかどうか (`would be signed`) だけを表示します。

### Windows (Authenticode)

**証明書の種類:** コード署名の証明書 (OV か EV) の秘密鍵は、2023 年 6 月から HSM かハードウェアトークンに置く決まり
(CA/Browser Forum の Baseline Requirements for Code Signing) になり、`.pfx` のファイルとして書き出して Secrets に置けなくなりました。
また 2024 年から、EV の証明書でも SmartScreen の評価がすぐには付かなくなり (評価はダウンロードの実績で上がる)、EV を選ぶ利点が
小さくなりました。そこで、鍵をクラウドの HSM に置いたまま署名するサービスを使います。既定は
[Azure Artifact Signing](https://learn.microsoft.com/azure/trusted-signing/) (旧 Trusted Signing。月額で、証明書は Microsoft が発行して
日々更新する) で、[artifact-signing-cli](https://github.com/levminer/trusted-signing-cli) が Azure CLI と signtool を使って署名します。
Artifact Signing は使える組織・個人の条件 (国・事業の実績) があるので、申し込む前に確かめてください。使えないときは、CA の
クラウドの署名 (SSL.com の eSigner・DigiCert の KeyLocker など) の CLI に替えます (下の「ほかの署名のサービス」)。

**タイムスタンプ:** 署名には必ずタイムスタンプを付けます (証明書が切れた後も署名が有効なまま残る。Artifact Signing の証明書は
3 日で切れるので、無いと数日で無効になる)。artifact-signing-cli は既定で `http://timestamp.acs.microsoft.com` の
タイムスタンプを SHA256 で付けます。

**Tauri の設定は `signCommand`:** Tauri は `bundle.windows` の `certificateThumbprint` (Windows の証明書ストアの証明書で signtool が署名する) か
`signCommand` (任意のコマンドで署名する) を読みます ([Tauri の Windows のコード署名](https://v2.tauri.app/distribute/sign/windows/))。
`certificateThumbprint` は証明書をランナーの証明書ストアに入れる必要があり、鍵を HSM に置く今の証明書では GitHub のランナーで
使えないので、`signCommand` を使います。コマンドは `tauri.conf.json` に書かず、`UGOE_WINDOWS_SIGN_COMMAND` があるときだけ
`scripts/app-build.mjs` が設定を重ねます (書いてしまうと、署名の道具の無いビルドがすべて失敗する)。Tauri はコマンドを空白で分けて
`%1` をファイルのパスに替えて実行するので、コマンドの中の値に空白を入れないでください。

**署名されるもの:** Tauri が署名するのは、アプリの実行ファイル・サイドカー (エディタの `ugoe-mcp`)・NSIS のインストーラとアンインストーラ・
MSI です。スタジオの仮想カメラの DLL (`ugoe_studio_vcam.dll`) は Tauri ではなくインストーラのフック (`windows/hooks.nsh`) が入れるので
Tauri は署名せず、`pnpm vcam` (`apps/studio/scripts/build-vcam.mjs`) が作った直後に同じコマンドで署名します。DLL は OS の Frame Server
が読み込みます。署名が無くても読み込まれますが、ウイルス対策のソフトに止められにくくするために署名します。

**ワークフロー:** Windows のランナーで Secrets の `AZURE_CLIENT_SECRET` があれば、artifact-signing-cli を入れて
`UGOE_WINDOWS_SIGN_COMMAND=artifact-signing-cli -e <エンドポイント> -d ugoe %1` を設定します。アカウント・証明書のプロファイル・
Azure の資格情報は環境変数で渡します。

**ほかの署名のサービス:** `UGOE_WINDOWS_SIGN_COMMAND` は署名のコマンドなら何でもよいので、手元では使う道具のコマンドを
そのまま設定できます (例: `smctl sign --keypair-alias <別名> --input %1`)。ワークフローで替えるときは、`editor-build.yml` の
「Set up Windows code signing」の手順 (道具を入れてコマンドを設定する) と、Build の手順に渡す資格情報の Secrets を替えます。

### macOS (Developer ID と公証)

**証明書:** Apple Developer Program のアカウントで **Developer ID Application** の証明書を作り、キーチェーンから秘密鍵ごと `.p12` に
書き出します。Tauri は `APPLE_CERTIFICATE` (`.p12` の base64) と `APPLE_CERTIFICATE_PASSWORD` があれば一時のキーチェーンに
入れ、`APPLE_SIGNING_IDENTITY` (`Developer ID Application: <名義> (<チーム ID>)`) の名義で `.app` に署名します
([Tauri の macOS のコード署名](https://v2.tauri.app/distribute/sign/macos/))。手元の Mac でキーチェーンに証明書があるなら、
`APPLE_SIGNING_IDENTITY` だけで足ります。Tauri は Hardened Runtime (公証に要る) を既定で有効にし、`Entitlements.plist`
(カメラ・マイク) を付けます。

**公証:** Developer ID で配るものは公証が要ります (無いと Gatekeeper が止める)。Tauri は次のどちらかの組があれば、`tauri build` の中で
`.app` を notarytool で公証してステープルします。

| 組 | 環境変数 |
| --- | --- |
| App Store Connect の API キー (推奨) | `APPLE_API_ISSUER` (発行者 ID)・`APPLE_API_KEY` (キー ID)・`APPLE_API_KEY_PATH` (`.p8` のパス) |
| Apple ID | `APPLE_ID`・`APPLE_PASSWORD` (App 用パスワード)・`APPLE_TEAM_ID` |

API キーを勧めるのは、個人の Apple ID とそのパスワードを CI に置かずに済み、キーだけを取り消せるためです (キーの役割は Developer)。
Tauri は `.dmg` を公証しないので、`scripts/app-build.mjs` が `target/dist/<アプリ>/macos/` に写した `.dmg` を
`xcrun notarytool submit --wait` で公証し、`xcrun stapler staple` でステープルします。

**ワークフロー:** macOS のランナーで Secrets の `APPLE_API_PRIVATE_KEY` (`.p8` の中身) があれば `$RUNNER_TEMP` に書いて
`APPLE_API_KEY_PATH` を設定し、ほかの値は Secrets から環境変数で渡します。

**スタジオの仮想カメラ (Camera Extension) との関係:** CI と既定のビルドのスタジオは仮想カメラを入れない (`VCAM=1` なし) ので、ほかの
アプリと同じ手順で署名と公証ができます。仮想カメラを入れたスタジオ ([macOS の仮想カメラ](#macos-の仮想カメラ)) は、同じ Developer ID の
証明書と `APPLE_SIGNING_IDENTITY` を使い、加えてプロビジョニングプロファイル 2 つと `UGOE_APPLE_TEAM_ID` が要ります。
拡張はスクリプトが先に署名し、Tauri がその外のアプリに署名して公証します。CI で `VCAM=1` のビルドを作るのは、配布物に入れると決めてからです
(プロファイルも Secrets に置くことになる)。

### Linux

Linux の `.deb`・`.rpm`・`.AppImage` には署名しません。配布の形 (GitHub Release からのダウンロード) では、署名しても利用者が検証する
手順がふつう無いためです。GitHub Release は添付したファイルごとに SHA256 のダイジェストを表示するので、取り違えはそれで確かめられます。
apt や dnf のリポジトリで配る・AppImage に GPG の署名を埋める (Tauri の `SIGN=1`) ときは、そのとき鍵の扱いと一緒に決めます。

### Secrets を置く (初回だけ。人の手)

GitHub のリポジトリの Settings → Secrets and variables → Actions の **Repository secrets** に置きます。置いた分の OS だけが署名され、
置いていない OS は今までどおり署名しません。

| Secret | 中身 |
| --- | --- |
| `AZURE_TENANT_ID`・`AZURE_CLIENT_ID`・`AZURE_CLIENT_SECRET` | 証明書のプロファイルに Certificate Profile Signer の役割 (Artifact Signing の組み込みの役割) を付けた、Microsoft Entra ID のアプリの登録 (サービスプリンシパル) の資格情報。`AZURE_CLIENT_SECRET` があることが Windows で署名する合図 |
| `AZURE_ARTIFACT_SIGNING_ENDPOINT` | アカウントの地域のエンドポイント (例: `https://eus.codesigning.azure.net`) |
| `AZURE_ARTIFACT_SIGNING_ACCOUNT`・`AZURE_ARTIFACT_SIGNING_CERTIFICATE_PROFILE` | アカウントの名前と証明書のプロファイルの名前 |
| `APPLE_CERTIFICATE` | Developer ID Application の `.p12` の base64 (`base64 -i developer-id.p12`) |
| `APPLE_CERTIFICATE_PASSWORD` | `.p12` を書き出したときのパスワード |
| `APPLE_SIGNING_IDENTITY` | `Developer ID Application: <名義> (<チーム ID>)` (`security find-identity -v -p codesigning` で表示される名前) |
| `APPLE_API_ISSUER`・`APPLE_API_KEY` | App Store Connect の「ユーザーとアクセス」→「統合」→ API キーの発行者 ID とキー ID |
| `APPLE_API_PRIVATE_KEY` | API キーの `.p8` の中身 (`-----BEGIN PRIVATE KEY-----` から) |

- Secrets はリポジトリに書き込める人のワークフローから読めます。タグのときだけに絞るなら、environment を作って
  Deployment branches and tags を `v*` に絞り、Secrets をその environment に移し、`editor-build.yml` のジョブに `environment:` を付けます
  (そのときは手動の実行をブランチから行うと失敗するので、手動の実行もタグを選んで行います)。
- 証明書や API キーを取り消した・更新したら、Secrets も置き換えます。Artifact Signing の証明書は Microsoft が更新するので置き換えは要りません。
- 置いた後の最初のタグ (または手動の実行) で、ジョブの要約の「Code signing」の表が `signed` になっていることと、下の確かめ方で
  署名と公証を確かめます。

### 署名を確かめる

```powershell
# Windows: 署名とタイムスタンプ (Status が Valid、TimeStamperCertificate があること)
Get-AuthenticodeSignature .\ugoe_<版>_x64-setup.exe | Format-List *
signtool verify /pa /v "C:\Program Files\ugoe\ugoe-mcp.exe"
signtool verify /pa /v "C:\Program Files\ugoe-studio\ugoe_studio_vcam.dll"
```

```sh
# macOS: 署名・Gatekeeper の評価・ステープル
codesign --verify --deep --strict --verbose=2 /Applications/ugoe.app
spctl --assess --type execute --verbose /Applications/ugoe.app      # source=Notarized Developer ID
xcrun stapler validate /Applications/ugoe.app
xcrun stapler validate ugoe_<版>_universal.dmg
spctl --assess --type open --context context:primary-signature --verbose ugoe_<版>_universal.dmg
```

SmartScreen の評価はダウンロードの実績で付くので、署名した直後の版でも警告が出ることがあります (署名していれば、発行元に名義が出る)。
確かめていないことは [manual-check.md の D 節](manual-check.md#d-開発者向け-ビルドci配布バインディング) の D10・D11。

## GitHub Release の下書き

`v<版>` のタグで動くワークフローは、配布物をそのタグの GitHub Release の **下書き** に添付します。Release を公開する (下書きを
公開に切り替える) のは人の手です。添付は `scripts/github-release.mjs` で揃えます。

```sh
node scripts/github-release.mjs upload <タグ> <ファイル...>   # GH_TOKEN (contents: write) と gh が要る
```

- そのタグの Release が無ければ、下書きで作ります (`gh release create --draft --verify-tag`。題はタグ、本文は書き直す前提の短い文)。
- いくつものワークフローが同じ下書きに添付します。同時に作って下書きが重なったら、いちばん古いものに寄せ、添付の無い余分な下書きを消します。
- 同じ名前のファイルは置き換えます (ジョブをやり直したとき)。公開済みの Release には添付せず、失敗します (配った物を差し替えないため)。
- ワークフローの側は、ビルドのジョブの後に `if: startsWith(github.ref, 'refs/tags/v')` のジョブを置き、`permissions: contents: write` で
  成果物を集めてこのスクリプトを呼びます (`editor-build.yml` の `release` のジョブ)。

| 添付するもの | ワークフロー |
| --- | --- |
| エディタ・ビューア・スタジオのインストーラ (`.msi`・NSIS の `.exe`・`.dmg`・`.deb`・`.rpm`・`.AppImage`) | `editor-build.yml` |

公開の前に、人が次を行います。

1. 下書きに、配布物がそろっていることを確かめる (ワークフローがすべて終わってから)。
2. 本文 (変更点・既知の問題・署名の有無) を書き直す。
3. 「Publish release」で公開する。

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
