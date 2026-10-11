# リリース

配布物の一覧と方針は [portability.md#配布](portability.md#配布)。すべての配布物で版を揃え、版の正はルートの `Cargo.toml` の
`[workspace.package] version` です。`v<版>` のタグ (例: `v0.1.0`) を打つと、`.github/workflows/release.yml` が動きます。

1. まず重い検査の一式 `release-checks.yml` (別のターゲット・Windows と macOS・GPU・Godot・配布物の適合テスト・e2e・Unity・perf・
   tier 3 の build-std・fuzz。[testing.md#重い検査とリリースの前の検査](testing.md#重い検査とリリースの前の検査)) を流します。
   push・PR ごとの CI (`ci.yml`) は Linux の軽い検査だけなので、ここで初めて流れる検査があります (利用者の決定 2026-10-11。T0091)。
2. 検査がすべて通ったら、下の表の配布物ごとのワークフローを `workflow_call` で呼びます。1 つでも落ちたら、配布物は作りません。
   直したら新しいコミットに新しいタグを打つか、Actions の画面で落ちたジョブを再実行します。

タグを打つ前に確かめたいときは、`Release checks` を手で起動します (main などのブランチで。課金で約 210 分)。配布物のワークフローも
それぞれ手で起動でき、予行になります (公開や添付はタグを ref に選んで入力を選んだときだけ)。

| 配布物 | ワークフロー | 手順 |
| --- | --- | --- |
| crates.io のクレート | `.github/workflows/crates-release.yml` | [crates.io](#cratesio) |
| npm の `@ugoe/runtime` | `.github/workflows/web-runtime-release.yml` | [bindings/web/README.md#リリース](../bindings/web/README.md#リリース) |
| エディタ・ビューア・スタジオのインストーラ | `.github/workflows/editor-build.yml` (`just app::build` を OS ごとのランナーで実行。Secrets があれば署名し、タグでは [下書きの Release](#github-release-の下書き) に添付) | [apps/editor/README.md#各-os-でのビルド](../apps/editor/README.md#各-os-でのビルド)・[デスクトップアプリの署名](#デスクトップアプリの署名) |
| Unity の UPM パッケージ (tarball) | `.github/workflows/unity.yml` (`release-checks.yml` の一部として動き、成果物に残すだけ。ほかの検査と並んで作られるので、配る場所に置くのは `release.yml` の run 全体が通ってから) | [bindings/unity/README.md#リリース](../bindings/unity/README.md#リリース) |
| スタジオの macOS の仮想カメラ (Camera Extension) | まだ無い (手で作る。1.0 の配布物には入れない。[1.0 の後の候補](roadmap.md#10-の範囲)) | [macOS の仮想カメラ](#macos-の仮想カメラ) |
| C ABI (`ugoe-ffi` の共有ライブラリ・静的ライブラリとヘッダ) | `.github/workflows/ffi-release.yml` (OS ごとのランナーで作り、GitHub Release の下書きに添付) | [C ABI の配布物](#c-abi-の配布物) |

公開するかどうか、いつ公開するかは、リポジトリを持つ人 (利用者) が決めて行います。1.0 (v1.0.0) は、[1.0 の範囲](roadmap.md#10-の範囲) (M1〜M11 の完了。
[ADR 0035](adr/0035-one-point-zero-scope-m1-to-m11.md)) を終えてから、Release のマイルストーンで公開します。

## 公開の前の点検

リポジトリを公開 (Public) にすると、作業ツリーだけでなく履歴 (すべてのコミットの差分とメッセージ・著者) も見えます。
公開の前に、見せてはいけないもの・見せる意味の無いものが無いかを調べます。2026-10-11 の点検の結果は
[T0090](tasks/T0090-pre-publication-audit.md) のメモにあります。

**決定 (利用者 2026-10-11): 履歴は書き換えません。** T0090 の履歴の点検で見つかったのは、タスクファイルの古い版に残る機械に固有のパス
(WSL と Windows のユーザー名・ディレクトリの置き方・エージェントのセッションの UUID) だけで、秘密の値・外部 SDK・生成物は
ありませんでした。書き換えるとすべてのコミットの ID が変わり、タスクファイルや文書のコミットの ID・CI の実行への参照が切れるので、
`git filter-repo` などは使わず、今の作業ツリーだけを `just local-paths` で守ります。

### 自動の検査

- **機械に固有のパス:** `just local-paths` (`scripts/check-local-paths.mjs`) が、ホームのディレクトリ (`/home/<名前>`・`/Users/<名前>`・
  `C:\Users\<名前>`・WSL の `\home\<名前>`) とエージェントの一時ディレクトリ (`/tmp/claude-<番号>/…`) を見つけたら落ちます。`just ci`・
  CI の `rust-checks`・pre-commit (lefthook) で動きます。文書やタスクのメモでは、リポジトリの中の相対パスか `<repo>`・`<scratchpad>`・
  `$HOME`・`%USERPROFILE%` と書きます。テストや例のパスは 1 文字の名前 (`/home/u`・`C:\Users\a`) か、スクリプトの `PLACEHOLDER_USERS`
  の名前 (`me`・`you` など) を使います。
- **秘密の値:** 自動の検査はありません (CI や `just` で回すかは T0098 で決める)。公開の直前に [gitleaks](https://github.com/gitleaks/gitleaks) を版を固定して (T0090 では 8.24.3)
  リポジトリの外に置き、作業ツリーと履歴の両方を見ます:
  `gitleaks dir . --no-banner` と `gitleaks git . --log-opts="--all" --no-banner`。T0090 では、WebSocket の RFC 6455 の例の
  `Sec-WebSocket-Key` (`apps/studio/src-tauri/src/api/`) だけが見つかりました (秘密の値ではありません)。
- **外部 SDK・ライセンス:** `just licenses` が依存のライセンスを見ます。画像・モデル・PSD の出所は `assets/fixtures/README.md`・
  `assets/fixtures/psd-real/README.md`・`assets/samples/README.md` の表に書きます (表に無いものを足さない)。アプリのアイコンの出所はまだ書いていません (T0097)。

### 人がすること

公開の前に、リポジトリを持つ人が GitHub の設定で行います。

- **コミットの著者:** 履歴のすべてのコミットの著者の名前とメールアドレスが見えます。今後のコミットで隠すなら、GitHub の
  `<ID>+<ユーザー名>@users.noreply.github.com` を `git config user.email` にし、Settings → Emails の
  「Block command line pushes that expose my email」を有効にします (履歴は書き換えないので、過去のコミットの著者はそのまま見えます)。
- **push するもの:** `main` とタグ (`v*`) だけを push します。タスクのブランチ (`task/…`)・worktree・stash は手元のものなので、
  `git push --mirror` や `git push --all` はしません。
- **リポジトリの設定:** 既定のブランチは `main`。Issues を有効にし、Discussions・Wiki・Projects を使うかを決めます (使わないなら切る)。
  マージの方法 (squash・rebase・merge commit) をそろえ、「Automatically delete head branches」を有効にします。
- **ブランチとタグの保護 (Rulesets):** `main` は PR を通さない push・force push・削除を禁じ、CI (`ci.yml` のジョブ) を必須の
  チェックにします。`v*` のタグは公開のワークフロー (crates.io・npm・インストーラ・C ABI) を動かすので、作れる人を限ります。
- **PR の運用へ移る:** 公開するまでは、オーケストレーターが手元の main にマージを重ねてまとめて push しています。公開したら、
  タスクごとに PR を作って `ci.yml` の検査を通してからマージする運用に移ります。必須の検査は `ci.yml` の `rust`・`web`・`rust-checks`
  (`release-checks.yml` は PR で動かないので入れない)。公開の前に、手元の main を push し終えておきます
  ([multi-agent.md](workflow/multi-agent.md#main-を-push-する区切り)。T0099)。
- **Secrets:** 公開しても Secrets の値は見えませんが、書き込める人のワークフローからは読めます。公開に使うもの
  (`CARGO_REGISTRY_TOKEN`・`NPM_TOKEN`・`APPLE_*`・`AZURE_*`) は environment に置き、Deployment branches and tags を `v*` に絞り、
  required reviewers を付けます ([crates.io](#cratesio)・[Secrets を置く](#secrets-を置く-初回だけ人の手))。
- **フォークからの PR:** ワークフローは `pull_request_target` を使っていないので、フォークからの PR に Secrets は渡りません (この形を保つ)。
  Settings → Actions → General で、外の人の PR のワークフローを承認してから動かす設定 (「Require approval for all external contributors」) にし、
  「Workflow permissions」を読み取りだけにします (書き込みが要るジョブは `permissions:` で個別に付けている)。
- **セキュリティの機能:** Private vulnerability reporting・Dependabot alerts・Secret scanning と push protection を有効にします。
- **文書:** `CONTRIBUTING.md` (貢献の手順)・`SECURITY.md` (脆弱性の報告先)・`CODE_OF_CONDUCT.md` (Contributor Covenant 2.1)・
  Issue と PR のテンプレート (`.github/ISSUE_TEMPLATE/`・`.github/pull_request_template.md`) はあります (T0096)。公開の前に次を決めて直します。
  - **連絡先:** `SECURITY.md` (GitHub を使えない人の連絡先) と `CODE_OF_CONDUCT.md` (Enforcement の節。行動規範の違反の報告先) の
    `[INSERT CONTACT METHOD]` を、利用者が決めた連絡先 (メールアドレスなど) に置き換えます。`git grep -n "INSERT CONTACT METHOD"` で残りが無いことを確かめます。
  - **リポジトリの URL:** `SECURITY.md` と `.github/ISSUE_TEMPLATE/` は `https://github.com/rosylilly/ugoe` を指します。公開する場所が違うなら直します。
  - **Private vulnerability reporting:** `SECURITY.md` の報告先なので、上の「セキュリティの機能」で必ず有効にします。
  - **ラベル:** Issue のテンプレートは GitHub の既定のラベル `bug`・`enhancement` を付けます。消したなら作り直すかテンプレートを直します。

## 同梱物のライセンスの表記

エディタ・ビューア・スタジオは、同梱する第三者のもの (MediaPipe の wasm とモデル、バンドルに入る JS のライブラリ、バイナリに入る
Rust のクレート) のライセンスの表記を `apps/<アプリ>/public/third-party-notices.txt` に持ち、アプリの「ライセンス」の画面で
表示します (T0926・T0005)。この 3 つのファイルは手で書かず、`just notices` (`scripts/third-party-notices.mjs`) が次の 3 つから生成します。

- MediaPipe の節: `packages/face-tracking/third-party-notices.txt` の最初の節から終わりまで (Apache-2.0 の条文を含む)。
- JS のライブラリの節: アプリを書き出さずに `vite build` し、バンドルに入ったモジュールの node_modules のパッケージごとに、
  名前・版・ライセンス・ソースと、パッケージのライセンスの文 (LICENSE をそのまま) を並べる。版はロックファイルで決まる
  node_modules のもの。MIT と別のライセンスから選べるもの (Tauri の JS API など) は MIT の文を載せ、文のファイルが無い
  Tauri のプラグインは `LICENSE.spdx` の著作権者と MIT の文を載せる。仮想のモジュール (Vite・rolldown が足す小さな補助) は載せない。
- Rust のクレートの節: アプリのバックエンド (`apps/<アプリ>/src-tauri/Cargo.toml`) を
  [cargo-about](https://github.com/EmbarkStudios/cargo-about) にかけ、クレートの一覧 (名前・版・ライセンス・ソース) と
  ライセンスの文 (同じ文は 1 つにまとめ、使うクレートを添える) を並べる。組み立ては `scripts/lib/rust-notices.mjs`、
  cargo-about の設定は `scripts/about.toml`。
  - 対象: Windows・macOS・Linux (x86_64 と aarch64) のどれかのビルドに入るクレート。ビルドスクリプトだけ・テストだけが使う
    クレートは載せない。ワークスペースのクレート (ugoe 自身) は載せない。版は `Cargo.lock` のもの。
  - 手続きマクロのクレート (`serde_derive`・`thiserror-impl` など。`cargo metadata` の target の kind が `proc-macro`) と、
    手続きマクロ・ビルドスクリプトからしか使われないクレート (`syn`・`quote`・`proc-macro2` など) も載せない。どちらもホスト向けに
    ビルドされてコンパイラが読み込むか、手続きマクロ・ビルドスクリプトにリンクされるだけで、配布物のバイナリには入らない。
    cargo-about はこれらを通常の依存と区別しないので、`scripts/lib/rust-notices.mjs` が `cargo metadata` の依存をアプリの
    クレートから通常の依存だけでたどり (手続きマクロのクレートの先はたどらない)、たどれないものを cargo-about の結果から除く。
    通常の依存からも使われるクレートは残る。除いた数は `just notices` のログに出る。
    `cargo metadata` は機能をホスト向けの依存と合わせて解くので、ホスト向けの機能だけが引き込むクレート (Tauri の `tauri-utils` の
    ビルド用の機能の依存など) は残る (載せすぎるほうに寄せる)。
  - 選べるライセンス (`MIT OR Apache-2.0` など) は、`scripts/about.toml` の `accepted` の前にあるものを選ぶ (MIT を最初に、
    MIT-0 を Apache-2.0 より前に置く。`CC0-1.0 OR MIT-0 OR Apache-2.0` の `dunce` は MIT-0 の文になる)。
    `accepted` は `scripts/check-licenses.mjs` の GPL-3.0 と両立するライセンスと同じ集合で、`just licenses` が突き合わせる。
  - 文はクレートのライセンスのファイル (LICENSE・COPYING・NOTICE など) から取る。cargo-about はソースのファイルの見出しからも文を
    拾うが、ソースの全文になるので載せない。ライセンスのファイルを持たないクレートは SPDX の標準の文を載せ、著作権者の欄の代わりに
    Cargo.toml の authors (名前だけ) を添える。文が 1 つも見つからないクレートがあると生成が失敗する。
  - 文の取り出しはネットワークを使わない (`--frozen`。手元とネットワークの状態で結果が変わらないように)。
  - cargo-about の版は `scripts/lib/rust-notices.mjs` の `CARGO_ABOUT_VERSION` に固定し、`just cargo-about` が
    その版を入れる (`cargo install cargo-about --version <版> --locked --features cli`)。CI (ci.yml の rust-checks) も同じ
    レシピで入れる。cargo-about は依存のすべてのファイルを読むのでアプリ 1 つに数十秒かかり、結果は入力 (cargo-about の版・設定・
    `Cargo.lock`・すべての `Cargo.toml`) のハッシュごとに `target/cargo-about/` に覚える。

`just licenses` (`just ci` に含まれる) が生成したものとコミットしたものを突き合わせ、食い違えば失敗します (JS の依存の版がロックファイルと
変わったとき、Rust の依存が変わったときを含む)。これには cargo-about が要ります (無ければ `just cargo-about` を案内して失敗します)。

依存やモデルを更新したときの手順:

1. JS の依存を足す・上げる・外したら (`pnpm install` でロックファイルが変わったら)、Rust の依存を足す・上げる・外したら
   (`Cargo.lock` やアプリの `Cargo.toml` が変わったら)、`just notices` で 3 つのファイルを生成し直し、差分 (載るパッケージと
   クレート・版・ライセンスの文) を読んでからコミットする。
   ライセンスの文のファイルを持たないパッケージがバンドルに入ると生成が失敗するので、ライセンスを確かめてスクリプトに扱いを足す。
2. MediaPipe の wasm やモデルを更新したら (`packages/face-tracking/scripts/mediapipe-assets.mjs` の URL とハッシュ)、
   `packages/face-tracking/third-party-notices.txt` の節 (ファイル・取得元・モデルカード・ライセンス) を直してから、1 と同じく生成し直す。
3. 新しいアプリを足したら、スクリプトの `APPS` に (バックエンドの `Cargo.toml` とともに) 加え、アプリの「ライセンス」の画面が
   `third-party-notices.txt` を読むようにする。
4. cargo-about の版を上げるときは `CARGO_ABOUT_VERSION` を変え、`just cargo-about` で入れてから `just notices` で生成し直す。

### MIT の配布物

ランタイムを外へ出す MIT の配布物も、中に入る Rust のクレート (MIT・Apache-2.0 など。表記を配布物に入れることを求める) の表記を
持ちます (T1407)。表記は `scripts/dist-notices.mjs` (`just notices`) が、アプリの Rust のクレートの節と同じ道具と形
(cargo-about・`scripts/about.toml`・`scripts/lib/rust-notices.mjs`) で生成し、`just licenses` が依存 (`Cargo.lock`・
`bindings/godot/Cargo.lock`・`Cargo.toml`) と突き合わせます。手で書き直しません。

| 配布物 | 表記のファイル | 対象のクレートとターゲット |
| --- | --- | --- |
| npm の `@ugoe/runtime` | `bindings/web/THIRD-PARTY-NOTICES.txt` (`package.json` の `files` に入れる) | `ugoe-wasm` の `wasm32-unknown-unknown` |
| Unity の UPM パッケージ | `bindings/unity/com.ugoe.unity/Third Party Notices.md` (Unity の慣例の名前。tarball に入っているかを `release.mjs pack` が確かめる) | `ugoe-ffi` の `unity.yml` のターゲット (Windows・macOS の 2 つ・Linux・Android の 3 つ・iOS) |
| Godot のアドオン | `bindings/godot/example/addons/ugoe/THIRD-PARTY-NOTICES.txt` (アドオンのフォルダに入る) | `bindings/godot` の `.gdextension` のターゲット (Windows・macOS の 2 つ・Linux の 2 つ) |
| C ABI のアーカイブ | `crates/ugoe-ffi/THIRD-PARTY-NOTICES.txt` | `ugoe-ffi` の Windows・macOS の 2 つ・Linux の 2 つ |

- ugoe 自身のクレートは載せず、表記の頭で ugoe が MIT であること (ライセンスの文の在り処) を示します。
- 対象のクレートは、配布物のライブラリのクレートを表のターゲットのどれかに向けてビルドしたときに入るもの (既定の features)。
  アプリと同じく、手続きマクロのクレートと、手続きマクロ・ビルドスクリプトだけが使うクレートは載せない (`wasm-bindgen-macro`・
  `godot-macros`・`syn` など)。
  ターゲットを足したら `scripts/dist-notices.mjs` の `DISTS` も直します (Unity は `unity.yml` の Android・iOS のターゲットを
  含むかをテストが確かめる)。
- Godot のアドオンには godot-rust のクレート (MPL-2.0) が入ります。MPL-2.0 の 3.2 は実行形式を配るときにソースの在り処を示すことを
  求めるので、MPL-2.0 で使うクレートごとに crates.io のその版のページ・`.crate` のアーカイブ・リポジトリの URL を表記の終わりに書きます
  (変更は加えていない。[ADR 0026](adr/0026-godot-binding.md))。
- C ABI のアーカイブには、`scripts/ffi-dist.mjs` がこのファイルを `THIRD-PARTY-NOTICES.txt` として詰めます。

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

### 公開の範囲

公開するクレートの `pub` の項目は、そのまま利用者の API になります (`ugoe` は下位のクレートを `ugoe::core`・`ugoe::model` などとして
丸ごと再公開するので、下位のクレートの公開の項目はすべて `ugoe` の API でもあります)。

**方針 (利用者 2026-10-11):** 公開も 1.0 の日程も決まっていないので、API・ABI の版や挙動の安定には神経質にならず、よりよいソフトウェアに
育てることを優先します。公開の範囲・名前・型・C ABI を変えるときも、移行の説明・互換の別名・semver の約束のための仕掛け
(`unstable-` の feature・`#[non_exhaustive]` の揃え・約束に含めないという注記など) は要りません。下の目安は、約束のためではなく、
クレートの中を変えやすく・読みやすく保つためのものです。公開の範囲を狭めるのも「内部の整理」として扱います (T0195 の洗い出しと、
そこから起票した T0196 (core・runtime・format・motion)・T0336 (physics))。

- **既定は `pub(crate)`。** `pub` にするのは、次のどれかに当たる項目だけです。
  - ほかの公開するクレートが使う (`ugoe-physics` が `ugoe-runtime` の `ParameterValues` を使うなど)。
  - 公開の項目の型 (関数の引数と戻り値・公開のフィールド・enum の値) に出てくる (出てくるなら公開するしかない。出したくなければ型のほうを変える)。
  - 組み込み先 (ホスト・エンジン・C ABI・wasm・ツール) が使う口として、doc コメントで用途を説明できる。
- **ワークスペースの都合だけで `pub` にしない。** 実装の部品 (積分の 1 ステップ・衝突の補助・形式の内部の配置の定数など) は、
  同じクレートのテストが使うだけなら `pub(crate)` で足ります。公開しないクレート (`ugoe-ffi`・`ugoe-wasm`・ツール層) が要るときは、
  利用者にも意味のある口かを考え、そうでなければ公開するクレートの側に意味のある口を足します。
- **1 つの項目に道は 1 つ。** モジュールの項目をすべてクレートの根で再公開するなら、モジュールは非公開にします (ファイルの分け方を
  API にしない)。根で再公開しない項目を持つモジュール (`ugoe_core::lighting`・`ugoe_model` のデータのモジュールなど) は公開のままです。
- **`#[doc(hidden)]` は、公開するクレート同士の配管で利用者に見せない項目だけに使います。** 利用者に見せたくないだけの項目は `pub(crate)` にします。
- **テストとコード生成の道具** (`ugoe_model::fixtures`、feature `schema`・`ts` と `ugoe_core::codegen`・`ugoe_model::spec_files`) は
  ワークスペースのテストと道具のためのもので、今の置き場所のまま使います (別の feature の裏へ移すなどはしない)。中身や形は、テストと道具の
  都合で自由に変えてかまいません。
- **`#[non_exhaustive]`** は、付けると網羅的な `match` が書けなくなる代わりに値を足しやすくなります。揃えることはせず、値が増えていく
  ことが分かっている型に、作るときに判断して付けます。
- **外部のクレートの型を公開 API に出すと、その版も API の一部になります** (`glam` の `Vec2` など、`ugoe-render` の `wgpu`、
  feature の下の `schemars`・`ts-rs`)。依存を上げるときに組み込み先が困らないよう、新しく出すときは PR の本文に理由を書きます。
- 公開の範囲は `cargo +nightly rustdoc -p <クレート> -- -Z unstable-options --output-format json` の JSON で一覧にできます
  (手元で一度使うだけの道具なので、リポジトリには入れていません。調べ方は T0195 のメモ)。

### 版を上げる

1. ルートの `Cargo.toml` の `[workspace.package] version` を変えます。
2. `node scripts/crates.mjs sync` で、`[workspace.dependencies]` の公開するクレートの `version` を揃えます。
3. `node bindings/web/scripts/release.mjs sync` と `node bindings/unity/release.mjs sync` で、npm のパッケージと
   Unity のパッケージの版も揃えます。
4. `cargo update --workspace` で `Cargo.lock` を更新します。
5. `just changelog v<版>` で `CHANGELOG.md` を作り直し、差分を読みます ([変更履歴とリリースノート](#変更履歴とリリースノート))。
   1〜5 の変更を 1 つのコミットにします。
6. そのコミットに `v<版>` のタグを打って push します。

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

- **タグから (通常):** `v<版>` のタグを push すると、`release.yml` が重い検査の後に `crates-release.yml` を呼び、予行 (`package` ジョブ) が動き、通れば
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
| Linux | (無い) | 署名しない (代わりにチェックサムと成果物の証明で確かめる。[Linux](#linux)) |

- 設定が半端なとき (公証の資格情報の組の一部だけ・署名の名義の無い証明書・`%1` の無いコマンド) は、黙って署名せずに作らず、何も作らずに止まります。
- 空の値は「無い」と同じに扱い、`tauri build` に渡す前に消します (GitHub Actions は置いていない Secrets を空の文字列で渡し、
  Tauri は空の値も設定されていると読むため)。
- 最後に、アプリと OS ごとに署名・公証したかを表示します。GitHub Actions では同じ表をジョブの要約 (`$GITHUB_STEP_SUMMARY`) に書きます。
  `DRY_RUN=1` では、署名するかどうか (`would be signed`) だけを表示します。
- 環境変数 `UGOE_UNSIGNED_REASON` があれば、署名しないときの理由の前に出します (`editor-build.yml` のタグでないジョブが、
  署名の Secrets を読めない理由を書く。[Secrets を置く](#secrets-を置く-初回だけ人の手))。

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

Linux の `.deb`・`.rpm`・`.AppImage` には署名しません (利用者の決定 2026-10-11。T0093)。

- 1.0 は GitHub Release からのダウンロードだけで配り、apt・dnf のリポジトリは作りません。リポジトリが無いと、`.deb`・`.rpm` の
  GPG の署名を利用者のパッケージマネージャが確かめる経路が無く (公開鍵を入れる手順を別に案内することになる)、AppImage に埋めた署名
  (Tauri の `SIGN=1`) も確かめる道具がふつう入っていません。鍵を守り続ける手間に比べて、確かめる人がほとんどいないためです。
- 代わりに、ほかの配布物と同じく **SHA-256 のチェックサム** (`<ファイル>.sha256`) と **GitHub の成果物の証明** (build provenance) を
  添えます。証明は、そのファイルがこのリポジトリのどのワークフローの、どのコミット・タグのビルドで作られたかを Sigstore で署名して記録
  したもので、利用者は `gh attestation verify` で確かめます ([配布物を確かめる](#配布物を確かめる))。鍵は GitHub Actions の OIDC で
  ビルドごとに作られるので、守る鍵がありません。
- apt や dnf のリポジトリで配ると決めたら、そのとき鍵の扱いと一緒に GPG の署名を決めます。

### Secrets を置く (初回だけ。人の手)

署名の Secrets は、`v*` のタグからだけ使える environment `release` に置きます。`editor-build.yml` は、`v*` のタグのときだけ
`build-signed` のジョブに `environment: release` を付けて署名し、それ以外 (手での起動・ブランチ) では `build` のジョブが environment を
付けずに署名せずに作ります (ブランチから動かしても保護の規則で失敗しません)。ジョブの要約の「Code signing」の表には、署名しなかった理由
(`not a v* tag ...`) が出ます。

1. GitHub のリポジトリの Settings → Environments で、environment `release` を作ります。
   - **Deployment branches and tags** を「Selected branches and tags」にし、タグの規則 `v*` だけを許します (ブランチは加えない)。
   - **Required reviewers** は任意です。付けると、タグの run の 9 つの署名するジョブが承認を待ちます (承認の画面でまとめて承認できます)。
     付けなくても、Secrets を読めるのは `v*` のタグの run だけです。
2. その environment の **Environment secrets** に、下の表の Secrets を置きます。置いた分の OS だけが署名され、置いていない OS は
   今までどおり署名しません。
3. Settings → Secrets and variables → Actions の **Repository secrets** に同じ名前の Secrets が残っていれば消します (残っていると、
   タグでない `build` のジョブもそれで署名してしまいます)。

`crates-io` の environment ([公開できるようにする](#公開できるようにする-初回だけ)) は公開の前に人の承認を待つためのもので
(required reviewers が要る)、`release` は Secrets を読める ref をタグに絞るためのものです (承認は任意)。

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

- **手での起動で署名を試す:** Actions の画面で `Editor build` を「Run workflow」し、「Use workflow from」で **`v*` のタグ** を選びます
  (ブランチを選ぶと署名しません)。タグを ref にした run は `build-signed` のジョブで署名し、`release` のジョブがそのタグの
  GitHub Release の下書きにインストーラを添付します (下書きが無ければ作る)。試すだけなら、捨ててよいタグ (例: `v0.0.0-signing-test`)
  を打って選び、後で下書きの Release とタグを消します。
- 証明書や API キーを取り消した・更新したら、Secrets も置き換えます。Artifact Signing の証明書は Microsoft が更新するので置き換えは要りません。
- 置いた後の最初のタグ (またはタグを選んだ手動の実行) で、ジョブの要約の「Code signing」の表が `signed` になっていることと、下の確かめ方で
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
確かめていないことは [manual-check.md の D 節](manual-check.md#d-開発者向け-ビルドci配布バインディング) の D10〜D12 と、environment の保護の規則での実行。

## 配布物を確かめる

ダウンロードした配布物が、このリポジトリの CI で作ったものと同じかを、利用者が確かめるための仕組みです (T0093)。
署名 ([デスクトップアプリの署名](#デスクトップアプリの署名)) とは別に、すべての OS の配布物に付けます。

| 配布物 | チェックサム | 成果物の証明 | 作るところ |
| --- | --- | --- | --- |
| エディタ・ビューア・スタジオのインストーラ | `<インストーラ>.sha256` (下書きの Release に添付) | `actions/attest-build-provenance` | `editor-build.yml` の `build`・`build-signed` (`.sha256` は `scripts/app-build.mjs`) |
| C ABI のアーカイブ | `<アーカイブ>.sha256` (下書きの Release に添付) | `actions/attest-build-provenance` | `ffi-release.yml` の `archive` (`.sha256` は `scripts/ffi-dist.mjs`) |
| npm の `@ugoe/runtime` | npm のレジストリが持つ (`integrity`。`npm install` が確かめる) | npm の来歴 (`npm publish --provenance`) | `web-runtime-release.yml` |
| Unity の UPM の tarball・Godot の zip | 同じ形で付ける予定 (T0421) | 同じ形で付ける予定 (T0421) | — |

- `.sha256` の中身は `sha256sum` と同じ 1 行 (`<16 進のダイジェスト>  <ファイル名>`) です (`scripts/lib/checksum.mjs`)。
  手元の `just app::build`・`just ffi-dist` も同じファイルを作ります。インストーラの `.sha256` は、macOS の `.dmg` の公証のステープル
  (ファイルを書き換える) の後に作ります。
- 成果物の証明は、ビルドしたジョブで、`.sha256` をまとめたもの (`subject-checksums`) を主体にして付けます。タグ (`release.yml` から
  呼ばれたとき) と手での起動の両方で付けます (手での起動で証明の付け方を予行できるように)。PR の CI (`ci.yml`) は配布物を作らない
  ので付けません。ブランチから手で起動したビルドにも証明は付くので、配った版を確かめるときは `--source-ref` でタグに絞ります (下)。
- 証明を付けるジョブにだけ `id-token: write` (Sigstore の証明書を得る OIDC のトークン) と `attestations: write` (証明をリポジトリに
  置く) を与えます。`release.yml` の呼ぶジョブ (`apps`・`ffi`) にも同じ権限を書きます (呼ばれたワークフローは、呼んだ側が渡した権限より
  強くできない)。
- 非公開のリポジトリでは、GitHub のプランによって成果物の証明を作れません。そのため証明のステップは、リポジトリが公開のとき
  (`github.event.repository.private` が偽) だけ動きます。非公開のうちはチェックサムだけを作ります。公開してから打つタグには証明が付きます。
- npm の `@ugoe/runtime` は、npm の来歴 (provenance) で足ります。`npm publish --provenance` は GitHub Actions の OIDC で同じ Sigstore の
  証明 (SLSA の来歴) を作って npm のレジストリに置き、npmjs.com のパッケージのページに出し、利用者は `npm audit signatures` で確かめます。
  tarball のダイジェストもレジストリが持ち、`npm install` がいつも確かめるので、`.sha256` や `actions/attest-build-provenance` を重ねません。
- crates.io のクレートは、crates.io の索引がダイジェストを持ち cargo が確かめるので、何も足しません。

### 確かめ方 (利用者)

チェックサム: ダウンロードしたファイルと `.sha256` を同じディレクトリに置きます。

```sh
# Linux (macOS は sha256sum の代わりに `shasum -a 256 -c`)
sha256sum -c ugoe_<版>_amd64.deb.sha256        # ugoe_<版>_amd64.deb: OK
```

```powershell
# Windows: 表示されたハッシュが .sha256 の最初の語と同じこと (大文字小文字は問わない)
(Get-FileHash .\ugoe_<版>_x64-setup.exe -Algorithm SHA256).Hash
Get-Content .\ugoe_<版>_x64-setup.exe.sha256
```

成果物の証明: [GitHub CLI](https://cli.github.com/) (`gh`) が要ります ([gh attestation verify](https://cli.github.com/manual/gh_attestation_verify))。

```sh
gh attestation verify ugoe_<版>_amd64.deb --repo rosylilly/ugoe
# 配った版のタグのビルドに絞る (ブランチから手で起動したビルドの証明を受け付けない)
gh attestation verify ugoe_<版>_amd64.deb --repo rosylilly/ugoe --source-ref refs/tags/v<版>
```

成功すると、証明を付けたワークフロー (`editor-build.yml`・`ffi-release.yml`) と、元のコミット・タグが表示されます。
`.sha256` はファイルの取り違え・壊れたダウンロードを見つけるもので、`.sha256` も同じ Release から来るので、Release そのものが
差し替えられたことは見つけられません。それを確かめるのが成果物の証明です。
確かめていないことは [manual-check.md の D 節](manual-check.md#d-開発者向け-ビルドci配布バインディング) の D14・D15。

## GitHub Release の下書き

`v<版>` のタグで動くワークフローは、配布物をそのタグの GitHub Release の **下書き** に添付します。Release を公開する (下書きを
公開に切り替える) のは人の手です。添付は `scripts/github-release.mjs` で揃えます。

```sh
node scripts/github-release.mjs upload <タグ> <ファイル...>   # GH_TOKEN (contents: write) と gh が要る
node scripts/github-release.mjs notes <タグ> <ファイル>       # 本文をファイルの中身にする
```

- そのタグの Release が無ければ、下書きで作ります (`gh release create --draft --verify-tag`。題はタグ、本文は書き直す前提の短い文)。
- いくつものワークフローが同じ下書きに添付します。同時に作って下書きが重なったら、いちばん古いものに寄せ、添付の無い余分な下書きを消します。
- 添付は、タグではなく選んだ下書き (いちばん古いもの) の id に対して行います。添付の始まった下書きが重なって残っても、どのワークフローも
  同じ下書きに添付するためです。
- 同じ名前のファイルは置き換えます (ジョブをやり直したとき)。公開済みの Release には添付せず、失敗します (配った物を差し替えないため)。
- 本文は `release-notes.yml` が `notes` で、`just release-notes <タグ>` の出力 ([変更履歴とリリースノート](#リリースノートとワークフロー)) に
  します。`upload` は本文に触らず、`notes` は添付に触りません。どちらも下書きが無ければ作り、重なりを寄せるので、ワークフローの
  動く順は問いません。公開済みの Release の本文は書き換えず、失敗します。`release-notes.yml` をやり直すと、人が書き直した本文も
  生成したノートで上書きするので注意します。
- ワークフローの側は、ビルドのジョブの後に `if: startsWith(github.ref, 'refs/tags/v')` のジョブを置き、`permissions: contents: write` で
  成果物を集めてこのスクリプトを呼びます (`editor-build.yml` の `release` のジョブ)。`release.yml` から呼ばれるので、`release.yml` の
  呼ぶジョブにも `permissions: contents: write` を書きます (呼ばれたワークフローは、呼んだ側が渡した権限より強くできない)。

| 添付するもの | ワークフロー |
| --- | --- |
| エディタ・ビューア・スタジオのインストーラ (`.msi`・NSIS の `.exe`・`.dmg`・`.deb`・`.rpm`・`.AppImage`) と、それぞれの `.sha256` ([配布物を確かめる](#配布物を確かめる)) | `editor-build.yml` |
| C ABI のアーカイブ (`ugoe-ffi-<版>-<プラットフォーム>.tar.gz`・`.zip` と `.sha256`。[C ABI の配布物](#c-abi-の配布物)) | `ffi-release.yml` |

公開の前に、人が次を行います。

1. 下書きに、配布物がそろっていることを確かめる (ワークフローがすべて終わってから)。
2. 本文 (生成したリリースノート) に、要約・既知の問題・署名の有無と、確かめ方 ([配布物を確かめる](#配布物を確かめる) へのリンク) を書き足す。
3. 「Publish release」で公開する。

## 変更履歴とリリースノート

変更履歴 (`CHANGELOG.md`) とリリースノートは手で書かず、Conventional Commits のコミット (commitlint で検査している。
[AGENTS.md#コミット](../AGENTS.md#コミット)) から [git-cliff](https://git-cliff.org) で作ります (T0086)。設定は `scripts/cliff.toml`、
入口は `scripts/changelog.mjs` です。

| コマンド | 内容 |
| --- | --- |
| `just changelog` | `CHANGELOG.md` を作り直す (タグの無いコミットは `Unreleased` の節) |
| `just changelog v1.0.0` | まだ打っていないタグを渡すと、未リリースのコミットをその版の節として書く (リリースの準備) |
| `just release-notes v1.0.0` | そのタグの範囲 (1 つ前の版のタグから。最初の版は最初のコミットから) のリリースノートを出す。タグがまだ無ければ未リリースのコミットの下見。`--output <file>` でファイルに書く |
| `just git-cliff-version` | 固定した git-cliff の版 |

### 道具

git-cliff (MIT OR Apache-2.0、Rust 製) を使います。外部の道具を足さず scripts/ の Node のスクリプトで作る案と比べて、
こちらを選んだ理由は次のとおりです。

- Conventional Commits の読み取り (scope・`!`・`BREAKING CHANGE:` のフッター)、タグの範囲の切り出し、節の並びを、設定と
  テンプレートだけで書ける。自前で書くと、フッターの解釈やタグの範囲の扱いを自分で持ち、テストし続けることになる。
- 出力の形はテンプレート (`scripts/cliff.toml` の `body`) で変えられ、コードを書き換えずに済む。

代わりに、手元と CI に git-cliff を入れる手間があります。次のように扱います。

- **版の固定:** 版は `scripts/changelog.mjs` の `GIT_CLIFF_VERSION` (今は 2.14.2) に 1 つだけ書きます。スクリプトは
  `git-cliff --version` がこの版でなければ、入れ方を出して止まります。手元では
  `cargo install git-cliff --version 2.14.2 --locked`、CI では `just git-cliff-version` の版を `taiki-e/install-action` で
  入れます (ビルド済みのもの)。同じ版・同じ設定・同じ履歴なら、手元と CI で同じ出力になります (日付はタグのコミットの日付)。
- **`just licenses` との関係:** git-cliff はワークスペースの依存 (`Cargo.lock`) にも npm の依存にも入らない、手元で使うだけの
  道具なので、`just licenses` の検査の対象になりません。配布物にも入りません。
- **`just ci` との関係:** `just ci` は git-cliff を呼びません (入っていない機械でも通るように)。設定の検査は単体テスト
  (`scripts/changelog.test.mjs`。commitlint のすべての type の扱いと、タグの形の一致) と、設定を変えた PR で動く
  `release-notes.yml` (実際に git-cliff で下見を作る) で行います。

版を上げるときは、出力を前の版と比べてから上げます (`just changelog` の差分を読む)。

### 分け方

コミットが英語なので、出力も英語です。

- **節:** `Breaking changes`・`Features` (feat)・`Bug fixes` (fix)・`Performance` (perf)・`Reverts` (revert) の順。
  利用者から見えない type (build・chore・ci・docs・refactor・style・test) は載せません。
- **破壊的な変更:** 件名の `!` か `BREAKING CHANGE:` のフッターがあるコミットは、type によらず (載せない type でも) 先頭の
  `Breaking changes` に集め、type の節には重ねて載せません。フッターの説明があれば、その行の下に字下げして載せます。
- **scope:** 行の頭に太字で出します (`- **runtime:** Evaluate ...`)。scope の無いコミットは件名だけ。
- **タスク ID:** 件名の末尾の `(T0123)` は文字のまま残し、リンクにはしません。タスクファイルの名前には ID の後に短い名前が付く
  (`docs/tasks/T0123-<slug>.md`) ので、ID だけからは安定した URL を作れないためです。ID でリポジトリを検索すれば辿れます。
- **コミット:** 各行の末尾に、短いハッシュと GitHub のコミットへのリンクを付けます。
- 件名の最初の文字は大文字にします (コミットの件名は小文字始まりの規約なので)。

### CHANGELOG.md の扱い

1.0 より前はタグが無く、すべてのコミットが `Unreleased` になるので、`CHANGELOG.md` は **最初のタグを打つまで作らず、コミット
もしません**。`Unreleased` の節をコミットしておくと、コミットのたびに古くなり、並行する作業の間で食い違うためです。

リリースのときは、[版を上げる](#版を上げる) のタグを打つ前に `just changelog v<版>` で作り直し、版を上げるコミットに含めます
(タグがそのコミットを指すので、`CHANGELOG.md` の最新の節とタグが一致します)。最初の版の節とリリースノートは、それまでの
すべてのコミットを並べるので長くなります (今の履歴で feat・fix・perf がおよそ 500 行)。必要なら、公開する前に Release の
本文の先頭へ人が要約を書き足します。

### リリースノートとワークフロー

`v*` のタグを push すると、`release.yml` が重い検査の後に `.github/workflows/release-notes.yml` を呼び、`just release-notes <タグ>` でそのタグの範囲のノートを作り、
ジョブの要約と成果物 (`release-notes`) に置き、`draft` のジョブがそのタグの下書きの Release の本文にします
(`node scripts/github-release.mjs notes`。[GitHub Release の下書き](#github-release-の下書き))。
手で起動したときと、`scripts/cliff.toml`・`scripts/changelog.mjs` を変えた PR では、未リリースのコミットを仮の版として下見を作ります
(下書きには触りません)。

## macOS の仮想カメラ

スタジオの macOS の仮想カメラ (Camera Extension、[studio.md](studio.md#macos-camera-extension)・ADR 0028 の 8.) を入れたスタジオの、
署名・公証・entitlement の手順です。**いまは配布しません。** 1.0 の配布物には入れず、CI で作って署名・公証し配布物に入れるのは
1.0 の後です ([1.0 の後の候補](roadmap.md#10-の範囲)。利用者の決定 2026-10-11)。入れるときは、利用者が Apple Developer Program の
アカウントとプロビジョニングプロファイルを用意し、署名の名義を決めてからです。それまでの配布物は `tauri.conf.json` のまま (拡張も
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

## C ABI の配布物

C・C++ やほかの言語から使う人のために、`ugoe-ffi` のライブラリとヘッダを OS ごとのアーカイブにして配ります
([portability.md#配布](portability.md#配布) の「C / その他」。T1408)。使い方は [crates/ugoe-ffi/README.md](../crates/ugoe-ffi/README.md)。

| アーカイブ | 作るランナー | `lib/` に入るもの |
| --- | --- | --- |
| `ugoe-ffi-<版>-linux-x86_64.tar.gz` | `ubuntu-24.04` | `libugoe_ffi.so.<ABI の版>`・`libugoe_ffi.so` (シンボリックリンク)・`libugoe_ffi.a` |
| `ugoe-ffi-<版>-linux-aarch64.tar.gz` | `ubuntu-24.04-arm` | `libugoe_ffi.so.<ABI の版>`・`libugoe_ffi.so` (シンボリックリンク)・`libugoe_ffi.a` |
| `ugoe-ffi-<版>-windows-x86_64.zip` | `windows-2022` | `ugoe_ffi.dll`・`ugoe_ffi.dll.lib` (DLL のインポートライブラリ)・`ugoe_ffi.lib` (静的ライブラリ) |
| `ugoe-ffi-<版>-macos-universal.tar.gz` | `macos-14` | `libugoe_ffi.<ABI の版>.dylib`・`libugoe_ffi.dylib` (シンボリックリンク)・`libugoe_ffi.a` (どれも arm64 と x86_64 を `lipo` でまとめた universal) |

- どのアーカイブも、最上位のディレクトリ `ugoe-ffi-<版>-<プラットフォーム>/` の下に `include/ugoe.h`・`lib/`・`examples/sample.c` (C の見本)・
  `LICENSE` (MIT)・`README.md` (中身とリンクの例。元は [crates/ugoe-ffi/archive/README.md](../crates/ugoe-ffi/archive/README.md))・
  `THIRD-PARTY-NOTICES.txt` (入っている Rust のクレートの表記。元は `crates/ugoe-ffi/THIRD-PARTY-NOTICES.txt`。[MIT の配布物](#mit-の配布物)) を持ちます。
  隣に SHA-256 の `<アーカイブ>.sha256` を置き、CI では成果物の証明も付けます ([配布物を確かめる](#配布物を確かめる))。
- 形は Windows が `.zip` (Windows に入っているもので開ける)、ほかが `.tar.gz` です。
- macOS は 1 つの universal にします (アーキテクチャごとに分けると、利用者が両方を `lipo` でまとめ直すことが多いため。
  unity.yml のプラグインと同じ形)。
- **共有ライブラリの名前に ABI の版を入れます (T1409)。** `<ABI の版>` は `ugoe_ffi::ABI_VERSION` (ヘッダの `UGOE_ABI_VERSION`。今は 3)。
  - Linux: 実体は `libugoe_ffi.so.<ABI の版>` で、soname も同じ。`libugoe_ffi.so` はそこへのシンボリックリンク (`-lugoe_ffi` でのリンク用)。
  - macOS: 実体は `libugoe_ffi.<ABI の版>.dylib` で、インストール名は `@rpath/libugoe_ffi.<ABI の版>.dylib`。`libugoe_ffi.dylib` は
    そこへのシンボリックリンク。
  - Windows: DLL には soname・インストール名に当たるものが無いので、`ugoe_ffi.dll` のまま。
  - 理由: リンクしたプログラムには soname・インストール名が残り、動的リンカはその名前のファイルを探す。ABI を壊した版
    (`UGOE_ABI_VERSION` を上げた版) のライブラリを同じ場所に入れても、名前が違うので、古い ABI でビルドしたプログラムが黙って
    新しいライブラリを読むことはなく、見つからないという分かりやすい失敗になる。ABI を壊さない版の更新 (関数を足すだけ) は名前が
    同じなので、入れ替えるだけで読める。版はライブラリの版 (`ugoe_version()`) でなく ABI の版にする (ABI を壊さない更新で
    リンクし直しを求めないため)。
  - macOS も同じ形にそろえる: dyld もインストール名でライブラリを探すので、Linux と同じ守りになる。`libfoo.<版>.dylib` と
    リンクのための `libfoo.dylib` は macOS の慣例の形で、利用者のリンクの書き方 (`-L lib -lugoe_ffi`) も Linux と同じになる。
  - 付けるのは配布物のビルド (`scripts/ffi-dist.mjs pack`) だけ。Linux は `cargo rustc` に `-Wl,-soname,…` を渡し、macOS は
    `install_name_tool -id` で付ける。同じスクリプトの `build-plugin` (Unity のプラグイン) には付けない ([ワークフロー](#ワークフロー))。
    `ugoe-ffi` のビルドすべて (build.rs) には付けない: Unity のプラグイン (unity.yml の Linux・Android の
    `libugoe_ffi.so`・macOS の `libugoe_ffi.dylib`) はファイル名を指定して読み込み、ほかのライブラリがそれにリンクすることは無いので
    soname は働かない。一方で Android の動的リンカは soname で読み込み済みのライブラリを区別するので、ファイル名と違う soname は
    思わぬ食い違いのもとになる。テスト・`just ffi-check`・適合テストは静的ライブラリを使い、Godot の GDExtension は `ugoe-ffi` を
    使わない。
  - 版は `crates/ugoe-ffi/src/lib.rs` の `ABI_VERSION` から読む (手で書いた数を持たない)。`pack` はそれがヘッダの `UGOE_ABI_VERSION` と
    一致するかを確かめてから付け、`verify` はアーカイブの中のヘッダの版で、シンボリックリンクが展開で保たれたか、soname
    (`readelf -d`)・インストール名 (`otool -D`) が版の付いた名前か、`-lugoe_ffi` でリンクした見本がその名前を記録したかを確かめる。
- Linux は glibc のランナーで作ります (musl は配りません)。
- **std の無い静的ライブラリ (T0491) は入れません。** std の無い環境 (ゲーム機・組み込み機器) はそれぞれのターゲット
  (`aarch64-unknown-none` など、SDK ごとのもの) 向けに作るもので、ここで作れるホストの OS 向けのものは使い道がありません。
  作り方は [crates/ugoe-ffi/README.md#std-の無い環境](../crates/ugoe-ffi/README.md#std-の無い環境) にあり、アーカイブの README からも案内します。
- Android・iOS のライブラリは入れません (Unity の UPM パッケージにあります)。
- 第三者のライセンスの表記 (依存するクレートのもの) はまだ入れていません (T1407)。

### 手元で作る

```sh
just ffi-dist                # target/ffi-dist/ に手元の OS のアーカイブと .sha256 を作る (cbindgen と C コンパイラが要る)
just ffi-dist --tag v0.1.0   # タグとルートの Cargo.toml の版が揃っているかも確かめる
just ffi-dist-check          # アーカイブを展開し、中のヘッダとライブラリだけで C の見本をビルドして動かす
```

`just ffi-dist` (`scripts/ffi-dist.mjs pack`) は、ヘッダが cbindgen の生成物と同じか (`just ffi-check` と同じ検査) を確かめてから、
`cargo build --locked --release -p ugoe-ffi --target <ターゲット>` で作ったライブラリを詰めます (ターゲットは `scripts/ffi-dist.mjs` の `BUILDS`)。
macOS では `aarch64-apple-darwin` と `x86_64-apple-darwin` のターゲット (`rustup target add`) が要ります (Linux では soname を付けるため
`cargo rustc --locked --release -p ugoe-ffi --lib --target <ターゲット>` で作ります)。
`just ffi-dist-check` (`verify`) は、ファイルの一覧と SHA-256・共有ライブラリの名前 (上の ABI の版) を確かめ、静的ライブラリと
共有ライブラリ (`-lugoe_ffi`) のそれぞれにリンクした見本を、見本のモデル (`assets/samples`) で動かします。手元の OS のアーカイブだけを調べられます。

### ワークフロー

`.github/workflows/ffi-release.yml` は、`v*` のタグ (`release.yml` が重い検査の後に呼ぶ) と手での起動で動きます。Windows・macOS の
ランナーを使うので、PR では動かしません (配布物のスクリプトの単体テストは `just script-check` が `ci.yml` で流す。T0091)。

1. `archive` ジョブ: 上の表のランナーごとに `pack` と `verify` を行い、アーカイブに成果物の証明を付け (公開のリポジトリのとき)、
   成果物 `ugoe-ffi-archive-<プラットフォーム>` に残します。タグのときは `--tag` でタグと版を確かめます。
2. `release` ジョブ (タグのときだけ): すべてのアーカイブと `.sha256` を、`scripts/github-release.mjs` でタグの GitHub Release の下書きに添付します
   ([GitHub Release の下書き](#github-release-の下書き))。

`unity.yml` も `ugoe-ffi` を OS ごとにビルドします。**ワークフローは分け、ビルドは `scripts/ffi-dist.mjs` を共有します (T1410)。**

- ワークフローを分けるのは、動く条件と成果物を独立に保つため (T1408)。`unity.yml` は Android・iOS を含む Unity のプラグインの並びを作り、
  こちらは静的ライブラリ・ヘッダ・検査を含みます。`ugoe-ffi` のビルドは 1 分ほどなので、重ねてもかかる時間はわずかです。
- ビルドの手順 (ターゲットごとの cargo・macOS の `lipo` とインストール名・できたファイルの名前) は `scripts/ffi-dist.mjs` の `BUILDS` と
  その関数にまとめ、`pack` (こちら) と `build-plugin` (`unity.yml` の `native`) が使います。片方だけ直して食い違うことを防ぐためです。
  `unity.yml` の `native` に残るのは、環境の用意 (rustup のターゲット・Android の cargo-ndk と NDK) と成果物のアップロードだけです。
- `build-plugin` は soname・版の付いた名前を付けません ([上](#c-abi-の配布物))。macOS のプラグインのインストール名は
  `@rpath/libugoe_ffi.dylib` (版の無いファイル名) です。
