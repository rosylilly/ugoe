# テスト方針

## 種類

| 種類 | 場所 | 実行 |
| --- | --- | --- |
| ユニットテスト | 各クレートの `#[cfg(test)] mod tests` | `just test` |
| 結合テスト | 各クレートの `tests/` | `just test` |
| スナップショットテスト | `insta` (`tests/snapshots/`) | `just test`。更新は `cargo insta review` |
| 互換性テスト | `crates/ugoe-format/tests/compat.rs` と資料 `assets/compat/` | `just test`。資料の作り直しは [互換性テスト](#互換性テスト) |
| 描画テスト (GPU) | `gpu-tests` feature を持つクレート (`ugoe-render` など) | `just test-gpu` |
| フロントエンドの検査 | Biome + svelte-check + Vitest (`apps/*/src/**/*.test.ts`・`packages/*/src/**/*.test.ts`。部品を描くテストは [下](#エディタの部品のテスト)) | `just web-check` (先に `just wasm-fresh` で古い wasm の生成物を作り直す。[下](#wasm-の生成物の古さ)) |
| wasm の型の写しの検査 | `scripts/check-wasm-types.mjs`。手書きの型の写し (`bindings/web/stub/ugoe_wasm.d.ts`・`@ugoe/player` と `@ugoe/face-tracking` の interface) を `just wasm` の生成物と比べる | `just wasm-types` (生成物が無ければ一部の写しをスタブと比べる。CI の web ジョブは `just wasm` の後に生成物と比べる) |
| 依存方向の検査 | `scripts/check-layers.mjs` | `just layers` |
| 名前の大文字と小文字の検査 | `scripts/check-case-collisions.mjs`。同じディレクトリに大文字と小文字だけが違う名前が無いか (macOS と Windows のファイルシステムでは同じファイルになる)。スクリプトの拡張子を取った import の名前も比べる (`FileMenu.svelte` と、`./fileMenu.svelte` で import する `fileMenu.svelte.ts` がぶつかる。T0088) | `just case-check` |
| リポジトリの URL の検査 | `scripts/check-repo-url.mjs`。このリポジトリを指す URL (owner か repo の片方だけが同じもの) と、パッケージの設定の `repository` が、ルートの `Cargo.toml` の `workspace.package.repository` と食い違わないか (T1003、[release.md](release.md#自動の検査)) | `just repo-url` |
| 機械に固有のパスの検査 | `scripts/check-local-paths.mjs`。ホームのディレクトリ (`/home/<名前>`・`/Users/<名前>`・`C:\Users\<名前>`) とエージェントの一時ディレクトリ (`/tmp/claude-<番号>/…`) が入っていないか (公開するため。例のパスは 1 文字の名前か `PLACEHOLDER_USERS` の名前を使う。T0090、[release.md](release.md#公開の前の点検))。pre-commit でも動く | `just local-paths` |
| 秘密の値の検査 | `scripts/secrets.mjs` (gitleaks。版と sha256 を固定して取得する) と設定 `scripts/gitleaks.toml` (既定の規則と、理由を書いた誤検知の allowlist)。作業ツリー (Git が無視するものを除く) とコミットに API キー・トークン・秘密鍵などが無いか (T0098、[release.md](release.md#自動の検査)) | `just secrets` (既定は履歴のすべて。CI の `rust-checks` は `--since` で push・PR で入るコミットだけ。初回の取得にネットワークが要る) |
| 参照の検査 | `scripts/check-doc-refs.mjs` (コードのコメントから `docs/`・`spec/` の見出しへの参照)・`scripts/check-ffi-bindings.mjs` (C ABI のヘッダと、Unity の宣言・fuzz の入口 `ugoe_ffi::fuzzing` の呼び出しの過不足) | `just refs` |
| CI のグループの検査 | `scripts/ci-groups.mjs` (すべてのクレートがどれか 1 つのグループに入っているか)・`scripts/check-lock-sync.mjs` (ワークスペースの外の `Cargo.lock` の依存の版がルートとそろうか。[fuzz](#fuzz)) | `just groups` ([CI のジョブ](#ci-のジョブ)) |
| 補助スクリプトの単体テスト | `scripts/*.test.mjs` のうち、ほかのレシピが流さないもの (mbx の判定・`--locked` で lock が古いときの文・npm のライセンスの例外・秘密の値の検査の道具の取得と範囲・wasm の生成物の古さの判定・worktree の cargo の設定) | `just script-check` |
| API ドキュメントの検査 | rustdoc の警告 (壊れた intra-doc link、公開の文書から非公開の項目へのリンク、URL の書き方など) をエラーにして `cargo doc --workspace --no-deps`。wasm32 でだけコンパイルするコード (`ugoe-wasm` の `src/web.rs`・`src/web/`) は wasm32 のターゲットで `cargo doc -p ugoe-wasm` | `just doc-check` (CI はグループごとに `just doc-group <グループ>`)。wasm32 の分は `just wasm-check` |
| UI の監査 | `tests/ui-audit`。3 つのアプリのビルドした画面を偽のホストとヘッドレスの Chrome で開き、テーマ・言語・幅・devicePixelRatio を変えて撮り、コントラスト・はみ出し・文字と押せる部品の重なり・押せる大きさ・名前・Tab の順を数える | `just ui-audit` (Chrome が要る。重いので `just ci` に入れない。道具の単体テストは `just ui-audit-check`。[下](#ui-の監査)) |
| 実機の e2e テスト | `tests/e2e`。本物の Tauri のアプリ (エディタ・ビューア・スタジオ。WebKitGTK) を起動し、WebDriver で主な流れを操作する | `just e2e` (Linux だけ。tauri-driver・WebKitWebDriver・Xvfb が要る。[下](#実機の-e2e-テスト)) |
| 配布物の適合テスト | `tests/conformance` (`ugoe-conformance`)。C ABI・wasm (npm パッケージ。WebGL2 と WebGPU)・Unity (C# の層)・Godot の頂点と画像を参照と比べる。`<ugoe-avatar>` の作り直しのシナリオと、問い合わせの JSON の出力のキーと TS の型の一致も本物のブラウザで確かめる | `just conformance` (C コンパイラ・Chrome・GPU が要る。dotnet・Godot は無ければ飛ばす。[下](#配布物の適合テスト)) |
| PuppetSurface のブラウザのテスト | `packages/player/browser-test`。エディタと Player が共有する `PuppetSurface` と `Player` を本物の wasm とヘッドレスの Chrome の WebGL2 で動かし、作成・描画・WebGL2 の喪失からの作り直し・諦め・破棄・devicePixelRatio の追従を確かめる | `just player-browser` (Chrome と wasm-bindgen-cli が要る。CI の `release-checks.yml` の `conformance` ジョブで流す。[下](#puppetsurface-のブラウザのテスト)) |
| エディタのブラウザのテスト | `apps/editor/browser-test`。ビルドしたエディタの画面を `just ui-audit` の偽のホストと本物の wasm とヘッドレスの Chrome の WebGL2 で動かし、GPU の喪失のまわり (プレビューの着せ替えの読み込みの途中の喪失と、失っている間の付け外し) を確かめる | `just editor-browser` (Chrome・wasm-bindgen-cli・cargo が要る。CI の `release-checks.yml` の `conformance` ジョブで流す。[下](#エディタのブラウザのテスト)) |
| fuzz | `fuzz/` (cargo-fuzz、ワークスペースの外)。バンドル・モジュール・画像・JSON の読み込みと、読めたモデルの評価 | `just fuzz <target> [秒]` (nightly と cargo-fuzz が要る。`just ci` に入れず、CI は `release-checks.yml` が呼ぶ `fuzz.yml`。[下](#fuzz)) |
| tier 3 のターゲットの build-std | `core`・`alloc` が配られない tier 3 のターゲット (`aarch64-nintendo-switch-freestanding` と、64 ビットのアトミック操作の無い `armv6k-nintendo-3ds`) で、nightly の `-Zbuild-std` で `ugoe-ffi` の `std` の無い静的ライブラリを作り、未解決のシンボルが決めた一覧と同じかを確かめる | `just nostd-build-std [ターゲット...]` (nightly と rust-src と llvm-tools が要る。`just ci` に入れず、CI は `release-checks.yml` が呼ぶ `nostd-build-std.yml`。[下](#tier-3-のターゲットの-build-std)) |
| 性能の検査 | `Puppet` の 1 フレームの評価の中央値を上限と比べる。ネイティブは `crates/ugoe/examples/puppet_timing.rs` を `--check` で、wasm (ブラウザ) は計測のページ `bindings/web/examples/timing/` を `measure.mjs --max-us 300` で | `just perf` (= `just perf-native` と `just perf-wasm`。時間は機械と負荷で揺れるので `just ci` に入れず、CI は `release-checks.yml` が呼ぶ `perf.yml`。wasm は wasm-bindgen-cli と Chrome が要る。[下](#遅くなったら気づく検査)) |
| C ABI の検査 | ヘッダ `crates/ugoe-ffi/include/ugoe.h` が最新か、C の見本 `crates/ugoe-ffi/examples/c/sample.c` のビルドと実行。`std` の無い形の静的ライブラリをホストの C ライブラリとリンクした見本 (`examples/c/nostd.c`) の実行 | `just ffi-check` (cbindgen が要る。[crates/ugoe-ffi/README.md](../crates/ugoe-ffi/README.md#ヘッダの生成)) |

`just ci` はこれらを (GPU テスト・C ABI の検査・配布物の適合テスト・fuzz・tier 3 のターゲットの build-std・性能の検査以外) すべて実行します。
C ABI の検査はワークスペースの外のツール (cbindgen) と C コンパイラが要るので `just ci` に入れず、CI の `rust (bindings)` ジョブで別に実行します ([CI のジョブ](#ci-のジョブ))。
配布物の適合テストも同じ理由で、CI の `release-checks.yml` の `conformance` ジョブで別に実行します (比べ合わせの単体テストは `just ci` に入る)。
PuppetSurface とエディタのブラウザのテストも wasm のビルドと Chrome が要るので `just ci` に入れず、同じ `conformance` ジョブで流します。
実機の e2e テストは 3 つのアプリをビルドし直すので、push・PR ごとの CI に入れず、`release-checks.yml` が呼ぶ `e2e.yml` で流します ([下](#実機の-e2e-テスト))。
UI の監査も 3 つのアプリをビルドして 200 ほどの場面を撮るので `just ci` に入れず、画面を変える PR で人とエージェントが流します ([下](#ui-の監査))。

## wasm の生成物の古さ

エディタ・ビューア・スタジオの wasm の生成物 (`apps/editor/src/lib/viewport/generated`・`apps/viewer/src/lib/wasm/generated`・
`apps/studio/src/lib/wasm/generated`) は git の管理外で、`just wasm <out>` (各アプリの `pnpm wasm`) で作ります。`@ugoe/runtime` の生成物
(`bindings/web/src/wasm`) も git の管理外で、`just web-runtime` (`pnpm --filter @ugoe/runtime build`) で作ります。pull や merge で
ugoe-wasm の API が変わっても作業ツリーには前の生成物が残るので、そのままだと svelte-check や `@ugoe/runtime` の tsc が
コードの誤りでなく生成物の古さで落ちます (T0062・T0082)。そこで:

- `just wasm` (`scripts/wasm.mjs build`) と `just web-runtime` (`bindings/web/scripts/build.mjs`。同じ `fingerprint` を使う) は
  生成物の隣に、入力の指紋 `ugoe_wasm.fingerprint` を書きます。`@ugoe/runtime` の `dist/` には写しません。入力は ugoe-wasm と、
  それが (dev 以外で) 依存するワークスペースのクレートの `Cargo.toml`・`build.rs`・`src/` の下のすべてのファイルと、
  ルートの `Cargo.toml`・`Cargo.lock` (wasm-bindgen の版を含む)・`rust-toolchain.toml` です。テスト・例・README は入れません。
- `just wasm-fresh` (`scripts/wasm.mjs fresh`) は、ビルドせずに今の指紋を求めて (100 ほどのファイルを読むだけで、数十ミリ秒) 4 つの生成物と比べ、
  古いものだけを作り直します。指紋の無い生成物 (指紋を書く前に作ったもの) も古いとみなします。生成物の無いもの (wasm 本体
  `ugoe_wasm_bg.wasm` の無いもの) は飛ばします (型検査は `bindings/web/stub` を読む。`@ugoe/runtime` の検査が `src/wasm` に置く
  スタブの写しも生成物とはみなさない)。`@ugoe/runtime` の生成物は配布用のプロファイル `web-dist` で作るので、
  `bindings/web/scripts/build.mjs --wasm-only` で作り直します (`dist/` は作らない。fat LTO なので、mbx の置き場に当たらなければ手元で 2 分ほど)。
  確かめるだけなら `node scripts/wasm.mjs check` (古ければ終了コード 1)。`pnpm --filter @ugoe/runtime check` を直接流したときは、
  生成物が古ければ警告を出します (作り直しはしない)。
- `just web-check` (`just ci`) と lefthook の pre-push の web-check は、先に `just wasm-fresh` を流します。作り直しは止めずに行います。
  ugoe-wasm の wasm32 の release ビルド (ほかの差分が無ければ手元で数秒〜数十秒。mbx の置き場に当たれば数秒) と wasm-bindgen だけで、
  pre-push の clippy と全テストより軽いためです (`@ugoe/runtime` の作り直しは fat LTO で重いが、`just web-runtime` を一度でも流した
  作業ツリーでしか起きない)。pre-push の web-check はアプリと `@ugoe/runtime` などの型検査を流します。pre-push のように just を通らずに `scripts/wasm.mjs` を呼んでも、mbx の shim が入っていれば
  cargo は mbx を通ります (`exportMbxPath`。[docs/workflow/build-cache.md](workflow/build-cache.md))。
- 作り直すには wasm-bindgen-cli (`Cargo.lock` と同じ版) が要ります。無ければ入れ方と、代わりに古い生成物を消す (`rm -r <生成物>`。
  型検査はスタブを読む) コマンドを出して止まります。

判定の単体テストは `scripts/wasm.test.mjs` (`just script-check`) です。小さな偽のワークスペースで、指紋の書き換え・指紋の無い生成物・
入力の変更 (ソース・シェーダ・Cargo.toml・Cargo.lock・rust-toolchain.toml) を古いと見つけ、テスト・README・dev の依存だけのクレートの
変更と、生成物の無いもの (型だけのスタブの写しも) を飛ばすことと、`@ugoe/runtime` の生成物も確かめることを見ます。

## CI のジョブ

CI のワークフローは、push・PR ごとに動く軽い検査と、手での起動・定期・リリースの前に動く重い検査に分けています
(利用者の決定 2026-10-11。T0091)。重いビルド (Windows・macOS・GPU・アプリなど) を毎回の push で流さないためです。
GitHub Actions の時間は、非公開のリポジトリでは月 3000 分の無料枠から、Windows は 2 倍・macOS は 10 倍で数えて引かれます
(公開すると標準のランナーは無料になるが、方針は同じ)。
公開するまでは、オーケストレーターが main への push をまとめて回数を減らします (1 日に 1〜2 回まで。
[multi-agent.md](workflow/multi-agent.md#main-を-push-する区切り)。T0099)。公開してからは PR を通す運用に移り、`ci.yml` を必須の検査にします。

| ワークフロー | 動くとき | 内容 |
| --- | --- | --- |
| `ci.yml` (軽い) | main への push・PR | 手元の `just ci` に当たる検査を、Linux のランナーだけで流す。変わったパスで絞る ([下](#変わったパスでの絞り込み)) |
| `release-checks.yml` (重い) | 手での起動 (`workflow_dispatch`)・定期 (毎月 1 日と 15 日。main が前に通った run から変わっていなければ飛ばす)・リリースの前 (`release.yml` が呼ぶ) | 別のターゲット・Windows と macOS・GPU・Godot・配布物の適合テスト・e2e・Unity・perf・tier 3 の build-std・fuzz・固定した道具の新しい版の表 ([release.md](release.md#固定した道具の版)。落ちない) ([下](#重い検査とリリースの前の検査)) |
| `release.yml` | `v*` のタグ | `release-checks.yml` を通してから配布物のワークフローを呼ぶ。検査が落ちたら配布物を作らない ([release.md](release.md)) |
| `release-notes.yml` | リリースノートの設定・スクリプトを変えた PR (と手での起動・`release.yml`) | git-cliff でノートの下見を作る (Linux で 1 分ほど。[release.md](release.md#リリースノートとワークフロー)) |

`ci.yml` の Rust の検査は、クレートのまとまり (グループ) ごとの別のジョブで行います。
ワークスペース全体のテストを 1 つのジョブでビルドすると、GitHub Actions のランナーのディスク (空きは約 14GB) を超えるためです
(デバッグ情報ありで約 17GB。T0002)。グループの定義の正は `scripts/ci-groups.mjs` の `GROUPS` で、CI の matrix もここから作ります。

| ジョブ (`ci.yml`) | 内容 |
| --- | --- |
| `rust-checks` | ワークスペースをビルドしない検査と文書の検査 (変わったパスによらず常に動く): fmt・`just layers`・`just case-check`・`just local-paths`・`just repo-url`・`just secrets --since <push の前か PR の base>`・`just refs`・`just groups`・`just licenses` (cargo-about を `just cargo-about` で入れ、実行ファイルを版ごとに覚える)・`just publish-check`・`just tasks`・`just app-build-check`・`just script-check`・`just ui-audit-check`・公開仕様の検査 (`just spec` と `tools/spec-check`) |
| `rust-groups` | 変わったファイルから、動かすグループの matrix と `web` を動かすかを決める (`node scripts/ci-groups.mjs changed`。[下](#変わったパスでの絞り込み)) |
| `rust (runtime)` | ランタイム層 (`ugoe-core` から `ugoe` まで) の clippy とテスト、参照レンダラを外した `ugoe` の clippy とテストと wgpu に依存しないことの検査 (`just sdk-no-render-check`。[portability.md](portability.md#ugoe-クレートの-feature))、`std` 無しのビルド (`just nostd-check`。`aarch64-unknown-none` と、64 ビットのアトミック操作の無い `thumbv7em-none-eabihf`。[portability.md](portability.md#ランタイム層が守る制約-移植性のため))。`ugoe-format`・`ugoe-ffi` の `std` の無い形の単体テスト (ホストで `--no-default-features`。下の「std の無い形の単体テスト」) と、`ugoe-ffi` の `std` の無い静的ライブラリを C の見本とリンクし、未解決のシンボルを調べる検査を含む ([crates/ugoe-ffi/README.md](../crates/ugoe-ffi/README.md#std-の無い環境))。fuzz のクレート (`fuzz/`) の stable での検査 (`just fuzz-lint`。[fuzz](#fuzz)) も |
| `rust (bindings)` | `ugoe-ffi`・`ugoe-wasm`・`ugoe-conformance` の clippy とテスト、wasm32 の clippy と rustdoc (`just wasm-check`)、C ABI (`just ffi-check`) |
| `rust (assets)` | `ugoe-psd`・`ugoe-mesher`・`ugoe-rigging`・`ugoe-headless` の clippy とテスト |
| `rust (tools)` | `ugoe-fs`・`ugoe-editor-core`・`ugoe-ai`・`ugoe-cli`・`ugoe-mcp` の clippy とテスト |
| `rust (apps)` | エディタ・ビューア・スタジオ (Tauri) の clippy とテスト、生成物の検査 (`just gen && git diff --exit-code`)。Tauri のシステムの依存はこのジョブでだけ入れる |
| `rust` | 上の Rust のジョブの結果をまとめる (ブランチの保護で必須にする検査の名前を、グループが増えても変えないため)。変わったパスでグループを飛ばしたときは成功とみなす |
| `web` | フロントエンドの lint・型検査・単体テストと、wasm を作ってからのアプリと `@ugoe/runtime` のビルド (`just web-check` に当たる)。文書だけの変更では動かない |
| `commitlint` | PR のコミットメッセージ |

- 手元では `just lint-group <グループ>` と `just test-group <グループ>` で、CI の 1 つのジョブと同じ clippy とテストを走らせられます。`just ci` は今までどおりワークスペース全体を検査します。
- `rust (<グループ>)` の各ジョブは、clippy とテストの後に `just doc-group <グループ>` で、そのグループのクレートの API ドキュメントを rustdoc の警告をエラーにして作ります (`just ci` の `just doc-check` のグループの分。T0027)。依存はテストのビルドと同じ check の成果物を使うので、ディスクはあまり増えません。角括弧 (`[0,1]²` など) は intra-doc link と読まれるので、バッククォートで囲むか `\[` と書きます。非公開の項目は公開の文書からリンクせず、バッククォートだけで書きます。
- `ugoe-cli` の bin は名前が `ugoe` で SDK (`ugoe` クレート) の lib と出力先 (`target/doc/ugoe/`) が同じなので、`doc = false` で API ドキュメントを作りません。cargo は rustdoc の前に `target/doc/<クレート>` を消すため、衝突したままだと `cargo doc --workspace` の中で 2 つの rustdoc が互いの出力を消し、`just doc-check` がまれに失敗し、`just doc` で開く `ugoe` のページが CLI のものになることがありました (T0033。[cargo#6313](https://github.com/rust-lang/cargo/issues/6313))。bin の名前を lib と同じにするときは同じく `doc = false` を付けます。
- `ugoe-wasm` の `src/web.rs`・`src/web/` (`WebPuppet` など) は `cfg(target_arch = "wasm32")` の中にあり、ホストの `doc-check` ではコンパイルされません。`just wasm-check` が wasm32 のターゲットで `cargo doc -p ugoe-wasm --no-deps` を rustdoc の警告をエラーにして流します (CI は `rust (bindings)`。T0033)。ホストの文書から wasm32 だけの項目へはリンクできない (ホストの `doc-check` で壊れたリンクになる) ので、バッククォートだけで書きます。
- `just script-check` (scripts/ の単体テスト) は Node だけで動くので、ワークスペースをビルドしない `rust-checks` のジョブで流します (T0027)。`just groups`・`just app-build-check` が流すテストは、それぞれのレシピの側に入っています。
- clippy (`just lint`・`just lint-group`) は、既定の feature と、すべての feature (`--all-features`) の 2 回流します。`gpu-tests` などの feature の下のコード (GPU のテストなど) は既定の feature ではコンパイルされず、警告が溜まっていたためです (T0211・T0013)。グループでの `--all-features` は、選んだクレートの feature だけを有効にします。
- `-p` で選んだクレートの間でだけ feature が統合されるので、ワークスペース全体では別のクレートが有効にする feature (例: `ugoe-editor-core` が有効にする `ugoe-model/schema`) は、グループの `features` に書きます。`required-features` を持つテストや例 (`gpu-tests` を除く) の feature が書かれていなければ `just groups` が失敗します。`just crate <クレート>` も、グループの `features` のうちそのクレートのもの (`ugoe-format/png` など) を有効にして clippy とテストを流します (T0186)。
- 各ジョブは、最初にランナーの使わない SDK (.NET・Android SDK・Haskell・CodeQL) を消してディスクを空けます。
- CI では `CARGO_PROFILE_DEV_DEBUG=0` でデバッグ情報を作りません (ci.yml・release-checks.yml などのワークフロー全体の `env`)。target とキャッシュがおよそ半分になります。パニックの位置 (ファイルと行) はデバッグ情報がなくても表示されますが、バックトレースには行番号が出ません。
- 依存クレートの成果物は `Swatinem/rust-cache` で持ち越します。方針は下の「CI のキャッシュ」。
- nightly が要る検査 (`fuzz.yml`・`nostd-build-std.yml`) と、時間で判定する性能の検査 (`perf.yml`。ランナーの揺れで関係の無い変更を
  落とさないため) は、push・PR ごとの CI に入れず、`release-checks.yml` の一部として流します ([下](#重い検査とリリースの前の検査))。

各グループの target の大きさの目安 (T0002 で手元で測った値。`CARGO_PROFILE_DEV_DEBUG=0` で clippy とテストのビルドの後):

| グループ | target |
| --- | --- |
| runtime | 約 0.8GB |
| bindings | 約 0.9GB (wasm32 の clippy を足して約 1.2GB) |
| assets | 約 0.7GB |
| tools | 約 1.4GB |
| apps | 約 2.7GB (`just gen` を足して約 2.8GB) |

### 変わったパスでの絞り込み

`ci.yml` の `rust-groups` のジョブが `node scripts/ci-groups.mjs changed <base> HEAD` で、変わったファイルから動かすものを決めます
(`affected`)。base は、push では push の前のコミット (`github.event.before`)、PR では PR のマージのコミットの 1 つ目の親です。
前のコミットが取れなければ (新しいブランチ・履歴の書き換え) すべてを動かします。`rust-checks` は絞らず、いつも動きます。

| 変わったもの | 動かすもの |
| --- | --- |
| 文書だけ (`docs/` のファイルと、クレートの外の `.md`。下の「テストが読む文書」を除く) | なし (`rust-checks` だけ) |
| クレートのディレクトリの中 (`crates/<名前>/`・`apps/<アプリ>/src-tauri/` など。README も) | そのクレートと、それに依存する (開発時の依存を含む) クレートのグループと `web` |
| `apps/<アプリ>/` の src-tauri の外 (フロントエンド・翻訳・設定) | そのアプリのクレートの変更として扱う (src-tauri のテストが翻訳や `tauri.conf.json` を読む) |
| `packages/`・`bindings/web/`・`tests/e2e/`・`tests/ui-audit/`・`tools/` | `web` だけ |
| テストが読む文書 (`DOCS_READ_BY_TESTS`: `docs/editor.md`・`docs/spec/`・`docs/studio-api.schema.json`) | 読むクレートの変更として扱う (`web` は動かさない) |
| それ以外 (`Cargo.toml`・`Cargo.lock`・`rust-toolchain.toml`・`justfile`・`scripts/`・`.github/`・`assets/`・`spec/`・`fuzz/`・`bindings/unity/` など) | すべてのグループと `web` |

- グループの `covers` は、そのグループのジョブがほかのグループのクレートも検査するときに書きます (`runtime` の `just nostd-check` と
  `just fuzz-lint` は `ugoe-ffi` も検査するので、`covers: ["ugoe-ffi"]`)。
- Rust のテストが文書を読むようになったら (`include_str!("../../docs/…")` など)、`DOCS_READ_BY_TESTS` に足します。足し忘れると
  `just groups` が、ソースの中の文字列の全体が `docs/…`・`spec/…` のパスであるものを見つけて失敗します (クレートの外のほかの
  ファイルを読むテストは、そのファイルが「それ以外」に入るので絞り込みで飛ばされません)。
- 2026-09-28〜10-11 の main への push (17 回) は、それぞれ数十〜数百のコミットをまとめたもので、ほとんどが `scripts/`・`.github/`・
  `Cargo.lock` などを含み、絞り込みですべてのグループが動きました (`tools`・`apps` だけになったのは 2 回)。絞り込みが効くのは、
  文書だけの push と、小さな PR です。

### 重い検査とリリースの前の検査

`release-checks.yml` のジョブと、2026-10-11 の main の run で測った時間 (キャッシュが当たったとき。課金はジョブごとに分に切り上げ、
Windows は 2 倍・macOS は 10 倍。T0091):

| ジョブ | 内容 | 時間 | 課金の分 |
| --- | --- | --- | --- |
| `gate` | 定期の run だけ、前に通った run と main が同じなら残りを飛ばす | 数秒 | 1 |
| `rust (cross)` | 別のターゲットの clippy ([下](#別のターゲットの検査)) | 約 2 分 | 3 |
| `rust (windows)`・`rust (macos)` | Windows と macOS のランナーでの、ファイルを扱うクレートの clippy とテストと C ABI ([下](#windows-と-macos)) | 約 9 分・約 2.5 分 | 18・30 |
| `gpu (vulkan)`・`gpu (gl)` | 描画テスト ([描画テスト](#描画テスト)) | 約 10 分ずつ | 20 |
| `godot` | Godot のバインディング ([bindings/godot/README.md](../bindings/godot/README.md)) | 約 9 分 | 10 |
| `conformance` | 配布物の適合テストと、ブラウザの PuppetSurface とエディタ ([配布物の適合テスト](#配布物の適合テスト)) | 約 7.5 分 | 8 |
| `e2e` (`e2e.yml`) | 実機の e2e テスト ([実機の e2e テスト](#実機の-e2e-テスト)) | 約 24 分 | 24 |
| `unity` (`unity.yml`) | Unity のパッケージの検査と、7 つのプラットフォームの `ugoe-ffi` (Windows 1・macOS 2) | 約 13 分 (合計) | 49 |
| `perf` (`perf.yml`) | 1 フレームの評価の時間 ([遅くなったら気づく検査](#遅くなったら気づく検査)) | 約 5 分 | 6 |
| `nostd-build-std` (`nostd-build-std.yml`) | tier 3 のターゲットの build-std ([下](#tier-3-のターゲットの-build-std)) | 約 2 分 | 3 |
| `fuzz` (`fuzz.yml`) | 5 つのターゲットを 5 分ずつ ([fuzz](#fuzz)) | 約 8 分ずつ (見込み) | 約 40 |
| `tool-updates` | 版を手で固定した道具を上流の最新の版と比べ、表をジョブの要約に書く (落ちない。`gate` を待たず定期の run でも毎回動く。[release.md](release.md#固定した道具の版)) | 1 分未満 (見込み) | 1 |

- 1 回の run は課金で約 210 分 (キャッシュが外れると、Windows・macOS の依存のビルドで約 330 分まで)。定期は月に 2 回なので約 420〜660 分です。
  キャッシュは main の run でだけ保存するので、定期の run が次の定期の run のためのキャッシュを作ります (7 日使われないキャッシュは
  GitHub が消すので、半月おきの run ではたいてい外れる見込み)。
- `e2e.yml`・`unity.yml`・`perf.yml`・`nostd-build-std.yml`・`fuzz.yml` は、`workflow_call` で呼ばれるほか、それぞれ単独で手で起動できます。
  呼ばれたワークフローの `github.workflow` は呼んだ側の名前になるので、各ワークフローの `concurrency` のグループの名前は書いて決めます
  (`${{ github.workflow }}` のままだと呼んだ側と同じグループになり、互いを待って止まる)。
- `ci.yml` (軽い) の 1 回の push は、キャッシュが当たると課金で約 35 分、外れると約 70 分です (グループの絞り込みの前。
  T0091 の前は 1 回の push で `ci.yml`・`e2e.yml`・`unity.yml` が動き、約 160〜230 分。ほかに毎晩の `e2e.yml` が約 24 分)。
- 自前のランナー (self-hosted) は使いません (公開すると、フォークからの PR で自前の機械が使われる危険があるため)。
### CI のキャッシュ

依存クレートの成果物は [`Swatinem/rust-cache`](https://github.com/Swatinem/rust-cache) で持ち越します。リポジトリのキャッシュは合計 10GB が上限で、
超えると使われていないものから消されます ([GitHub の文書](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching#usage-limits-and-eviction-policy))。
上限は引き上げず、キャッシュの数と大きさを抑えます (T0051)。

- **キー:** rust-cache のキーは `v0-rust-<shared-key>-<OS>-<アーキテクチャ>-<環境のハッシュ>-<Cargo.toml と Cargo.lock のハッシュ>` です。
  環境のハッシュは rustc の版と `CARGO`・`RUST`・`CC` などで始まる環境変数から作るので、キーを共有するジョブは環境変数を揃えます
  (`CARGO_PROFILE_DEV_DEBUG` を ci.yml・release-checks.yml などのワークフロー全体に置くのはこのため)。最後のハッシュは `Cargo.toml` が変わるたびに変わり、
  そのときは最後のハッシュを除いた前方一致で、前のキャッシュから部分的に戻します。
- **固定の版だけを鍵に入れる:** rust-cache (v2.9.0 から) は環境のハッシュに、作業ディレクトリの `rustc -vV` (`rust-toolchain.toml`
  の版) に加えて `rustup toolchain list` のすべてのツールチェインの版を入れます ([src/config.ts の `getRustVersions`](https://github.com/Swatinem/rust-cache/blob/v2.9.2/src/config.ts))。
  ランナーのイメージの stable が残っていると、イメージが stable を上げるたびにすべてのキーが外れ、依存をすべてビルドし直します
  (2026-10-11 の run 38098330681 で、ログの「Rust Versions」に固定の 1.98.1 とイメージの 1.99.0 が並び、`rust (apps)` が
  いつもの約 6 分から約 26 分になった。T0091)。そこで rust-cache を使うジョブは、rust-cache の前に
  `.github/actions/rust-toolchain` (固定の版を入れて既定にし、ほかのツールチェインを消す) を置きます (T1000)。
  `add-rust-environment-hash-key: false` は `Cargo.toml` と `Cargo.lock` のハッシュも鍵から外し、鍵が変わらなくなって
  キャッシュが新しくならない (完全一致では保存しない) ので使いません。nightly が要る fuzz.yml は、この手順の後に nightly を入れるので、
  nightly の版も鍵に入ります (fuzz の target は nightly でビルドするので、nightly が変われば使えない)。
  鍵に入った版は、ジョブのログの rust-cache の手順の「Cache Configuration」の「Rust Versions」で確かめられます (1 行だけなら固定の版だけ)。
- **main でだけ保存する:** `save-if: ${{ github.ref == 'refs/heads/main' }}`。PR やタグの run が保存したキャッシュは、その ref
  (と後の同じ ref の run) からしか読めず、容量を食うだけです ([GitHub の文書](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching#restrictions-for-accessing-a-cache))。
  PR とタグの run は main のキャッシュから始まります。
- **ビルドの重なるジョブはキーを共有し、保存は 1 つのジョブだけにする:** 同じキーで並んで動くジョブは、先に終わったものだけが保存します。
  そこで、ビルドがほかのジョブのビルドに含まれるジョブは、そのキーを `save-if: false` で読むだけにします。

  | 保存するジョブ (`shared-key`) | 読むだけのジョブ |
  | --- | --- |
  | `rust (tools)` (`group-tools`) | `gpu (vulkan)`・`gpu (gl)` (テストのビルドの依存が `rust (tools)` のテストのビルドに含まれることを手元で確かめた) |
  | `web` (`web`) | web-runtime-release.yml (`@ugoe/runtime` の web-dist)、e2e.yml (アプリの `pnpm wasm` の `ugoe-wasm` の wasm32 の release) |
  | `conformance` (`conformance`。release-checks.yml) | `godot` (参照の画像を描く `ugoe-cli`・`ugoe-conformance`・`ugoe-ffi`) |
  | `rust (apps)` (`group-apps`) | e2e.yml (アプリの dev の依存。下の「e2e.yml は自分のキャッシュを持たない」) |

  `ci.yml` (push ごと) のジョブが保存するキャッシュは push のたびに新しくなり、release-checks.yml (月に 2 回と手での起動) のジョブが
  保存するもの (`conformance`・`cross`・`rust (windows)`・`rust (macos)`・`godot-gdextension`・`perf`・`fuzz`) はその run のときだけ
  新しくなります。web-runtime-release.yml と e2e.yml がいつも新しいキャッシュを読めるように、`ugoe-wasm` の wasm32 のビルドは
  push ごとに動く `web` のジョブのキャッシュに置きます (T0091 の前は `conformance` のジョブが push ごとに保存していた)。

- **`godot` の GDExtension は別のキャッシュにする:** rust-cache は保存の前に、`workspaces` に書いたワークスペースの
  「依存でないもの」(`cargo metadata` でそのワークスペースの外にあるパッケージ以外) を target から消します
  ([src/save.ts・src/cleanup.ts](https://github.com/Swatinem/rust-cache))。GDExtension (`bindings/godot` はワークスペースの外、
  ADR 0026) をルートの `target/` でビルドすると、その依存 (godot-rust など) は保存の前に消えて毎回コンパイルされていました
  (dev 約 3.5 分・release 約 2.5 分。T0051)。そこで CI (`CI` がある) では `bindings/godot/build.mjs` が GDExtension だけを
  `bindings/godot/target/` でビルドし、`godot` のジョブは rust-cache を 2 つ使います。1 つ目は今のとおり `conformance` のキーを読むだけ
  (ルートの `target/`。参照の画像を描くクレート)、2 つ目は `shared-key: godot-gdextension`・`workspaces: bindings/godot -> target` で
  main でだけ保存します。1 つの手順に 2 つの workspace を書くと、キーを `conformance` と共有して読むだけにする (GDExtension の依存を
  保存できない) か、`godot` だけのキーにする (参照のクレートの分を `conformance` のキャッシュと重ねて保存し、容量が増える) かになるので、
  手順を分けました。2 つ目のキーの最後のハッシュは `bindings/godot` の `Cargo.toml` と `Cargo.lock` (と `rust-toolchain.toml`) から
  作るので、ルートの `Cargo.toml` が変わっても変わりません。大きさは dev と release の依存で約 0.4GB の見込みです
  (手元で `CARGO_PROFILE_DEV_DEBUG=0` でビルドし、保存の前の掃除を真似た target を gzip して約 0.37GB。T0056)。
- **失敗したジョブでも保存する (`cache-on-failure: true`) のは `rust (windows)`・`rust (macos)` だけ:** 成功するまで保存しないと、
  テストが落ち続ける間は毎回依存をすべてビルドする (Windows で約 16 分) ためです。依存がそろってから保存されるように、
  後の手順は前の手順が落ちても流します (`if: ${{ !cancelled() }}`)。ほかの Linux のジョブには付けません。
  Linux のジョブは main で前に保存したキャッシュから前方一致で戻せ、コンパイルの誤りで途中で止まったビルドを保存すると、
  次に `Cargo.toml` が変わるまでその不完全なキャッシュが完全一致で使われ続けるためです。
  `rust (macos)` のキャッシュには `just ffi-check` が作る `std` の無い形 (プロファイル `nostd`) の依存も入ります (手元の Linux で
  `nostd` の依存を gzip して約 36MB。`std` がある形の依存は `just test-os` のビルドとほぼ重なり、増えるのは 10 個ほど。T0067)。
  `rust (windows)` は `std` がある形だけを作るので、増えるのはその 10 個ほどです (T0069)。
- **キャッシュを使わないジョブ:** unity.yml (`check` と `native`。`ugoe-ffi` の依存は少なく、ビルドの時間の大半はキャッシュされない
  ワークスペースのクレートなので、キャッシュがあっても速くならない)、editor-build.yml (タグか手での起動でだけ動く。アプリ × OS の
  リリースビルドのキャッシュは大きく、上限を圧迫する)。
- **perf.yml と fuzz.yml も main でだけ保存する:** release.yml (タグ) から呼ばれたときに保存すると、そのタグからしか読めない
  キャッシュ (perf の release のビルドは約 0.35GB) が上限を圧迫するためです。
- **e2e.yml は自分のキャッシュを持たない:** rust-cache を 2 つ使い、`web` と `group-apps` のキーを `save-if: false` で
  読むだけにします (T0059)。`tests/e2e/run.mjs` は、アプリごとの `tauri build` ではなく、各アプリの beforeBuildCommand
  (`pnpm wasm && pnpm build`) の後に、選んだアプリ (と隣に置く `ugoe-mcp`) を 1 回の
  `cargo build --bins --features tauri/custom-protocol -p ugoe-editor -p ugoe-mcp -p ugoe-viewer -p ugoe-studio` でビルドします
  (引数は `tauri build` が中で流すものと同じ。[tests/e2e/README.md](../tests/e2e/README.md#流れ))。アプリをまとめると feature の統合が
  `rust (apps)` (4 つのクレートを 1 回でビルドする) と揃い、違うのは `tauri/custom-protocol` で変わる tauri・プラグインなどの依存
  17 個だけになります (手元で `rust (apps)` の手順の後に 26 秒。アプリを 1 つずつ `tauri build` していたときは約 170 個)。
  `pnpm wasm` の `ugoe-wasm` の wasm32 の release は `web` の `just wasm` と同じコマンドなので、そのキャッシュに入っています。
  2 つの rust-cache は同じ `target/` に続けて戻し (同じ名前の成果物は同じ中身)、どちらの環境のハッシュにも 1 つ目の restore が置く
  `CARGO_INCREMENTAL=0` が入ります (ci.yml の 1 つの rust-cache の restore も、キーを作る前に置く) ので、キーは保存する側と揃います。
  e2e.yml は release-checks.yml から動くので、`Cargo.toml` が変わってから main への push で ci.yml が新しいキーを保存するまでの間は、
  前のキーから前方一致で戻して差分をコンパイルします。
- **容量の目安:** main の 1 組 (同じ `Cargo.toml` のハッシュのキャッシュ) が上限の半分 (5GB) 程度に収まるようにします
  (`Cargo.toml` が変わった直後は新旧の 2 組が並ぶため)。測るには `gh api repos/{owner}/{repo}/actions/cache/usage` と
  `gh cache list --limit 100`。T0051 の前 (2026-10-06) は 1 組が約 8.7GB (rust-cache が 21 個) で、2 組目で上限を超えていました。
  `godot-gdextension` (約 0.4GB) はルートの `Cargo.toml` では変わらないので、2 組が並ぶときも 1 個のままです。
  見積もりでは 1 組が約 4.6GB です (e2e のキャッシュ (約 1.1GB) をやめた。T0059)。2 組が並んで上限を超えたときに消されるのは
  最後に使われた日の古いもの (新しい組がすでに保存された古い組の分) なので、当たりへの影響は小さいと見ています (T0057)。

### 別のターゲットの検査

`#[cfg(windows)]`・`#[cfg(target_os = "macos")]` の中のコードは、ubuntu の `rust-group` のジョブと Linux の手元ではコンパイルされず、
clippy の拒否 (仮想カメラの `clippy::undocumented_unsafe_blocks` など) も効きません。そこで `rust (cross)` のジョブが、ubuntu のまま
`--target` を変えて `cargo clippy --all-targets -- -D warnings` を流します (T0829)。リンクはしないので、ターゲットの標準ライブラリ
(`rustup target add`) だけがあればよく、Windows や macOS の SDK は要りません。テストは動かせません。

- 定義の正は `scripts/cross-check.mjs` の `CHECKS` (検査の名前・クレート・ターゲット・feature)。CI のジョブは `CHECKS` が要るターゲットを
  `node scripts/cross-check.mjs targets` で足し、すべての検査を流します。
- 手元では `just cross-check` (すべて) か `just cross-check <名前>`。仮想カメラは `just vcam-check`
  (`ugoe-studio-vcam` を `x86_64-pc-windows-msvc` と `aarch64-apple-darwin` で)。ターゲットが入っていなければ `rustup target add …` の
  入れ方を出して止まります。`just ci` には含めないので、cfg で OS を分けたコードを触ったら実行してください。
- 検査を足すときは `CHECKS` に書くだけでジョブに入ります。定義のクレートがワークスペースにあるかは `just groups` が検査します。
- feature は `just lint` の 2 回目と同じく、すべて (`allFeatures: true` で `--all-features`。選んだクレートの feature だけが有効になる) を
  使います。クレートに feature を足しても書き足す必要はありません。一部の feature だけにしたいときは `features` に `クレート/feature` を
  並べます (両方は書けない。T0033)。
- 依存のビルドスクリプトがターゲット向けに C やリソースをコンパイルするときは、そのツールが要ります。`CHECKS` の `commands` に
  コマンドを、`apt` に apt のパッケージを書くと、`just cross-check` は無いときに入れ方を出して止まり、CI のジョブは
  `node scripts/cross-check.mjs apt` のパッケージを入れます。

| 検査 | クレート | ターゲット | 要るもの |
| --- | --- | --- | --- |
| `vcam` | `ugoe-studio-vcam` | `x86_64-pc-windows-msvc`・`aarch64-apple-darwin` | ターゲットの標準ライブラリだけ |
| `gpu-priority` | `ugoe-studio-gpu-priority` (スタジオの GPU の優先度。ADR 0034) | `x86_64-pc-windows-msvc` | ターゲットの標準ライブラリだけ |
| `apps` | `ugoe-editor`・`ugoe-viewer`・`ugoe-studio` | `x86_64-pc-windows-gnu` | MinGW の gcc と windres (`gcc-mingw-w64-x86-64`) |
| `tools` | `ugoe-ai` と `OS_CRATES` (`ugoe-format`・`ugoe-fs`・`ugoe-editor-core`・`ugoe-cli`・`ugoe-mcp`) | `x86_64-pc-windows-gnu` | MinGW の gcc (`gcc-mingw-w64-x86-64`。依存の ring の C) |

アプリ (`apps`) は Tauri と `ugoe-ai` の依存の C (ring) と、tauri-build のリソース (`.rc`) をターゲット向けにコンパイルするので、
ターゲットの標準ライブラリだけでは検査できません (T0013)。`tools` (T0027) も ring の C をコンパイルするので、同じく `windows-gnu` で検査します。

- `tools` のクレートは、`ugoe-ai` (`rust (windows)` に入らない) と、`rust (windows)` の Clippy (`just lint-os`) が見る `scripts/ci-groups.mjs` の
  `OS_CRATES` です。一覧は `OS_CRATES` から読むので、`OS_CRATES` にクレートを足せばここにも入ります。`rust (windows)` を待たずに、
  unix だけのテスト (`#[cfg(unix)]`) が使う補助が Windows で未使用になる誤りなどを手元で見つけるためです (T0962。T0959 の誤りは
  `just cross-check tools` で見つかります)。macOS のターゲットは ring の C に Apple の SDK が要り ubuntu では検査できないので、
  `rust (macos)` のジョブ (`just lint-os`) に任せます。

- **Windows:** msvc のターゲットは MSVC の SDK (cargo-xwin などで Microsoft の SDK を落とす) が要るので、MinGW で済む `windows-gnu` で
  検査します。アプリの Windows のコードは `cfg(windows)`・`target_os = "windows"` で分けていて、msvc と gnu の違い (`target_env`) に
  依存しないので、型検査と lint の範囲は変わりません。
- **macOS:** C のコンパイルに Apple の SDK (`TargetConditionals.h` など) が要り、ubuntu には入れられないので、`rust (macos)` のジョブが
  macOS の上で `just lint-group apps` を流します (下の「Windows と macOS」)。

### Windows と macOS

ファイルを書き出す・パスを扱うクレートの誤り (大文字小文字だけが違う名前を同じファイルとして扱う、パスの区切り、ファイルの権限など。T0667) は
ubuntu では見つからないので、`release-checks.yml` の `rust (windows)` (`windows-latest`) と `rust (macos)` (`macos-latest`) のジョブが
次を流します (T0013。T0091 の前は毎回の PR と main への push で流していた)。

- 対象のクレートの定義の正は `scripts/ci-groups.mjs` の `OS_CRATES`: `ugoe-format`・`ugoe-fs`・`ugoe-editor-core`・`ugoe-cli`・`ugoe-mcp`。
  ワークスペースのクレートかは `just groups` が検査します。
- `just lint-os`: 対象のクレートの clippy (すべての feature)。`just test-os`: 対象のクレートのテスト (`gpu-tests` の下のテストは動かない)。
  どちらも Node から cargo を呼ぶので、Windows の PowerShell からも動きます。手元の Linux でも流せます。
- macOS では、アプリの src-tauri の macOS のコード (エディタの資格情報の保存先など) と仮想カメラを、macOS の上の
  `just lint-group apps` で検査します (上の「別のターゲットの検査」)。
- C ABI (`just ffi-check`) も流し、C の見本を `ugoe-ffi` の静的ライブラリにリンクして動かします
  ([crates/ugoe-ffi/README.md](../crates/ugoe-ffi/README.md#std-の無い環境))。macOS は `std` がある形と `std` の無い形の両方で、macOS の
  C ライブラリ (`std` の無い形が呼ぶ `aligned_alloc` は macOS 10.15 以降) とリンクのフラグを見ます (T0067)。Windows は `std` がある形だけで、
  `scripts/ffi.mjs` はランナーに入っている `clang` (gcc 形式の引数のドライバ) で MSVC のライブラリとリンクします (T0069)。`cl` を使わないのは、
  `cl` は開発者のシェル (vcvars) の外では使えないが、`clang` は Visual Studio と Windows SDK を自分で探すので手順を足さずに済み、
  警告の扱い (`-Werror -pedantic`) も Linux・macOS と同じ引数で書けるためです。リンクするシステムのライブラリは
  `cargo rustc -p ugoe-ffi --lib --crate-type staticlib -- --print native-static-libs` の出力に合わせます。C ライブラリは Rust の側と同じ
  動的な msvcrt に揃えます。`-fms-runtime-lib=dll` はコンパイルにしか効かず、clang のドライバがリンクの行に足す静的な libcmt を
  `-Wl,-nodefaultlib:libcmt` で外します (T0079。lld-link は両方があっても通るが、MSVC の link.exe は静的な C ライブラリを選んで落ちるので、
  手元で cargo-xwin と lld-link で確かめても見えない)。`std` の無い形は MSVC の
  C ライブラリに `aligned_alloc` が無いので飛ばします。
- ファイルを扱うクレートを足したら `OS_CRATES` に足します (`just cross-check` の `tools` にも入る。上の「別のターゲットの検査」)。GPU も Tauri のシステムの依存も入れないので、それらが要るクレートは入れません。
- 手元の Linux で Windows のテストを近似するには、MinGW と Wine で `x86_64-pc-windows-gnu` のテストを流せます (CI の代わりにはならない。
  Wine のソケットやファイルシステムは本物の Windows と細部が違う):

  ```sh
  rustup target add x86_64-pc-windows-gnu   # MinGW (gcc-mingw-w64-x86-64) と wine も要る
  WINEPREFIX=/tmp/ugoe-wine WINEDLLOVERRIDES="mscoree,mshtml=" WINEDEBUG=-all DISPLAY= \
    CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUNNER=wine \
    cargo test -p ugoe-format -p ugoe-fs -p ugoe-editor-core -p ugoe-cli -p ugoe-mcp --target x86_64-pc-windows-gnu
  ```

  `WINEDLLOVERRIDES` は、新しい Wine の環境を作るときの Mono と Gecko の導入 (画面が無いと止まる) を飛ばすためです。

### std の無い形の単体テスト

`just nostd-check` (CI の `rust (runtime)`) は、`scripts/nostd-check.mjs` の `HOST_TEST_CRATES` (`ugoe-format`・`ugoe-ffi`) の単体テストを、ホストで既定の feature を外して (`cargo test -p <クレート> --lib --no-default-features`) 流し、テストのコードも同じ feature で clippy にかけます (`--lib --profile test`)。`std` があると選ばれない分岐 (`ugoe-format` の `ruzstd` の `std` 無しの zstd の展開、`png` が無いときの KTX2 の扱いなど) を確かめるためです ([portability.md](portability.md#ランタイム層が守る制約-移植性のため)。T0186・T0491)。

- 対象のクレートは開発時の依存に自分を置きません。置くと、その feature (`ugoe-format` なら `png` と `std`) が `--no-default-features` のテストでも有効になります。`just groups` (`nostd-check.mjs check`) が検査します。
- feature の要るテストは、単体テストなら `#[cfg(feature = "…")]`、結合テストなら `Cargo.toml` の `[[test]]` の `required-features` にし、その feature を CI のグループの `features` に書きます (上の CI のジョブ)。
- 有効になる feature は `cargo tree -p <クレート> --no-default-features -e features -i <クレート>` で確かめられます (クレート自身の feature が出なければよい)。

### tier 3 のターゲットの build-std

ゲーム機向けのターゲットの多くは Rust の tier 3 で、`core`・`alloc` が配られません。`just nostd-build-std` は、`scripts/nostd-check.mjs` の
`BUILD_STD_TARGETS` (`aarch64-nintendo-switch-freestanding` と、64 ビットのアトミック操作の無い 32 ビットの `armv6k-nintendo-3ds`) で、
nightly の `-Zbuild-std=core,alloc -Zbuild-std-features=compiler-builtins-mem` で `ugoe-ffi` の静的ライブラリを作り、
未解決のシンボルが `just nostd-check` と同じ 6 つ (`HOST_SYMBOLS`: `abort`・`aligned_alloc`・`calloc`・`free`・`malloc`・`realloc`) と同じかを
`llvm-nm` で確かめます (手順は [crates/ugoe-ffi/README.md](../crates/ugoe-ffi/README.md#core-と-alloc-が配られないターゲット-tier-3t0495)。T0495・T0071)。

- nightly が要るので `just ci`・push・PR ごとの CI には入れず、`.github/workflows/nostd-build-std.yml` を `release-checks.yml` が呼びます
  (単独でも手で起動できる)。nightly や依存の変更で壊れても PR は止まりません。fuzz.yml とは失敗の意味と直し方が違うので、別のワークフローにしています。
- 道具は `rustup toolchain install nightly --profile minimal --component rust-src,llvm-tools` で入れます (無ければ入れ方を出して止まります)。
  `llvm-nm` は nightly の llvm-tools のもの (rustc と同じ LLVM) を使い、`LLVM_NM` で変えられます。nightly は `UGOE_NIGHTLY_TOOLCHAIN`
  (既定 `nightly`) で日付を固定できます。nightly はルートの `Cargo.toml` の `rust-version` 以上であることが要ります。
- 未解決のシンボルは、静的ライブラリのどのメンバも定義しないものです (リンクの `--gc-sections` と違い、使われない関数の参照も残ります)。
  一覧に無いもの (例: `compiler-builtins-mem` が効かなくなって `memcpy` などが残る) も、一覧にあって無いものも失敗にします。
  C の見本とのリンクはしません (ターゲットごとに C の浮動小数点の ABI を合わせる必要があるため。リンクは stable の `aarch64-unknown-none` で
  `just nostd-check` が確かめます)。
- 成果物は `target/build-std/` に置きます (stable の `just nostd-check`・`just ffi-check` のプロファイル `nostd` と rustc の版が違い、
  作り直し合わないように)。mbx (ビルドのキャッシュ) は通しません。
- 1 つのターゲットが失敗しても残りのターゲットを続け、最後に失敗します。手元で 2 つのターゲットにかかる時間は約 1 分です。

### クレートを足したとき

1. `scripts/check-layers.mjs` の `ALLOWED` に登録する (依存方向)。
2. `scripts/ci-groups.mjs` の `GROUPS` の、近いグループの `crates` に足す。どのグループにも入っていないと `just groups` (`just ci` と CI の `rust-checks`) が失敗する。
3. ほかのクレートが有効にする feature の裏にテストがあるなら、そのグループの `features` に足す。
4. 1 つのグループの target が大きくなりすぎたら (目安は 5GB)、グループを分ける。`GROUPS` に足すだけで CI の matrix に入る。グループに固有の手順 (生成物の検査など) があれば、`ci.yml` の `rust-group` に `if: matrix.group == '<名前>'` のステップを足す。Tauri のシステムの依存が要るグループには `tauri: true` を書く。

## フィクスチャ

- ランタイムのテストの入力には `ugoe_model::fixtures` のモデルを使います。バイナリファイルを置くより、コードで組み立てたモデルのほうが差分を読みやすく、スキーマの変更にもコンパイラが追従させてくれるためです。
- 新しい機能には新しいフィクスチャ関数を追加します。**既存のフィクスチャの挙動は変えない** でください。他のエージェントのテストが壊れます。
- 画像や PSD のように、どうしてもファイルが必要な場合は `assets/fixtures/` に置きます。1 ファイル 1MB 以下にし、出所とライセンスを `assets/fixtures/README.md` に書きます。

## 一時ディレクトリ

ファイルを読み書きするテストは、[`tempfile`](https://docs.rs/tempfile) (dev-dependency) の一時ディレクトリを使います。

```rust
let tmp = tempfile::tempdir().unwrap();
let dir = tmp.path(); // 作ってある空のディレクトリ
let project = dir.join("a.ugoeproj"); // まだ無いパスが要るときは、中の名前を join する
```

- 名前は毎回違うので、並行のテスト (`just ci` を複数の worktree で流すときも) と重ならず、`TempDir` を落とすと (失敗して panic しても) 消えます。
  `std::env::temp_dir()` とプロセス ID で名前を作る補助を手で書かないでください (名前が重なり、失敗すると残る)。
- `TempDir` はテストの終わりまで変数に持っておきます (`tempfile::tempdir().unwrap().path()` と書くと、すぐに消えます)。
  正規化したパス (`canonicalize`) が要るなど、ファイルの中で何度も使う形があれば、`tempfile::TempDir` を持つ小さな構造体をそのファイルに書きます
  (例: `crates/ugoe-mcp/tests/editor.rs` の `TempDir`)。
- クレートで使うには、`[dev-dependencies]` に `tempfile.workspace = true` を足します。ランタイム層のクレートの通常の依存には入れません。
- 目で確かめるために絵を残す描画テスト (`crates/ugoe-cli/tests/*render_gpu.rs`) は、決まった名前のディレクトリ (一時ディレクトリか `CARGO_TARGET_TMPDIR` の下) に書いて残します (例外)。

## 互換性テスト

ADR 0013 の「新しいファイルを古いランタイムで劣化させて読める」「古いファイルを新しいランタイムで読める」ことを、
ファイルの資料 (`assets/compat/`) で検査します。テストは `crates/ugoe-format/tests/compat.rs` で、`just test` (`just ci`・CI の `rust (runtime)` ジョブ) に含まれます。
仕様は [spec/file-formats.md#機能の宣言と段階的な劣化](spec/file-formats.md#機能の宣言と段階的な劣化) です。

### 資料の置き場所と名前

| ディレクトリ | 中身 | テスト |
| --- | --- | --- |
| `assets/compat/dev/` | 今のコードの書き出し (`write_bundle` / `write_module`) で作ったファイル。1.0 の基準の機能をすべて含むように、フィクスチャごとに 1 ファイル | 今のコードで作り直したものとバイト単位で一致する (`corpus_is_up_to_date`)。今のランタイムで警告なしに読め、評価できる |
| `assets/compat/<版>/` (例 `1.0.0/`) | リリースの時点の `dev/` の写し。**作り直さない、書き換えない** | 今のランタイムで警告なしに読め、評価できる (`every_release_loads_without_warnings`) |
| `assets/compat/future/` | 新しい版で書き出したファイルを模したもの (下の表) | 劣化させて読める / 拒否される |

- ファイル名は中身を表す kebab-case で、拡張子はそのまま (`dynamic-hair.ugoe`・`long-hair.ugoemod`)。版のディレクトリの間で、同じ中身には同じ名前を使います。
- 各版のディレクトリには、モジュールを取り付ける本体 `dress-up-base.ugoe` と `posed-dress-up-base.ugoe` を必ず置きます (`.ugoemod` はどちらかに `compose` して評価する)。
  `posed-dress-up-base.ugoe` (`fixtures::posed_dress_up_base`) は `dress-up-base.ugoe` に腕の差分 `ArmA` / `ArmB` のポーズのグループを加え、スロット `Outfit` の `pose_parts` に出したもので、本体の要素に追従するモジュール (`sleeves.ugoemod`) を取り付けます。ほかのモジュールは `dress-up-base.ugoe` に取り付けます (`compat.rs` の `module_base`。`tools/spec-check` も同じ)。
- `dev/` のモジュールは、物理を持つ `long-hair.ugoemod`・圧縮した `dress.ugoemod`・ポーズのグループを持つ `jacket.ugoemod`・本体のポーズの要素に追従する (`ModulePose` の `links`) `sleeves.ugoemod` です。
  - `jacket.ugoemod` は、読んだポーズのグループ (要素とパーツ) が書き出す前と同じで、本体に取り付けるとグループが本体のグループの後ろに加わることも確かめます (`module_pose_groups_survive_and_compose`)。
  - `sleeves.ugoemod` は、読んだ追従が書き出す前と同じで、ポーズを出さない `dress-up-base.ugoe` には取り付けられず (`PoseNotExposed`)、`posed-dress-up-base.ugoe` に取り付けると袖 `SleeveA` / `SleeveB` が本体の要素 `ArmA` / `ArmB` の `links` に加わり、要素を切り替えると (フェードの途中も) 袖の不透明度が腕と同じになることを確かめます (`module_pose_links_follow_base_entries`)。
- 1 ファイル数 KB に収めます。画像は 1×1 の PNG です。

`future/` の資料:

| ファイル | 中身 | 期待する結果 |
| --- | --- | --- |
| `optional-feature.ugoe` | `dev/dress-up-base.ugoe` に、知らない任意の機能の宣言 (`test.future_optional`)・知らない enum の値 (合成モード・マスクの結合・デフォーマの種類・パラメータの種類・カーブの対象とセグメント・表情の演算)・知らないフィールド・知らないチャンク (`XFUT`) を加えたもの | 警告付きで読める。未知の値には仕様のフォールバックを適用し、評価の結果 (頂点・合成モードなど) は `dev/` の元のファイルと同じになる。書き出しは `UnknownValues` で拒否する |
| `optional-feature.ugoemod` | `dev/long-hair.ugoemod` に同様の変更 (物理の入力の種類を含む) と、頂点の持ち主 (`VertexOwner`) が知らない種類の物理の出力 (中身のある `{"bone": …}` と中身の無い `"lattice"`) を加えたもの | 警告付きで読め、本体に取り付けた評価の結果が元のモジュールと同じになる。未知の持ち主への出力は合成で残り、物理はそれを無視する (未知の出力を除いた設定で物理を進めた結果と同じになる) |
| `required-feature.ugoe` / `.ugoemod` | 知らない必須の機能 (`test.future_required`) を宣言し、本体のチャンクが今の版では読めない形のもの | `UnsupportedFeatures(["test.future_required"])` で拒否する (デコードより先に機能の宣言を調べる) |
| `newer-schema.ugoe` / `.ugoemod` | `schema_version` を今の版より 1 つ上げ、本体のチャンクを今の版では読めない形にしたもの (`schema_version` は先頭のフィールドでない) | `UnsupportedVersion` で拒否する (デコードより先に `schema_version` だけを読む) |

- `test.` で始まる機能 ID は試験のために予約されていて、本物の機能には使いません。
- `future/` のコンテナはテストの中で仕様だけから組み立て、`ugoe-format` の書き出しのコードを使いません (書き出しの誤りを読み込みのテストに持ち込まないため)。

### 作り直す

形式・フィクスチャ・書き出しを変えて `corpus_is_up_to_date` が失敗したら、資料を作り直してコミットします。

```sh
UGOE_UPDATE_FIXTURES=1 cargo test -p ugoe-format --test compat
cargo test -p ugoe-format --test compat   # 作り直した資料で、もう一度通す
```

- 作り直すのは `dev/` と `future/` だけです。余分なファイルは消します。版のディレクトリには触れません。
- 作り直している間は、ほかのテストが書きかけのファイルを読んで失敗することがあります。2 行目でもう一度実行して確かめます。
- 1.0 より前は形式を壊してよいので (ADR 0004)、`dev/` の差分はそのままコミットしてよいものとします。1.0 以降に `dev/` の既存のファイルの読み込みが変わるなら、それは互換性を壊す変更です (`SCHEMA_VERSION` を上げるか、機能の宣言で表す)。

### 資料を足す

- **リリースのとき**: `dev/` を `assets/compat/<版>/` に写してコミットします (`cp -r assets/compat/dev assets/compat/1.0.0`)。
  1.0 より前はリリースがないので、版のディレクトリはまだありません。1.0 より前の形式は互換性を保証しないため、作りません。
- **基準の機能やフィクスチャを足したとき**: `compat.rs` の `dev_corpus` にファイルを足して作り直します。
- **1.0 以降の機能を足したとき**: その機能を使うファイルを `dev/` に足します (`FEAT` に宣言が入る)。
  その機能を知らない古いランタイムは、ここでは実行できません。古いランタイムは知らない機能を ID によらず同じに扱い、
  知らない値を `Unknown` として読むので、`future/` の資料で「今のランタイムが知らないもの」の扱いを試しておくことが保証になります。
  これまで試していない場所 (新しい enum やチャンク) を拡張するときは、先に `future/` の変更に加え、そのフォールバックをテストします。

## 公開仕様の例

公開仕様の例 (`spec/examples/`、[spec/examples/README.md](../spec/examples/README.md)) は、小さなモデル (`model.json`) と、
パラメータの値ごとの頂点の位置・不透明度・描画順などの期待値 (`expected.json`) の組です。
ランタイムが期待値を満たすことを `crates/ugoe-format/tests/spec_examples.rs` で確かめます (`just test` に含まれる)。

- `model.json` は `ugoe_model::spec_files::example_models` のフィクスチャから `just gen` で作ります (手で編集しない)。
  最新かどうかは `ugoe-model` の `tests/spec_files.rs` と CI の `just gen && git diff --exit-code` が確かめます。
- `expected.json` は仕様の式から手で導いた値です (多くはフィクスチャの doc コメントとランタイムのテストの値)。
  **テストが落ちたときに、ランタイムの出力を写して期待値を合わせてはいけません。** 仕様とランタイムのどちらが誤っているかを調べます。
- 許容誤差は位置 0.01 px、描画順の値 0.001、不透明度・色・マスクの効き具合 0.0001 で、描画順の値の差が 0.001 以下のメッシュの間では描画の順番を問わない (公開仕様の [Tolerances](../spec/examples/README.md#tolerances)。評価の適合の基準と同じ)。
- 例を足すには、`example_models` にフィクスチャを足して `just gen` を実行し、`expected.json` を書きます。`spec/examples/README.md` の表にも行を足します。
- ファイルの例 (`spec/examples/bundles/`、公開仕様の [File examples](../spec/examples/README.md#file-examples)) は、小さな `.ugoe` と `.ugoemod` のファイルと、
  その一覧 `bundles.json` です。一覧の各項目は、チャンク表 (タグと圧縮の有無の並び) と、読めるファイルなら読んだ中身
  (対応する JSON の文書・`META` の値・画像の全画素・報告される知らない任意の機能)、拒否されるファイルなら拒否する読み込みの手順の名前
  (`feature_declarations` など) と知らない必須の機能を持ちます。ファイルは仕様だけから作った別の書き出し (`tools/spec-check`) で作ります。
  `png-*.ugoe` は、PNG のテクスチャの読み方 (rendering.md#前提) を確かめる例です。テクスチャの PNG の中身 (色の種類・ビット深度・`PLTE`・`tRNS`・
  `gAMA`・`iCCP` などのチャンク・インターレース・APNG のフレーム・サンプルの値) を一覧の `png` に書き、規則どおりに読んだ画素を `rgba` に持ちます。
  参照実装の読み込み (`read_bundle_with_report` / `read_module_with_report`) が一覧のとおりに読むことを
  `crates/ugoe-format/tests/spec_bundles.rs` で確かめます。食い違う例は理由を付けてテストの `KNOWN_MISMATCHES` に挙げます
  (例のファイルを参照実装に合わせて作り直してはいけません)。
- 描画の例 (`spec/examples/render-*/`、公開仕様の [Rendering examples](../spec/examples/README.md#rendering-examples)) は、`model.json` と、
  テクスチャの画素・描画先の大きさ・ビュー変換 (省略するとキャンバスを描画先に引き伸ばす)・光源・調べる画素の期待値
  (乗算済み、0〜255 の尺度) を持つ `render.json` の組です。
  調べる画素の中心は三角形の辺から描画先の画素で 1 以上離し、テクスチャは 1 色を基本にして、GPU によるラスタライズとフィルタの違いに
  頼らないようにしています (`render-texture` はフィルタを、`render-png` はテクセルの中心で PNG の読み方と乗算済みへの変換を確かめるので例外)。
  テクスチャは `png` に PNG の中身を書けます (ファイルの例と同じ記述。無ければ 8 ビットの RGBA)。許容誤差は各チャンネル 3 (rendering.md の Conformance と同じ) で、調べる画素はすべて満たします。
  期待値は仕様の式から画素ごとに導いた値で、`tools/spec-check` の CPU のラスタライザ (仕様だけから書いたもの) と一致します。
  使える光源が 8 個を超えるケース (レンダラは 8 個だけを使ってよいので互換の基準の外) は、どの 8 個を使っても、すべてを使っても
  調べる画素が同じになるように選んでいます。
  参照実装の描画が満たすことは GPU を使うテスト (`just test-gpu`、`crates/ugoe-render/tests/spec_render.rs`) で確かめます。
  食い違う画素は理由を付けてテストの `KNOWN_MISMATCHES` に挙げます。
- 時間の例 (`timeline.json` を持つもの、公開仕様の [Timeline examples](../spec/examples/README.md#timeline-examples)) は、フレームごとの
  ホストの操作と `dt` と、そのフレームの後のパラメータの値・パーツの不透明度の上書き・イベント・動的物理の加算量・評価の結果の期待値の列です。
  操作はパラメータの設定、モーションの開始と停止、表情の設定・追加・停止、リップシンクの音量、トラッキングの入力 (`null` で外す)、
  ポーズの要素の表示 (`show_pose`・フェードを飛ばす `show_pose_immediately`)、エフェクト設定の差し替え (`set_effects`)、物理のリセット、段の有効無効です。
  参照実装の `Puppet` が満たすことを `crates/ugoe/tests/spec_timelines.rs` で確かめ、食い違う例は理由を付けて `KNOWN_MISMATCHES` に挙げます。
- 互換性テストの資料 (`assets/compat/`) との違い: あちらはリリースごとのバイナリのファイルで、古い版のファイルが読めることと、
  知らない機能を劣化させて読めることを確かめる (評価の期待値は持たない)。こちらは仕様の一部 (CC BY 4.0) で、
  評価の規則とファイルの中身の並べ方を例で示し、他の実装が自分を確かめるためのもの。

## ランタイムの数値テスト

- 変形の結果は、許容誤差を付けて比較します (目安は `1e-4` px)。
- 仕様書の式に対応するテストには、テスト名かコメントで仕様書の節を示します。
- 評価の出力全体を `insta` の JSON スナップショットにしてよいですが、スナップショットを更新するときは、PR 本文に「なぜ値が変わったのか」を書きます。

## 壊れた入力と fuzz

外から来る入力 (`.ugoe`・`.ugoemod`・画像・JSON・ホストの数) でパニックしないことは、2 段で確かめます
(規則は [portability.md](portability.md#パニックしない))。

- **決まった入力のテスト** (`crates/ugoe-format/tests/broken_input.rs`・`crates/ugoe/tests/broken_input.rs`。T0184): 切り詰め・
  ビットの反転・数の書き換え・深い入れ子など。stable で依存を足さずに動き、`just ci` に入ります。
- **fuzz** (`fuzz/`。下の節): 決まった入力の外を、カバレッジを見て入力を変えながら探します。`just ci` には入れません。

### fuzz

[cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) (libFuzzer) のターゲットを、ルートのワークスペースの外のクレート `fuzz/` に置きます
(決めたことと理由は [ADR 0032](adr/0032-fuzzing-with-cargo-fuzz.md)。T0188 で利用者が選んだ)。

- ターゲットは `bundle` (バンドルの読み込み・画像のデコード・`Puppet` での評価)・`module` (モジュールの読み込みと着せ替えの本体への
  取り付け)・`image` (PNG と KTX2 のデコード)・`json` (JSON 形式のモデル・モジュール・物理・モーション・表情)・`ffi` (C ABI の入口。下)。
  詳しくは [fuzz/README.md](../fuzz/README.md)。
- **C ABI (`ffi`。T0497):** C ABI の関数は `unsafe extern "C" fn` で、呼ぶには `unsafe` が要る。利用者の決定 (2026-10-07) で、
  `ugoe-ffi` に feature `fuzzing` で安全な Rust の入口 `ugoe_ffi::fuzzing::run` を置き、ターゲットはそれを呼ぶ (`fuzz/` は
  `unsafe_code = "deny"` のまま。`unsafe` の例外を増やさず、`unsafe` は `ugoe-ffi` の中に留まる)。入口は入力を
  `[ファイル] [SCRIPT_MARKER] [台本]` に分け、ファイルをバンドル (読めなければモデルの JSON) とモジュールとして C のホストと同じ呼び方で
  読み、台本の 1 バイトごとに操作 (パラメータの番号と名前での設定・NaN と無限大・`update`・モーション・表情・トラッキング・受け手・
  当たり判定・物理とエフェクトの JSON・着せ替え・フレームの借用と写しなど) を選んで呼ぶ。借りた文字列と配列はすべて読み
  (fuzz のビルドは AddressSanitizer)、範囲の外の番号を返すなどのホストへの約束の破れも誤りにする。feature はヘッダに出さず、
  配るライブラリでは有効にしない (`panic-test` と同じ扱い)。
  - **std がある形のパニック:** C ABI は `catch_unwind` でパニックを `UGOE_ERROR_PANIC` に変えるが、libfuzzer-sys は初期化のときに
    「既定のフックを呼んでから `abort` する」パニックのフックを入れるので、巻き戻す前 (`catch_unwind` に届く前) に止まり、fuzz の
    クラッシュとして見つかる (T0497 で、わざとパニックさせて確かめた)。フックを通らずに `UGOE_ERROR_PANIC` が返ったときも、入口が
    誤りとして返す。
  - 同じ入口を、決まった入力 (`assets/compat` と `assets/samples` のファイルに決まった台本を付けたもの) で `crates/ugoe-ffi/tests/fuzzing.rs`
    (`required-features = ["fuzzing"]`。CI のグループ `bindings` が feature を有効にする) が stable で回す。`ffi` で見つかった入力はここに足す。
  - C ABI に関数を足して入口に入れ忘れると、`just refs` (`scripts/check-ffi-bindings.mjs`) がヘッダと入口の呼び出し (関数のポインタとして
    渡すのも含む) を比べ、呼んでいない関数の名前を出して止める。呼ばなくてよい関数 (`ugoe_abi_version` など入力を取らないもの) は
    スクリプトの `omit` に理由付きで置く (T0498)。どの台本で届くかまでは見ない。
- 手元では `just fuzz <target> [秒]` (既定 60 秒。`all` ですべて)。nightly と cargo-fuzz が要り、無ければ入れ方を示して止まります。
  nightly は fuzz のビルドだけで使い、`rust-toolchain.toml` (stable) は変えません。
- **ビルドのターゲット (T0087):** `fuzz/run.mjs` は、nightly の rustc のホスト (`rustc +nightly -vV` の `host:`) を `cargo fuzz build`・
  `cargo fuzz run` に `--target` で明示します。cargo-fuzz は `--target` が無いと、cargo-fuzz 自身をビルドしたときのターゲット
  (crate `current_platform` が埋め込む値) を使うためです。`cargo install cargo-fuzz` でソースから入れれば手元のホストと同じですが、
  配布されているビルド済みの cargo-fuzz (0.13.2 の時点で Linux は `x86_64-unknown-linux-musl`、macOS は `x86_64-apple-darwin` だけ) を
  使うとホストと食い違います。CI (taiki-e/install-action が cargo-binstall で入れる) は musl の版を使い、明示する前は 5 つのターゲットが
  すべて「sanitizer is incompatible with statically linked libc」でビルドの段で落ちました
  (出典: [cargo-fuzz の `default_target`](https://github.com/rust-fuzz/cargo-fuzz/blob/0.13.2/src/utils.rs) と
  [リリースの成果物](https://github.com/rust-fuzz/cargo-fuzz/releases/tag/0.13.2))。手元のホストが gnu・apple・msvc のどれでも、
  ソースから入れた cargo-fuzz なら明示しても振る舞いは変わりません。
- CI では `.github/workflows/fuzz.yml` (`release-checks.yml` が呼ぶ。単独でも手で起動できる) がターゲットごとに 5 分回し、育てた入力 (`fuzz/corpus/`) をキャッシュで
  次の回へ持ち越します。見つかった入力はワークフローの成果物 (`fuzz-artifacts-<target>`) に上げます。
- push・PR ごとの CI (ci.yml の `rust (runtime)`) は、`just fuzz-lint` で fuzz のクレートを stable で検査します (依存のライセンス・整形・
  clippy。T0072)。ターゲットは `ugoe`・`ugoe-format` の公開 API と `ugoe-ffi` の `fuzzing` を使うので、変えて壊れたら `release-checks.yml` を待たずに PR で気づきます。
  nightly と cargo-fuzz は要りませんが、`libfuzzer-sys` のビルドスクリプトが同梱の libFuzzer (C++) をコンパイルするので C++ の
  コンパイラが要ります。ランタイムの公開 API を変えたら手元でも `just fuzz-lint` を流してください (`just ci` には入れていません)。
- 依存のライセンスの検査 (`cargo metadata`) と clippy は `--locked` で流すので、ランタイムのクレートに依存を足して `fuzz/Cargo.lock` が
  古くなると `just fuzz-lint` が失敗します。ライセンスの検査が先に落ち、古い lock のファイルと直し方だけを 2 行で出します
  (`scripts/lib/cargo-locked.mjs`。T0073)。
  `cargo update --manifest-path fuzz/Cargo.toml --workspace` で足りない依存だけを足して直し、`fuzz/Cargo.lock` をコミットします
  ([fuzz/README.md](../fuzz/README.md))。Godot のバインディング (`bindings/godot/Cargo.lock`。`node bindings/godot/build.mjs --check`)
  も同じで、`cargo update --manifest-path bindings/godot/Cargo.toml --workspace` で直します ([bindings/godot/README.md](../bindings/godot/README.md))。
- **依存の版はルートの `Cargo.lock` とそろえます (T0074)。** `fuzz/Cargo.lock` と `bindings/godot/Cargo.lock` は、ルートの `Cargo.lock`
  にもある依存 (semver で互換な同じ系列) をルートと同じ版にします。理由: fuzz が回すコードや Godot に出すコードと、出荷するコード
  (ルートのワークスペースでビルドするアプリ・C ABI・wasm) で依存の版が違うと、fuzz が見つけた (あるいは見つけない) 不具合が
  出荷物と食い違うおそれがあるためです (着手のときは glam・libc・cc・twox-hash がずれていた)。外のクレートだけの依存
  (`libfuzzer-sys`・godot-rust) と、ルートと互換でない系列 (godot-rust の glam 0.32) は比べません。
  - `just groups` (`just ci` と CI の `rust-checks`) と `just fuzz-lint` が `scripts/check-lock-sync.mjs` (判定は `scripts/lib/lock-sync.mjs`)
    で比べ、ずれた依存と、ルートの版に合わせる `cargo update --manifest-path <…>/Cargo.toml -p <名前>@<版> --precise <ルートの版>` を出して
    止まります。cargo は使わず lock のファイルを読むだけなので、ワークスペースをビルドしないジョブで流せます。
  - ずれは、足した依存を `cargo update --workspace` が最新の版で入れたときと、ルートで依存を上げたとき (外の lock は自動では追従しない)
    に起きます。外のクレートの依存が新しい版を求めてルートの版に合わせられないときは、ルートで `cargo update -p <名前>` して両方を新しい版にします。
- 計装したターゲットのビルド (nightly と cargo-fuzz) まで含めた検査は `just fuzz-check` で、CI では `release-checks.yml` が呼ぶ fuzz.yml が流します。
- 見つかった入力を直したら、最小にした入力 (`cargo +nightly fuzz tmin`) を `broken_input.rs` の決まった入力に足し、stable の
  `just ci` で再発を止めます (例: `crates/ugoe-format/tests/broken_input.rs` の `found_by_fuzzing`。`ffi` なら
  `crates/ugoe-ffi/tests/fuzzing.rs`)。
- 依存 (`libfuzzer-sys` と同梱の libFuzzer) のライセンスは `just licenses` の対象の外なので、`scripts/check-licenses.mjs --fuzz`
  (`just fuzz-check`) で検査します。

## 性能の計測

性能の目標 ([roadmap.md](roadmap.md) の「1 体を 1ms 以内」) に響く処理は、各クレートの `examples/` に計測用のプログラムを置きます。
時間は機械と負荷で大きく変わるので `just ci` には入れず、手元で `--release` を付けて動かし、改善の前後をタスクファイルのメモに書きます。

| 計測 | コマンド | 内容 |
| --- | --- | --- |
| 着せ替えの合成とコンパイル | `cargo run --release -p ugoe-runtime --example compose_timing` | 中規模の本体にモジュール 4 つを取り付ける ([modules.md](spec/modules.md#性能)) |
| モーションのカーブの評価 | `cargo run --release -p ugoe-motion --example curve_timing` | フレームごとにキーを打った長いカーブ (300 セグメント × 100 本) を、線形の探索と前処理した探索 (`SegmentIndex`) で比べる |
| `Puppet` の 1 フレームの評価 | `cargo run --release -p ugoe --example puppet_timing` | 中程度のモデルを模した生成のモデルと見本の `avatar.ugoe` で、`Puppet::update` と受け手の `Puppet::follow` の 1 回を測る。同じ 2 つのモデルの物理の段の内訳も出す (下の「1 フレームの評価」) |
| `WebPuppet` の 1 フレームの評価 (wasm) | `just web-runtime` と `cargo run --release -p ugoe --example puppet_timing -- --write bindings/web/examples/timing/generated.ugoe` の後に `node bindings/web/examples/timing/measure.mjs --runs 5` (`--max-us 300` で上限と比べる。ビルドから上限との比較までは `just perf-wasm`) | 同じ 2 つのモデルで、ブラウザの中の `WebPuppet.update`・`follow` の 1 回を測る (計測のページ `bindings/web/examples/timing/`。ヘッドレスの Chrome で開いて表を出す。手元のブラウザで開くなら `node bindings/web/examples/timing/serve.mjs`。下の「1 フレームの評価」) |
| 物理の 1 フレーム | `cargo run --release -p ugoe-physics --example physics_timing` | fixtures の設定で、物理だけ (`PhysicsSystem::update`・`update_instance`) の時間と、`dynamic_hair` の段の内訳 (1 段目の評価・物理そのもの・2 段目の評価。下の「1 フレームの評価」) |

### 1 フレームの評価

目標の「1 体を 1ms 以内 (中程度の PC、数千頂点) で評価する」の「評価」は、ホストが毎フレーム呼ぶ `Puppet::update(dt)` の 1 回
(モーション・トラッキング相当の入力・表情・目パチ・呼吸・物理・リップシンク・ポーズ・デフォーマの変形と頂点) と読みます。
描画・GPU への転送・ファイルの読み込みは含めません。スタジオの受け手が毎刻み呼ぶ `Puppet::follow` も同じ例で測ります。

- **数千頂点:** 例の中で作る生成のモデル (`puppet_timing.rs` の `medium_model`) を目安にする。メッシュ 144 枚・頂点 4,532・三角形 6,132、
  ワープ 14 個 (体 → 顔 → 目・目玉・眉・口・髪の入れ子) と回転 28 個 (体・頭・髪の房 3 段 × 6 本・腕 2 段 × 4 本)、
  キーフォームの格子は 2〜3 軸 (体のワープは体の角度 X・Y × 呼吸)、マスク (目玉・口の中・顔の影)、標準のパラメータ 21 個。
  物理は静的なリグ 3 つ (前髪・横髪・後ろ髪) と動的なリグ 7 つ (髪の房 6 本は衝突体あり、スカートは制御点を動かす)。
  4 秒でループするモーション (ベジェのカーブ 7 本・腕のポーズの切り替え・リップシンクの重み) と標準の目パチ・呼吸・リップシンクを動かし、
  頭と体の角度・目の開き・音量を毎フレーム書き込む。実際のリグの重さの目安として、見本の `assets/samples/avatar.ugoe`
  (メッシュ 24 枚・頂点 1,410・物理のリグ 4 つ・モーション `idle`) も同じ条件で測る。
- **中程度の PC:** 測った機械は下の表のとおりデスクトップの上位の CPU なので、中程度の PC (数年前のノート PC の 4〜8 コアなど) では
  1 コアの速さの差で数倍遅いと見込む。評価は 1 スレッドで動くので、コア数は効かない。
- **測り方:** 60fps で 6,000 フレーム進め、200 フレームずつの平均のうち最も速いものと中央値を出す (`physics_timing` と同じ考え方)。
  内訳を見るため、物理の段を止めた `update` と評価 (`ModelInstance::update`) だけも測る。

結果 (T0190。AMD Ryzen 9 7900 (12 コア 24 スレッド)・WSL2 の Linux、ほかのビルドが並行して load average 6 前後。5 回流した中央値、単位は µs):

| モデル | `update` | `follow` | 物理を止めた `update` | 評価だけ |
| --- | --- | --- | --- | --- |
| 生成のモデル (頂点 4,532) | 54.5 (最速 53.5) | 53.9 (53.0) | 46.1 (45.6) | 45.4 (45.0) |
| `avatar.ugoe` (頂点 1,410) | 18.0 (17.9) | 17.8 (17.7) | 17.2 (17.0) | 17.1 (16.9) |

どちらも 1ms の目標を大きく下回ります (生成のモデルで目標の約 5%。中程度の PC で数倍遅くても収まる)。時間の大半は評価
(キーフォームの補間・デフォーマの変形・頂点) で、生成のモデルでは物理 (動的なリグの 2 段階の評価を含む) が約 8µs、
モーション・効果・ポーズは 1µs ほどです。ほかの CPU では測っていません。

**wasm (ブラウザ)。** 計測のページ (`bindings/web/examples/timing/`。T0192) が、同じ 2 つのモデル (生成のモデルは
`puppet_timing -- --write` で `.ugoe` に書き出したもの) をブラウザの中で読み、`WebPuppet.update` (入力の書き込みとモーションの再生は
ネイティブと同じ) と受け手の `follow`、物理の段を止めた `update` を同じ測り方で測ります (評価だけ (`ModelInstance::update`) は
wasm の API に無いので測らない)。`@ugoe/runtime` の wasm (`just web-runtime` の `web-dist` のプロファイル。毎フレーム動くクレートは
`opt-level = 3`) を使います。時間は `update` (`follow`) の呼び出しだけを `performance.now()` で測り、200 回の和で平均します。
Chrome は cross-origin isolated でないと `performance.now()` を 100µs に丸めるので、ページを配る `serve.mjs` が COOP と COEP を付けて
5µs の刻みにします (ページに `crossOriginIsolated` と刻みを出す)。V8 は wasm をまず速いコンパイラ (Liftoff) で動かし、後から最適化
したもの (TurboFan) に替えるので、始めの数百フレームは遅く、フレームの数を減らすと中央値が上がります (1,200 フレームでは生成のモデルの
`update` が 113µs)。

結果 (T0193 で測り直した値。上と同じ機械、ヘッドレスの Chrome 154 (`HeadlessChrome/154.0.0.0`、tests/ui-audit と同じもの。描画の API は
WebGL2 (SwiftShader)。`update` は CPU だけで動くので効かない)、cross-origin isolated (刻み 5µs)。ほかのワーカーのビルドが並行して
load average 3〜4。5 回開き直した中央値、単位は µs。「前」は T0193 の直す前の main のビルド。前と後を交互に 2 度ずつ測り、どちらの回も
同じ傾向だった (2 度目の生成のモデルの `update` は前 75.4・後 61.6)。ネイティブは同じ時間帯に 5 回流した中央値):

| モデル | `update` | `follow` | 物理を止めた `update` | 物理の段 (差) | ネイティブの `update` | wasm / ネイティブ |
| --- | --- | --- | --- | --- | --- | --- |
| 生成のモデル (頂点 4,532)、前 | 71.6 (最速 69.6) | 71.4 (69.0) | 51.3 (50.8) | 約 20 | 55.4 (54.3) | 約 1.3 倍 |
| 生成のモデル (頂点 4,532)、後 | 61.4 (最速 60.5) | 61.5 (60.2) | 51.5 (50.7) | 約 10 | 55.4 (54.3) | 約 1.1 倍 |
| `avatar.ugoe` (頂点 1,410)、後 | 18.8 (18.4) | 19.0 (18.7) | 17.6 (17.1) | 約 1 | 19.3 (18.2) | 約 1.0 倍 |

wasm でも 1ms の目標を大きく下回ります (生成のモデルで目標の約 6%)。物理を止めた `update` (評価とモーション・効果) はネイティブの約 1.1 倍
(51µs と 48µs) で、物理の段はネイティブ (約 7µs) の約 2.7 倍から約 1.3 倍になりました (`avatar.ugoe` は衝突体が無く、物理の段が
もともと約 1µs なので変わらない)。中程度の PC のブラウザ・Safari・Firefox・WebGPU を有効にした Chrome では測っていません。

**物理の段が wasm で遅かった理由 (T0193)。** 物理の段 (`PhysicsSystem::update_instance`) を 1 段目の評価 (`update_first_stage`) と
物理そのもの (鎖の積分・衝突・出力) に分け、同じコードをネイティブと wasm (ヘッドレスの Chrome) で測り、Chrome の CPU プロファイラ
(関数名の節を残したビルド) で物理そのものの内訳を見ました (生成のモデル。モーションを動かさない分、上の表より少し軽い。中央値、µs):

| | wasm、前 | wasm、後 | ネイティブ |
| --- | --- | --- | --- |
| 1 段目の評価 | 4.6 | 4.6 | 2.9 |
| 物理そのもの | 17.9 | 8.5 | 6.3 |

- **見込み (三角関数などの `libm` の関数) は半分だけ当たっていた。** 時間の約半分 (48%) は平方根 (`libm::sqrtf`) で、三角関数
  (`sincosf`・`atan2f`) は 1 割に満たなかった。平方根は衝突の判定 (`collider.rs` の点と線分の距離) が、区間を回す掃引と二分法の
  1 回ごとに数個ずつ求めるので、1 フレームに数千回呼ばれる。
- **`libm` は wasm32 で平方根の命令 (`f32.sqrt`) を使わない。** `libm` は安定版の Rust では wasm32 の命令を呼べず (nightly の
  `unstable-intrinsics` が要る)、ソフトウェアで反復して求める (1 回約 8.6ns。命令なら約 0.9ns)。x86_64 では SSE の命令を使うので、
  ネイティブだけが速かった。さらに `@ugoe/runtime` の wasm は `libm` を `opt-level = "z"` でビルドしていた (毎フレーム動くクレートの
  一覧 `HOT_CRATES` に入っていなかった) ので、`sincosf` も約 1.5 倍遅かった。
- **直したこと。** `ugoe_core::math::sqrt` は `std` があれば `std` の `f32::sqrt` (命令) を使い、衝突の判定は長さを
  `math::sqrt(v.length_squared())` で求める (`glam` の `length` の中の平方根は `libm` のまま)。平方根は IEEE で正しく丸めると
  決まっているので、結果のビット列は変わらない (wasm で `std` と `libm` の平方根をすべての `f32` で比べて一致。物理の golden と適合テストも同じ)。
  `bindings/web/scripts/build.mjs` の `HOT_CRATES` に `libm` を足した (wasm は約 4KB 増え、gzip では変わらない)。
- **残り。** 物理そのものはネイティブの約 1.35 倍で、大半は衝突の判定の算術 (`WorldCollider::blocks` と掃引) です。1 段目の評価
  (約 1.6 倍) と三角関数 (`sincosf` 約 4.3ns、`atan2f` 約 6.1ns。ネイティブの約 2 倍) はネイティブより遅いものの、wasm の一般的な
  遅さの範囲で、直す安い手は見つかっていません。

**物理の段の内訳の調べ方 (T0334)。** 動的物理のある 1 フレームは、1 段目の評価 (`ModelInstance::update_first_stage`) → 物理そのもの
(鎖の積分・衝突・出力) → 2 段目の評価 (`ModelInstance::update`) と進みます。物理の段が遅くなったときは、まずネイティブでどの段かを分けます。

- **ネイティブ:** `puppet_timing` (生成のモデルと `avatar.ugoe`) と `physics_timing` (fixtures の `dynamic_hair`) が、`Puppet` を通さずに
  `ModelInstance` と `PhysicsSystem` を直接使い、`first stage`・`update_instance` (1 段目を含む)・`physics itself` (両者の差)・
  `second stage` を出します (行の意味は各例の先頭のコメント)。1 段目は `update_instance` の中で行うので、別の回で `update_instance` の前に
  呼んで測り、差を物理そのものとします。入力は `update` と同じに動かし、モーションと効果は通しません。生成のモデルと `avatar.ugoe` の段を
  `ugoe-physics` の例で測らないのは、`ugoe-physics` が `.ugoe` を読めない (`ugoe-format` に dev で依存すると crates.io に公開する順が循環する) ためです。
  手元の値 (T0334。上と同じ機械、load average 9 前後。最も速いもの / 中央値、µs):

  | モデル | 1 段目の評価 | `update_instance` | 物理そのもの | 2 段目の評価 |
  | --- | --- | --- | --- | --- |
  | 生成のモデル (頂点 4,532) | 2.9 / 3.1 | 8.9 / 9.6 | 6.0 / 6.5 | 43.7 / 44.3 |
  | `avatar.ugoe` (頂点 1,410) | 1.7 / 1.7 | 2.1 / 2.2 | 0.4 / 0.5 | 15.5 / 15.8 |
- **wasm:** 計測のページは `WebPuppet` の公開の口を通るので、段を分けられません (分かるのは物理を止めた `update` との差だけ)。
  段を分けたいときは、T0193 と同じく作業用のクレートをリポジトリの外 (scratchpad など) に作ります。
  1. `ugoe-runtime` と `ugoe-physics` に (path で) 依存する `cdylib` のクレートを作り、`physics_timing` の段の計測と同じループを書く。
     時刻は JS の `performance.now()` を `#[wasm_bindgen]` の `extern` で import して取る (`std::time::Instant` は wasm32-unknown-unknown で使えない)。
     同じコードをネイティブ (`Instant`) でも動かせるようにしておくと、wasm / ネイティブの比が段ごとに出る。
  2. `cargo build --release --target wasm32-unknown-unknown` で、関数名の節 (`name`) を残してビルドする (`[profile.release]` に `strip = false` を書き、
     wasm-bindgen に `--remove-name-section` を付けず、wasm-opt を通さない)。`@ugoe/runtime` と同じ条件にするなら、毎フレーム動くクレートの `opt-level` を
     `bindings/web/scripts/build.mjs` の `HOT_CRATES` に合わせる。
  3. 計測のページと同じく COOP と COEP を付けて配り (刻みを 5µs にするため。`serve.mjs` を参考に)、ヘッドレスの Chrome で開いて段ごとの時間を出す。
  4. 関数ごとの内訳が要るときは、同じページを Chrome DevTools Protocol の `Profiler` (`Profiler.enable`・`Profiler.setSamplingInterval`・
     `Profiler.start`・`Profiler.stop`) で記録し、プロファイルのノードを関数名で `update_instance` の下 (`update_first_stage` を除く)
     に絞り、自己時間を数える。T0193 はこれで平方根 (`libm::sqrtf`) が物理そのものの 48% を占めることを見つけました。
- **ネイティブの関数ごとの内訳:** perf のある Linux なら `perf record` で `physics_timing` を記録します。T0193 の手元 (WSL2) は
  カーネルに perf が無く、時間を段に分けて測るだけでした。

### 遅くなったら気づく検査

変更で評価が遅くなったときに気づけるよう、ネイティブの `puppet_timing` に上限の判定 (`--check`。T0191) を、wasm (ブラウザ) の計測のページを
開く `measure.mjs` に上限の判定 (`--max-us`。T0194) を持たせ、`just perf` で両方を流します。どちらも `update` と `follow` の中央値が
上限を超えたら、超えたものを `OVER` と示して終了コード 1 で終わります。

| レシピ | 測るもの | 要るもの |
| --- | --- | --- |
| `just perf-native` | `puppet_timing -- --check` (release) | Rust だけ |
| `just perf-wasm` | `just web-runtime` と `puppet_timing -- --write` で生成物を毎回作り直してから (古いビルドを測らないように)、`measure.mjs --max-us 300` (ページを 1 回開く) | wasm-bindgen-cli (`Cargo.lock` と同じ版) と Chrome か Chromium (`CHROME` で場所を指定できる) |
| `just perf` | 上の 2 つを順に (ネイティブが落ちたら wasm は流さない) | 両方 |

wasm-bindgen-cli が無ければ `just web-runtime` が入れ方を出して失敗し、Chrome が無ければ `measure.mjs` が `Chrome is not found` と出して
終了コード 2 で終わります (飛ばして成功にはしない。上限を確かめずに通ったと誤解しないため)。手元で Chrome を入れないなら `just perf-native` を流します。

- **方針 (利用者の決定 2026-10-08):** 時間の上限を `just perf` と、push・PR ごとではない CI のジョブ (`.github/workflows/perf.yml`。
  `release-checks.yml` が呼ぶ。単独でも手で起動できる。T0091) で確かめます。
  新しい依存や道具は入れません。共有の CI のランナーでは時間が 2〜3 割揺れるので、PR ごとの CI (`just ci`) で時間を比べると関係の無い
  PR が落ちます。前の値と比べるベンチマーク (`criterion` など) は前の値が別の機械のものになって誤報が多く、命令の数を数える方法
  (`iai-callgrind` と valgrind) は依存と道具が増えるため、今は入れません。PR ごとに細かい後退を止めたくなったら、そのとき命令の数を足します。
- **上限 (1 フレームの中央値。ネイティブは `puppet_timing.rs` の `LIMITS`、wasm は `just perf-wasm` の `--max-us`):**

  | モデル | ネイティブの `update`・`follow` | 手元の値 (上の表) | wasm の `update`・`follow` | 手元の値 (上の表) |
  | --- | --- | --- | --- | --- |
  | 生成のモデル | 300µs | 55µs ほど | 300µs | 60µs ほど (T0193 の前は 70µs ほど) |
  | `avatar.ugoe` | 100µs | 18µs ほど | 300µs | 18µs ほど |

- **決め方:** GitHub の `ubuntu-latest` のランナー (共有の仮想機械) は、手元の機械 (上の表) より 1 コアの速さで 2 倍ほど遅く、
  さらに 2〜3 割揺れると見込み (生成のモデルで 150µs 弱、`avatar.ugoe` で 50µs 弱)、その 2 倍ほどを上限にしました。
  生成のモデルの上限 300µs は、中程度の PC がランナーの 3 倍遅くても目標の 1ms に収まる値でもあります。揺れで落ちないことを優先したので、
  ランナーでは 2 倍ほどの後退と、目標を割りそうなことに気づけます。1〜2 割の細かい後退は揺れに埋もれて分かりません。
- **wasm の上限 (利用者の決定 2026-10-08、T0194):** ネイティブの生成のモデルと同じ 300µs を、すべてのモデルの `update` と `follow` に 1 つの値で
  使います。手元の中央値は回ごとに 3 割ほど揺れました (生成のモデルの `update` で 67〜87µs) が、それに対しても 3 倍以上の余裕があり、
  ランナーが 2 倍遅くても下回ります。`avatar.ugoe` には緩く (手元の値の 15 倍ほど)、wasm で気づけるのは主に生成のモデルの後退です
  (`avatar.ugoe` の小さな後退はネイティブの 100µs の上限で見ます)。物理を止めた `update` は内訳なので比べません。
- **揺れへの備え:** 中央値は 200 フレームずつの平均 30 個の中央値で、一時の重なりにはもともと強くしてあります。それでも上限を超えたものは
  2 回まで測り直し、最も速い中央値で判定します (`RETRIES`。wasm はページを開き直して測り直す。`bindings/web/examples/timing/check.mjs`)。手元で流すときは、ほかのビルドと重ならないときにします
  (ほかのビルドが並行して load average 16 ほどの手元の機械でも、生成のモデルの `update` は 80µs ほどで上限を下回りました)。
- **CI:** `perf.yml` は CPU (`lscpu`) を出してから `just perf-native` と `just perf-wasm` を別の段で流します (ネイティブが落ちても wasm を流す)。
  wasm の段のために Node と pnpm の依存、ビルド済みの wasm-bindgen-cli を入れ、Chrome は `ubuntu-latest` に入っているものを使います。
  キャッシュの無いときの見込みは release と web-dist のビルドがそれぞれ 10 分ほど、計測が合わせて 2 分ほどで、`timeout-minutes: 30` に収まります。
  落ちたら、まず CPU と測った値を見て、ランナーが遅いのかコードが遅いのかを見分けます。ランナーで測った値が分かったら、この節に書き、上限が緩すぎ・きつすぎなら見直します。

## 描画テスト

- `ugoe-render` の描画テストは、オフスクリーンのテクスチャに描画して画素を読み戻し、期待する画像と比較します (許容誤差付き)。
- GPU を使うテストは各クレートの `gpu-tests` feature の裏に置きます。`just test-gpu` は `scripts/gpu-test-args.mjs` で `gpu-tests` feature を持つワークスペースのクレートを探し、まとめて `cargo test -p <crate> --features <crate>/gpu-tests` します。新しいクレートに描画テストを足すときは feature を定義するだけでよく、justfile や CI を直す必要はありません。
- アダプタが見つからないときは、飛ばさずに失敗させます (feature を明示的に有効にしたときだけ動くため)。
- 期待画像は `crates/ugoe-render/tests/golden/` に置きます。更新するときは PR に新旧の画像を載せます。
- `just test-gpu -- --nocapture` とすると、テストが選んだアダプタ (名前・ドライバ・バックエンド) が表示されます。
- テストは 1 つのデバイスをテストのスレッドの間で共有して並行に使うことが多いです。読み戻しで `Device::poll` が戻っても、`map_async` のコールバックはほかのスレッドの `poll` が集めたまま、まだ呼ばれていないことがあります (wgpu はロックを外してから呼ぶ)。待ち終わったらコールバックが呼ばれている、と決めつけず、呼ばれるまで上限のある回数で poll し直します (`ugoe-headless` の `Gpu::render` と、ugoe-render・ugoe-ai の描画テストの `tests/common/mod.rs` の `map_read` が例。T0963・T0964)。wgpu-core 30 はマップの状態をロックの中で変えるので、コールバックの前でも `get_mapped_range` は読めてしまいますが、それは文書にない実装の都合なので頼りません。手元で GL の揺れを確かめるときは `WGPU_BACKEND=gl LIBGL_ALWAYS_SOFTWARE=1` を付け、`-- --test-threads=16` などで並行を増やして何度か流します。

### CI での実行

GPU のない GitHub Actions の Linux ランナーでは、Mesa のソフトウェア実装で実行します (`.github/workflows/release-checks.yml` の `gpu` ジョブ)。バックエンドごとに matrix で 2 本走ります。

| matrix | ドライバ | 意味 |
| --- | --- | --- |
| `vulkan` | lavapipe (Vulkan) | ネイティブの主経路 |
| `gl` | llvmpipe (OpenGL ES / EGL) | WebGL2 相当のバックエンドでも動くことの確認 |

使う環境変数:

| 変数 | 値 | 目的 |
| --- | --- | --- |
| `WGPU_BACKEND` | `vulkan` / `gl` | wgpu が使うバックエンドを 1 つに絞る (テストは `InstanceDescriptor::new_without_display_handle_from_env` で読む) |
| `VK_DRIVER_FILES` (と旧名の `VK_ICD_FILENAMES`) | `/usr/share/vulkan/icd.d/lvp_icd*.json` | Vulkan ローダーが lavapipe 以外のドライバを読まないようにする |
| `LIBGL_ALWAYS_SOFTWARE` | `1` | GL を llvmpipe に固定する |

ジョブは Tauri のシステム依存を入れず、`just test-gpu` が指定するクレートのテストだけをビルドします。Tauri のアプリ (`scripts/ci-groups.mjs` で `tauri: true` のグループ。スタジオの仮想カメラの画のテスト) の `gpu-tests` は、環境変数 `CI` があると `scripts/gpu-test-args.mjs` が外すので、手元の `just test-gpu` だけで走ります。ログの `vulkaninfo --summary` (vulkan のみ) と `ugoe-render gpu tests: using adapter ...` の行で、どのドライバで走ったかを確認できます。

### 手元で CI と同じ条件で動かす

Mesa (`mesa-vulkan-drivers` `libegl-mesa0` `libgl1-mesa-dri`) が入っていれば、手元の GPU があってもソフトウェア実装に固定して再現できます。

```sh
icd="$(ls /usr/share/vulkan/icd.d/lvp_icd*.json | head -n 1)"
# lavapipe (Vulkan)
VK_DRIVER_FILES=$icd VK_ICD_FILENAMES=$icd WGPU_BACKEND=vulkan just test-gpu -- --nocapture
# llvmpipe (GL)
WGPU_BACKEND=gl LIBGL_ALWAYS_SOFTWARE=1 just test-gpu -- --nocapture
```

WSL2 では環境変数を付けないと dzn (Vulkan on D3D12) が選ばれることがあります。dzn はインスタンスの破棄時に SIGSEGV を起こすので、テストはデバイスを static に置いて破棄しないようにしています (`crates/ugoe-render/tests/gpu.rs`)。


## 配布物の適合テスト

同じモデル・同じパラメータから、すべての配布物が同じ頂点と画像を出すことを検査します ([ADR 0017](adr/0017-single-runtime-distribution.md) の決定 3)。
参照は `ugoe::Puppet` (`ModelInstance`) の評価の結果と `ugoe render` の画像で、許容誤差は公開仕様の評価と描画の適合の基準
(画像は不透明な背景 `#808080` の上で比べる)。ケースは `tests/conformance/cases.json` (見本のモデル・着せ替え・ライティング・
時間を進めた物理と、公開仕様の評価の例・バンドルの例)。

```sh
just conformance            # C ABI・wasm (ヘッドレスの Chrome の WebGL2 と WebGPU)・<ugoe-avatar>・問い合わせの JSON のキー・Unity (dotnet)・Godot。dotnet・Godot が無ければ飛ばす
just conformance --only c   # C ABI だけ
```

ランタイムの評価・描画・C ABI・wasm の API を変えたら実行します。手順・計画の形式・エンジン向けのバインディングを足す手順は
[tests/conformance/README.md](../tests/conformance/README.md)。

`<ugoe-avatar>` (`avatar`) は参照と比べず、作成・切り離し・作り直しのシナリオを本物の wasm とヘッドレスの Chrome (WebGL2 は SwiftShader) で
確かめます。WebGL2 のコンテキストの喪失 (`WEBGL_lose_context`) のほか、描いている間と作成の途中に GPU プロセスを落とすシナリオ
(`gpu-process-crash`・`gpu-process-crash-during-create`。T0484) もあり、落とし方と Chrome の起動の設定は下の
[PuppetSurface のブラウザのテスト](#puppetsurface-のブラウザのテスト) と揃えています (run.mjs が CDP の `Browser.crashGpuProcess` を、
playwright を使わずに WebSocket で送る)。wasm の panic があればそのシナリオは失敗します。シナリオの一覧は
[tests/conformance/README.md](../tests/conformance/README.md#ugoe-avatar-のシナリオ)。

見本 (`bindings/web/examples/basic`) の「camera (MediaPipe)」が Face Landmarker を作った後に、MediaPipe の利用状況の送信
(`odml.pa.googleapis.com`) がネットワークに出ないことは `example` で確かめます (`just conformance --only example`。T0487)。
ヘッドレスの Chrome の偽のカメラで見本を開き、CDP の `Network.requestWillBeSent` で要求を記録します。MediaPipe とモデルは CDP の
`Fetch` で手元のファイル (packages/face-tracking の `@mediapipe/tasks-vision` と、`mediapipe-assets.mjs` が取得したモデル) から応え、
送信の間隔 (60 秒) は 1 秒に縮めます。見本の包みを外した対照でも開き、要求が記録されることも確かめます。CI では `conformance` ジョブの
`just conformance` で走ります。詳細は [tests/conformance/README.md](../tests/conformance/README.md#見本の-mediapipe-の利用状況の送信)。

## PuppetSurface のブラウザのテスト

エディタのビューポートと Player が共有する `PuppetSurface` (`packages/player/src/surface.ts`。[docs/ui.md](ui.md#webpuppet-の寿命)) と、
それを使う `Player` (`player.svelte.ts`) を、本物の wasm とヘッドレスの Chrome (WebGL2 は SwiftShader) で確かめます (T0936・T0939・T0944)。
順序の細部は偽の wasm の Vitest (`surface.test.ts`・`player*.test.ts`) が確かめるので、ここでは本物の GPU と wasm でも通ることを見ます。

```sh
just player-browser                                    # ugoe-wasm をビルドしてすべてのシナリオ
just player-browser --no-build --only context-loss-restore   # 前にビルドした wasm で、選んだシナリオだけ
```

- **シナリオ (`PuppetSurface`):** 作って描いて大きさを変えて捨てる・WebGL2 のコンテキストを失ったら復元を待って新しい canvas で作り直す (2 回続けて)・
  復元が届かなければ `waiting` で知らせ、`RESTORE_TIMEOUT_MS` の後に新しい canvas で作り直す (`context-loss-timeout`)・
  続けて失って諦め、次の `attach` で作れる・諦めた後の `restart` で作り直し、回数も数え直す (`restart-after-giving-up`)・
  失った WebGL2 のコンテキストしか取れない canvas では `WebPuppet.create` が panic せずに `CreateError` (`context_unavailable`) で失敗し、
  その後も wasm が使える (`create-on-lost-context`)・最初の作成でそうなれば `attach` は `"failed"` で `failed` に例外を渡す (`attach-on-lost-context`)・
  作り直しでそうなれば待って試し直し、作り直しの回数に数える (`recreate-on-lost-context`。T0475)・
  GPU の電力の希望 (`powerPreference`) を渡すと WebGL2 のコンテキストがその属性 (`antialias: false` のまま) で作られて描け、`adapter_name` は
  `WEBGL_debug_renderer_info` があれば名前・無ければ空、知らない希望はコードを持たない `Error`、失ったコンテキストでは希望を渡しても
  `context_unavailable`、`PuppetSurface.powerPreference` は作成と作り直しの両方で渡る (`power-preference`。T0499)・作っている間の `detach` と次の `attach`・devicePixelRatio だけが変わったら (1 → 2 → 1.5 → 1)
  描画バッファ (WebPuppet と canvas) を新しい倍率に合わせ直し、`detach` の後は聞かない (`pixel-ratio-change`)。どのシナリオの後も、生きている
  WebPuppet と canvas が残らないことを確かめます。喪失と復元は `WEBGL_lose_context` で起こします。
- **シナリオ (GPU プロセスの喪失。T0944):** 描いている間に GPU プロセスが落ちたら、`PuppetSurface` は WebGL2 の喪失として復元を待ち、新しい canvas で
  作り直して描く (2 回続けて。`gpu-process-crash`)。`Player` はモデル・モジュール・値を戻して描く (`player-gpu-process-crash`)。
  作成の途中の喪失は 3 つで確かめます: 作成の WebGL2 の呼び出しを数え、そのどの呼び出しの前で失っても `WebPuppet.create` が `CreateError`
  (`context_unavailable`) で失敗する (`lose-context-during-create`。決まった時機で、走るたびに同じ)・GPU プロセスが落ちた直後の Chrome のように、
  リンクできたプログラムの `getUniformBlockIndex` が `INVALID_INDEX` を返しても panic しない (`invalid-block-index-during-create`)・
  Node の側から時機をずらして GPU プロセスを落とし、その間は作って読み込んで描くことを繰り返しても、失敗はコードを持つ例外だけで panic しない
  (`gpu-process-crash-during-create`。どの段に当たるかは走るたびに変わる)。**wasm の panic** (`console_error_panic_hook` の `panicked at`) が
  1 つでもあれば、どのシナリオも失敗します。
- **シナリオ (`Player`):** 着せ替えの見本 (`dress-up.ugoe` とモジュール 1 つ) を開いてパラメータを動かし、WebGL2 のコンテキストを失って復元したら、
  新しい WebPuppet でモデルを読み直し、モジュールを付け直し、値を戻して描き (`capture` の読み戻しで確かめる。2 回続けて)、利用者のズームとパン
  (`view`) を保ち、その後に開き直したモデルは fit で見せる (`player-loss-restore`。T0942)。
  失ったら `waitingForGpu` になり、復元が届かなければ時間切れで作り直し、続けて失って諦めたら `canRestart` になり、`restart` で
  モデル・モジュール・値・ビューを戻して描く (`player-timeout-restart`)。
  devicePixelRatio だけが変わったら、canvas と読み戻しの大きさが新しい倍率になり、CSS ピクセルで見たビュー (`view`) は動かず、
  CSS ピクセルの点を中心にしたズームでその点が動かない (換算に今の倍率を使う。`player-pixel-ratio`)。
  描画バッファの倍率 (`renderScale`。T0858) を 0.5・0.75 (devicePixelRatio 1 と 2)・1 と変えると、canvas と読み戻しが「CSS ピクセル ×
  devicePixelRatio × 倍率」の画素になり、canvas は表示の大きさ (テストのページの CSS で引き伸ばす) のまま、ズームとパンを動かした後の `view` は動かず、
  点を中心にしたズームでその点が動かない (`player-render-scale`。パンを直さないと落ち、devicePixelRatio の変化でパンを直していなかった前も落ちた)。
  ポーズの欄 (`poseShown`) は本物の wasm の表示中の要素 (`pose_visible_json`) を読み、見本の `wave` で `HandsDown` → `Wave` → `HandsDown` と
  追従し、モーションが終わった後に選んだ要素は残る (`player-pose-follows-motion`。T0736。追従をやめると落ちることを確かめた)。
  コンテキストを失っている間 (`load_bundle`・`load_module` が `device_lost`) に初めのモデルを開いても、別のモデルとモジュールを開いても、
  `error`・`dressUpError` を出さず、復元の後にそのモデル (とモジュール) を開いて fit で描く (`player-loss-during-load`。T0955。
  直す前には落ちることを確かめた)。
  受け手のモードで、刻み 30 のフレームを約 2 秒届けると、描く回数 (`render` の呼び出し) はフレームの数ほどで `requestAnimationFrame` の回数より
  少なく (結果の行に数を出す。毎フレーム描いていた前は 60 フレームで 122 回、今は 60 回)、フレームが止まった後は描かないが、ページの
  スクリーンショット (run.mjs の `screenshot`。合成された画面) で最後の形が残り、ズームすると描く (`player-follower-draws-on-change`。T0857)。
  ふつうのモード (ビューア) で、待機モーション・目パチ・呼吸を止めた `dress-up.ugoe` は、読んだ後 1 回描くと描かなくなり (結果の行に
  数を出す)、描かない間もスクリーンショットで最後の形が残り、読み戻し (`capture`) は空でなく、パラメータを動かすと描いて画が変わり、
  ズームすると 1 回描く (`player-viewer-draws-on-change`。T0982。毎フレーム描く前の `#tick` では落ちることを確かめた。Player は呼吸を
  止める口を持たないので、ページが `load_bundle` を包んで `set_enabled("breath", false)` を呼ぶ)。
- **倍率の変化の起こし方:** run.mjs が CDP の `Emulation.setDeviceMetricsOverride` でページの大きさを変えずに倍率だけを変え、ページに
  `setDevicePixelRatio(ratio)` として出します。ヘッドレスの Chrome (154 で確かめた) は倍率の上書きだけではメディアクエリを評価し直さず、
  `matchMedia` の `matches` は変わっても `change` が届きません (ページの大きさも変えると届く)。そこで上書きのたびに、エミュレートする
  メディアの型を `""` と `"screen"` (どちらも画面) で交互に切り替えて評価し直させます。届いた `change` で聞き直す (`{once: true}` の後に
  新しい倍率の問い合わせを作る) 処理と描画バッファの大きさは本物で確かめますが、倍率の違う画面へウィンドウを移したときに本物の
  ブラウザ (と Tauri の WebView) が `change` を届けることは確かめていません (MDN の `matchMedia` の例にある、ブラウザの振る舞い)。
- **WebGL のコンテキストの数 (T0938):** Chrome は生きている WebGL のコンテキストを 16 までにし、超えると古いものから失わせます。作り直しを
  繰り返すシナリオでは、捨てた canvas のコンテキストも GC されるまで数に入り (復元したものは生き返る)、今のコンテキストが失われて
  落ちることがあるので、run.mjs は Chrome を `--max-active-webgl-contexts=256` で起動します (`--max-webgl-contexts N` で変えられる)。
  失ったコンテキストを `getContext` が返す状態は、シナリオが canvas のコンテキストを先に作って `WEBGL_lose_context` で失わせて作ります (T0475)。
- **GPU プロセスの落とし方 (T0944):** run.mjs がブラウザの CDP の `Browser.crashGpuProcess` で落とし、ページに `crashGpuProcess(delay?)` として
  出します (`delay` を渡すと待たずに戻り、Node の側から落とすので、ページの JS が同期的に動いている間 (wasm の作成の途中) にも落とせる)。
  ヘッドレスの Chrome (154) と SwiftShader で落とせ、落ちた canvas には `webglcontextlost` が届き、約 1 秒後に `webglcontextrestored` が届きます。
  Chrome は GPU プロセスが続けて落ちると GPU を諦め (SwiftShader の後に代わりが無いので、6 回ほどでブラウザごと終わる)、落ちたときに
  WebGL を使っていたページの WebGL を止めます (以後 `getContext` が `null`)。何度も落とすため、run.mjs は Chrome を
  `--disable-gpu-process-crash-limit` と `--disable-domain-blocking-for-3d-apis` で起動します (本物の Chrome では、続けて落ちると
  `context_unavailable` で作れなくなる。`PuppetSurface` は待って試し直し、上限で諦める)。
  作成の途中の決まった時機の喪失は、シナリオが `WebGL2RenderingContext` のプロトタイプのメソッドを包んで呼び出しを数え、N 回目の前に
  `loseContext` を呼んで起こします (wasm-bindgen のグルーはその場でメソッドを引くので包める)。
- **GPU プロセスの喪失で見つけた panic (T0944):** 作成の途中で失うと、glow の `from_webgl2_context` (`getSupportedExtensions()`・`getParameter(VERSION)`
  の `unwrap`) と wgpu-hal の `create_render_pipeline` (`getUniformBlockIndex()` の `unwrap`) で panic していました (panic の後は作成の Promise が
  決着しない)。後のものは `loseContext` では起きず (失ったコンテキストの `getUniformBlockIndex` は 0 を返す)、GPU プロセスを落としたときだけ、
  `isContextLost()` が偽のまま `INVALID_INDEX` が返って起きます。ugoe-wasm がこの 3 つを包んで直しました
  ([crates/ugoe-wasm/README.md](../crates/ugoe-wasm/README.md#作成の失敗-t0475))。直す前は、作成だけを 120 回ほど落としながら繰り返すと数回 panic し、
  直した後は (作成・読み込み・描画を繰り返しても) 0 回でした (T0944 のメモ)。
- **テストが壊れを見つけること (T0939 で確かめた):** `surface.ts` で、倍率の問い合わせを `attach` で作らない・`change` の後に聞き直さない・
  `detach` で聞くのをやめない・`change` で大きさを合わせ直さない、のどれでも `pixel-ratio-change` が落ちます。`player.svelte.ts` で、
  作り直しの後に値を戻さない・モジュールを付け直さない (`player-loss-restore`)、ズームの換算に倍率を掛けない (`player-pixel-ratio`) でも落ちます。
  `surface.ts` で作り直した WebPuppet にビューを渡さない (T0942) と、`player-loss-restore`・`player-timeout-restart` が落ちます。
- **エディタの `ViewportController` はここでは対象にしていません (T0939 で決めた):** `PuppetSurface` を通る経路 (作り直しで表示の設定とモデルを渡し直す・
  諦めたら状態と文を出す・倍率の変化で描き直しを頼む・CSS ピクセルと描画バッファの画素の換算) は Player と同じ形で薄く、本物の wasm で
  確かめたい部分 (作り直し・倍率・換算) は上の `PuppetSurface` と `Player` のシナリオで通ります。順序は偽の wasm の
  `apps/editor/src/lib/viewport/controller.test.ts` が確かめます。`@ugoe/player` のテストからアプリ (`apps/editor`) のコードは読みません
  (パッケージからアプリへ向きが逆になる)。エディタが作り直しのまわりに本物の wasm に頼る自分の処理を持ったもの (プレビューの着せ替えの
  読み込みの途中の喪失。T1644) は、ビルドしたエディタの画面を動かす [エディタのブラウザのテスト](#エディタのブラウザのテスト) で確かめます (T1656)。
  T0938 で足した「作り直す」ボタン (`ViewportController.restart`) は、状態を変えて `PuppetSurface.restart` を呼ぶだけなので、本物の wasm の部分は
  `restart-after-giving-up` で通ります (T0038)。
- **置き場所:** `packages/player/browser-test/` (GPL-3.0-only)。テストは GPL の `surface.ts`・`player.svelte.ts` を読むので、MIT の
  `tests/conformance` (と、ライセンスを書いていないのでリポジトリの既定の MIT になる `tests/ui-audit`) には置きません。Chrome の起動は
  `just ui-audit` と同じ playwright-core (ルートの devDependencies)。`@ugoe/player` を Vite (アプリと同じ版を `@ugoe/player` の
  devDependencies に入れた) と Svelte のプラグインで 1 つの ES モジュールにまとめて配ります (runes と依存を含むため)。
- **CI:** `conformance` ジョブ (Chrome と wasm-bindgen-cli が入っている) で、適合テストの後に流します。落ちたときはページのコンソールを
  `target/player-browser/page.log` に残します。
- WebGPU のデバイスの喪失は、ヘッドレスの Chrome で WebGPU を有効にしていないので確かめていません (偽の wasm のテストだけ。
  `Browser.crashGpuProcess` で起こせる見込みはあるが試していない)。本物の GPU (ドライバのリセットなど) での喪失は docs/manual-check.md の D09。

## エディタのブラウザのテスト

ビルドしたエディタの画面を、本物の wasm とヘッドレスの Chrome (WebGL2 は SwiftShader) で動かし、偽の wasm の Vitest では確かめられない
GPU の喪失のまわりを確かめます (T1656)。Tauri の代わりに [UI の監査](#ui-の監査) の偽のホスト (`tests/ui-audit/lib/editor-host.mjs`) を置き、
画面は Playwright で操作します。詳しくは [apps/editor/browser-test/README.md](../apps/editor/browser-test/README.md)。

```sh
just editor-browser                                              # wasm・エディタ・見本をビルドしてすべてのシナリオ
just editor-browser --no-build --only dress-up-loss-during-load  # 前のビルドで、選んだシナリオだけ
```

- **シナリオ:** プレビューの着せ替えで、WebGL2 のコンテキストを失っている間 (`WEBGL_lose_context`) にモジュールを読み込んでも
  (`load_module` が `device_lost`) パネルに失敗の文を出さず、作り直した WebPuppet に付けて描く (`dress-up-loss-during-load`)・
  失っている間の付け外し (チェックで外す・スロットの選択で差し替える) が、作り直した後に付けたい組で付く (`dress-up-change-while-lost`)。
  付いたことは本物の WebPuppet (`attached_modules`) と一覧の行で、描いたことは `read_pixels` の読み戻しで確かめます。T1644 の前の
  `dressUp.svelte.ts` では 2 つとも落ちます。
- **待ち方:** 作り直しの完了 (新しい WebPuppet が描き、ビューポートの「GPU の復元を待っています」が消える) を待ってから確かめます。
  作り直しの `WebPuppet.create` を 500 ms 遅らせて、CI の SwiftShader のように作成が遅いときも作り終える前に確かめないようにしています
  ([PuppetSurface のブラウザのテスト](#puppetsurface-のブラウザのテスト) の T0960 と同じ)。
- **e2e ではなくここにした理由 (T1656):** 喪失と復元を Chrome の `WEBGL_lose_context` で決まった時機に起こせ、本物の WebPuppet を
  ページから直接見られ、1 つのシナリオだけを速く何度も流せるため。Tauri と WebView の間 (IPC・ファイルのダイアログ) は見ないので、
  着せ替えの流れそのものは [実機の e2e テスト](#実機の-e2e-テスト) の手順が見ます。
- **CI:** `conformance` ジョブで `just player-browser` の後に流します。落ちたときはページのコンソールを `target/editor-browser/page.log` に残します。

## エディタの部品のテスト

エディタ (`apps/editor`) の Vitest は既定でノードの環境で動き、純粋な関数と `.svelte.ts` の状態を確かめます。
ポインタやキーの操作から送るコマンドまでの流れ (ビューポートのツールなど) は、部品を描いて確かめます (T0699)。

- **環境:** ファイルの先頭に `// @vitest-environment happy-dom` を書き、そのファイルだけ happy-dom (ノードの DOM の実装) で動かします。
  テストのときは Svelte のブラウザ向けの実装を読みます (`vite.config.ts` の `resolve.conditions`。`packages/ui` の部品のテストと同じ)。
- **描く:** svelte の `mount` で body に描き、テストの後で片付けます。ビューポートのツールの道具は
  `apps/editor/src/lib/viewport/tools/test/harness.svelte.ts` にあります (`render`・`pointer`・`drag`・`press`・`dialogButton`・Select の項目を選ぶ `choose`)。
- **偽物:** `ViewportController`・`EditorDocument`・`WebPuppet` は偽物 (`FakeController`・`FakeDocument`・`fakePuppet`) を渡します。
  `FakeDocument.apply` は送られたコマンドを `commands` に記録するので、コマンドの列をそのまま比べます。`gate` に約束を入れると、
  `apply` がそれを待つので送っている間の操作を試せます。送っている数は本物と同じく `pending` に数えます
  (パネルが `pending > 0` の間ボタンを無効にするのを確かめられる)。バックエンドの座標変換 (`ipc.localToCanvas`・`ipc.canvasToLocal`) は `vi.mock` で恒等写像にします。
- **wasm:** テストは本物の wasm (`just wasm` の生成物) を読みません。`src/test-setup.ts` が `viewport/wasm` の `loadUgoeWasm` を
  null を返す偽物にします (生成物があると、部品が読みに行った fetch がテストの片付けで中断され、処理されない rejection で Vitest が落ちるため。T1641)。
  wasm の型が要るテストは、ファイルの中で偽物を返す `vi.mock` を書き直します (`panels/timeline/timeline-state.test.ts` の `MotionSampler`)。
- **座標:** happy-dom の `getBoundingClientRect` は 0 を返すので、イベントの `clientX`・`clientY` がそのまま部品の座標 (CSS ピクセル) になります。
  偽のビューの変換 (`TRANSFORM`) は拡大率を 1 以外にして、CSS ピクセルとキャンバス座標の換算も確かめます。
- **確かめること:** 離したときに送るコマンド (1 つの Undo の単位)、ドラッグ中はプレビューだけで何も送らないこと、`pointercancel` では送らずに
  元に戻すこと、送っている間の操作、window で受けるキー (Esc・Delete・Ctrl (Cmd) + A) がダイアログ・入力欄の中では効かないこと。
- **読めないモデル:** `doc.parsedModel` が null (モデルの JSON が読めない) でも部品が例外を出さずに描けることを、ビューポートに重ねる部品
  (`viewport/unreadableModel.test.ts`)・主なパネル (`panels/unreadableModel.test.ts`)・プレビューの部品 (`PreviewPanel`・`DifferenceOverlay`。
  `preview/unreadableModel.test.ts`) で確かめます。新しいパネルや重ねる部品を作ったら、ここに足します。プレビューの部品には `PreviewMode` の偽物
  `FakePreview` (`preview/test/fakePreview.svelte.ts`。部品が読む状態を持ち、操作を `calls` に記録する。トラッカーと着せ替えは本物。
  `@ugoe/face-tracking` の読み込みが重いので `harness.svelte.ts` と分ける) を渡します。`harness.svelte.ts` の `FakeController` には
  プレビューが読む `message`・`usesLighting` と、ビューを寄せる `zoomAt`・`panBy` (記録する)、`fakePuppet` には `has_model`・
  `attached_modules`・`physics_json` があります。
- happy-dom は描画もレイアウトもしないので、見た目 (重なり・CSS の `pointer-events`) は確かめられません。それは実機の e2e テストで確かめます。

## UI の監査

3 つのアプリの画面を同じ物差しで繰り返し確かめる道具です (T0915)。基準は [docs/ui.md の「UI の品質の基準」](ui.md#ui-の品質の基準)。
M10 の UI のタスクは、直す前と後にこれを流して比べます。

```sh
just ui-audit --out target/ui-audit-before   # 直す前に撮る (3 つのアプリ・すべての場面。5 分ほど)
just ui-audit --only editor --quick          # 反復中: アプリを選び、ライトの英語の広い幅と狭い幅だけ
just ui-audit --no-build --compare target/ui-audit-before   # 直した後: 前の結果と比べて、増えた・消えた種類を出す
```

- **開き方:** `just wasm` と `vite build` でビルドした画面を小さな静的サーバで配り、playwright-core で入っている Chrome を動かす。
  Tauri の代わりに偽のホストを置く (エディタは読むだけの IPC の偽物と `ugoe sample dress-up` の見本、スタジオは `StudioHost` の偽物と
  見本のプロファイル、ビューアはブラウザの口のまま)。偽物は画面を見るためのもので、編集・保存の流れは Vitest と実機の e2e が確かめる。
- **場面:** ライト・ダーク × 英語・日本語 × 広い幅・狭い幅 (エディタ 1440 と 1024、ほか 1280 と 360) と、devicePixelRatio 2・200% の拡大
  (CSS の窓の幅が半分。英語と日本語)・強制色 (Windows の高コントラストのエミュレーション。暗い広い幅と明るい狭い幅)、ビューアとスタジオの 1600px。それぞれで空の状態・見本を開いた画面・下まで送った画面・設定・メニュー・エディタの右の列のタブなどを撮る。
  どれも `prefers-reduced-motion: reduce` で開く。
- **検査:** 文字のコントラスト (canvas・画像の上の文字は、文字を消して撮った画素で測る)・横のはみ出し (送る枠の外を位置の基準にする absolute の要素も)・文字と押せる部品の重なりと切れ・24px 未満の押せるもの・名前の無い部品 (axe-core)・そのほかの ARIA の規則
  (axe-core)・ランドマークと見出しの階層 (axe-core)・reduced motion の下の動き・強制色で見分けられない状態・画面に出たエラー・Tab の止まり (印・名前・画面の外)。同じ部品の行は 1 つの種類にまとめて数える。
- **時間:** 組 (テーマ × 言語 × 窓) ごとに別のページで開き、既定で 3 つの組を並行に流す (`--jobs N`)。エディタの 13 の組は 5 分半ほど。
  Tab のたどりで撮った画像は Node の側で読む (T0054)。混んだ機械では 3 つのアプリをまとめると 10 分を超えるので、上限のある所
  (エージェントが前面で流す Bash の 600 秒など) からは `--deadline <秒>` (起動から数えた締め切り。過ぎたら残りの組を飛ばして報告を書き、
  終了コード 1) を付ける。1 つの組は `--variant-timeout` (既定 480 秒) を過ぎたら場面の失敗にして次へ進む (T0884)。場面の中でダイアログ・メニュー・帯などが出るのを待つ上限は 1 つの `WAIT_TIMEOUT_MS` (20 秒。時間切れの文に何を待っていたかと最後に撮った場面を添える。T0985)。
- **結果:** `target/ui-audit/<アプリ>/` に PNG・アクセシビリティの木 (`.aria.yml`)・骨組み (`.outline.txt`)・Tab の順・報告 (`report.md`)、`target/ui-audit/summary.md` と
  `summary.json`。既定の終了コードは場面を開けなかったとき (時間の上限を過ぎた・締め切りで飛ばした組も) だけ 1 (`--strict` なら見つけたものがあっても 1)。
- **確かめないこと:** 実物の WebView (WebKitGTK は `just e2e` の側。T0919)・実際のスクリーンリーダー・ビューポートの中身・配置の良し悪し。
- **`just ui-shots` との関係:** `just ui-shots` は `@ugoe/ui` の部品の見本の画面 (packages/ui/README.md) を撮る。部品を変えたときはそれを、
  アプリの画面を変えたときは `just ui-audit` を使う。
- 場面の足し方・検査の作り・偽のホストは [tests/ui-audit/README.md](../tests/ui-audit/README.md)。

## 実機の e2e テスト

本物の Tauri のアプリを起動し、WebDriver で操作して主な流れを確かめます (T0006)。M8 のアプリはヘッドレスの Chrome とテストで
確かめていたため、WebView と Tauri の間の不具合 (T0812 のトラッキングの固まり、T0813 の出力ウィンドウのドラッグ) を実機で触るまで
見つけられなかったことから作りました。

```sh
just e2e                  # 3 つのアプリをビルドして流す (結果は target/e2e)
just e2e --only studio    # スタジオだけ
just e2e --no-build       # 前にビルドした実行ファイルで流し直す
just e2e --headed         # Xvfb を使わず、今の画面にウィンドウを出す
just e2e --update-images  # マニュアルの画像 (docs/manual/images) を本物の WebView で撮り直す (T0919)
```

- 道具: [tauri-driver](https://crates.io/crates/tauri-driver) **2.1.0** (`cargo install tauri-driver --version 2.1.0 --locked`。
  CI の `e2e.yml` の `TAURI_DRIVER_VERSION` と揃える)・WebKitWebDriver (Debian・Ubuntu の `webkit2gtk-driver`)・Xvfb・dbus-run-session・
  (あれば) gnome-keyring-daemon (無ければエディタの API キーの筋書きを飛ばす。CI では入れる)。
  WebDriver のクライアントは npm の依存を足さず、使う分だけの小さなもの (`tests/e2e/lib/webdriver.mjs`) を持ち、筋書きは node:test で書く
  (利用者の決定 2026-10-03)。
- 利用者の環境を汚さない: アプリの `XDG_CONFIG_HOME`・`XDG_DATA_HOME`・`XDG_CACHE_HOME`・`XDG_STATE_HOME` を `target/e2e/<アプリ>/xdg/` に向け、
  新しい D-Bus のセッションで動かす。エディタの API キーは、そのセッションで動かす使い捨ての鍵の保管庫に保存する (利用者の保管庫に触らない。T0050)。
  画面は既定で Xvfb (利用者の画面にウィンドウを出さない)。Xvfb の番号は空いているものを選ぶので、
  同じ機械で並行に流せる (T0022)。
- CSP: 各アプリの筋書きの最後に、CSP が止めたもの (`securitypolicyviolation` など) が 0 件であることを確かめる (T0022)。
  CSP が wasm や画像を止めても画面の一部が静かに動かなくなるだけなので、ほかの手順では気づけないため。数えない例外は持たない
  (MediaPipe の利用状況の送信は @ugoe/face-tracking が送る前に断る。T0943)。
- 筋書き: エディタ (見本のプロジェクトを開く・パラメータを動かす・編集して保存・PSD の取り込み・`.ugoe` の書き出し・プレビューの着せ替え・
  モジュールのプロジェクトの `.ugoemod` の書き出し・API キーの保存と削除 (鍵の保管庫)・偽の AI のプロバイダ (この計算機の OpenAI 互換のサーバ) での提案とそのプレビュー (見本の着せ替えの本体に取り込んだ PSD と、「New from PSD…」の新しいプロジェクト)・
  言語の切り替え)、ビューア (見本を開く・トラッキングの記録でパラメータが動く・設定)、スタジオ (見本のプロファイル・シーンの切り替え・
  出力ウィンドウ・重い検出の下のトラッキング (T0812 の回帰)・出力ウィンドウを隠しても止まらない・操作 API・ブラウザソース・
  プロファイルの自動保存と開き直し)。カメラは記録の再生で代え、重い検出は刻みの処理を 200ms 塞いで模す。3 つのアプリとも、
  見本のアバターの顔の画像でトラッキングし、本物の MediaPipe が読み込まれてパラメータが動くことも確かめる (T0023)。
- スタジオのトラッキングの待ちを前と後で比べる: `pacing.rs` を変えたときは、まず仮の時刻のテスト `late_values_spread_over_phases`
  (`cargo test -p ugoe-studio --lib late_values_spread_over_phases -- --nocapture`) の出力を前と後で並べる。値の届く時機と刻みの時機の
  ずれ (位相) を 32 通りに振った遅れた値の割合の最小・中央値・最大を出し、決まった値なので 1 回ずつで比べられる (中央値か最大が上限を
  超えると落ちる。T0878)。実機の e2e は、重い検出のトラッキングを既定で 13 秒流し、「トラッキングの待ち」の値が 12〜20 個しか集まらず
  遅れた値の割合が回ごとに 0〜28% と揺れる。実機でも比べるときだけ `UGOE_E2E_TRACKING_MS=60000 just e2e --only studio --no-build`
  のように長く流し (13000〜120000 ミリ秒。届く回数の下限は長さに比例する)、前と後で 3〜5 回ずつ `target/e2e/studio/tracking.json` の
  `trackingWaitSummary` を並べる。60 秒でも回ごとの差は残る (同じコードで 9〜32%。T0876)。1 回の値の数 (約 110 個) が少ないためで、
  1 回の中でトラッキングを始め直しても狭まらない (仮の時刻で確かめた。T0878) ので、1 回ずつでは比べない
  (手順は [tests/e2e/README.md](../tests/e2e/README.md#長く流す))。普段の `just e2e` と CI は既定の長さで流す。
- 失敗したとき: `target/e2e/<アプリ>/failures/<手順>/` にウィンドウごとと画面全体のスクリーンショット・ページの console、
  `target/e2e/<アプリ>/driver.log` にアプリの出力が残る。CI では成果物として残す。
- CI: `.github/workflows/e2e.yml` を `release-checks.yml` (手での起動・月に 2 回・リリースの前) が呼ぶ。単独でも手で起動できる
  (push・PR ごとの `ci.yml` には入れない。T0091)。
- **Windows と macOS は流さない。** macOS の WKWebView には WebDriver が無く、tauri-driver も対応していない。Windows は tauri-driver が
  Microsoft Edge Driver に中継して動くはずだが確かめていない。両方とも、リリースの前に手で確かめる一覧 [docs/manual-check.md](manual-check.md) で見る。
- 手順・偽のカメラと偽の AI のプロバイダの作り・筋書きの足し方は [tests/e2e/README.md](../tests/e2e/README.md)。

## モーションの途中の絵を確かめる

モーション・物理・表情の結果は、自動テスト (`crates/ugoe/tests/m3_puppet.rs`・`crates/ugoe-editor-core/tests/m3_workflow.rs`) が
パラメータと頂点の値で確かめます。見た目は次の手順で PNG にして目で確かめます (GPU が無ければ lavapipe / llvmpipe で動きます)。

```sh
# エディタで作ったモデル (物理・モーション・表情を含む .ugoe) の、モーション `Shake` の途中のコマ
just cli render model.ugoe -o shake-0.25.png --scale 2 --motion Shake --time 0.25
just cli render model.ugoe -o shake-0.50.png --scale 2 --motion Shake --time 0.5
# 同じ時刻を物理なしで描き、髪の遅れ (物理の効果) を見比べる
just cli render model.ugoe -o shake-0.25-still.png --scale 2 --motion Shake --time 0.25 --no-physics
# 表情だけ・物理だけ (頭を 30 にした直後から 0.2 秒揺らす)
just cli render model.ugoe -o smile.png --expression Smile --no-effects
just cli render model.ugoe -o swing.png --param AngleX=30 --time 0.2
```

- `--time` は `Puppet::update(1 / fps)` を繰り返して進めるので (既定 60 fps)、物理の揺れもその時刻のものになります。
  `--param` の値は基準値になり、モーションの対象のパラメータはモーションの値で上書きされます。
- 目パチは乱数のシードで決まるので同じ時刻なら同じ絵ですが、見比べるときは `--no-effects` で止めると分かりやすくなります。
- 連番のコマは、`--time` を変えて繰り返し呼びます (例: `for t in 0 0.1 0.2 0.3; do just cli render m.ugoe -o f-$t.png --motion Shake --time $t; done`)。
- 自動の描画テスト `crates/ugoe-cli/tests/m3_render_gpu.rs` (`just test-gpu`) も同じ手順で `shake-*.png` などを
  一時ディレクトリの `ugoe-m3-render/` に残します。

## AI 関連

- 実際の API は CI で呼びません。`FakeProvider` に録画した応答を読ませてテストします。
- 実際の API を使うテストは `#[ignore]` を付け、環境変数 `UGOE_AI_LIVE=1` のときだけ手動で実行します。
