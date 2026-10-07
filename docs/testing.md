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
| 参照の検査 | `scripts/check-doc-refs.mjs` (コードのコメントから `docs/`・`spec/` の見出しへの参照)・`scripts/check-ffi-bindings.mjs` (C ABI のヘッダと、Unity の宣言・fuzz の入口 `ugoe_ffi::fuzzing` の呼び出しの過不足) | `just refs` |
| CI のグループの検査 | `scripts/ci-groups.mjs` (すべてのクレートがどれか 1 つのグループに入っているか)・`scripts/check-lock-sync.mjs` (ワークスペースの外の `Cargo.lock` の依存の版がルートとそろうか。[fuzz](#fuzz)) | `just groups` ([CI のジョブ](#ci-のジョブ)) |
| 補助スクリプトの単体テスト | `scripts/*.test.mjs` のうち、ほかのレシピが流さないもの (mbx の判定・`--locked` で lock が古いときの文・npm のライセンスの例外・wasm の生成物の古さの判定・worktree の cargo の設定) | `just script-check` |
| API ドキュメントの検査 | rustdoc の警告 (壊れた intra-doc link、公開の文書から非公開の項目へのリンク、URL の書き方など) をエラーにして `cargo doc --workspace --no-deps`。wasm32 でだけコンパイルするコード (`ugoe-wasm` の `src/web.rs`・`src/web/`) は wasm32 のターゲットで `cargo doc -p ugoe-wasm` | `just doc-check` (CI はグループごとに `just doc-group <グループ>`)。wasm32 の分は `just wasm-check` |
| UI の監査 | `tests/ui-audit`。3 つのアプリのビルドした画面を偽のホストとヘッドレスの Chrome で開き、テーマ・言語・幅・devicePixelRatio を変えて撮り、コントラスト・はみ出し・文字と押せる部品の重なり・押せる大きさ・名前・Tab の順を数える | `just ui-audit` (Chrome が要る。重いので `just ci` に入れない。道具の単体テストは `just ui-audit-check`。[下](#ui-の監査)) |
| 実機の e2e テスト | `tests/e2e`。本物の Tauri のアプリ (エディタ・ビューア・スタジオ。WebKitGTK) を起動し、WebDriver で主な流れを操作する | `just e2e` (Linux だけ。tauri-driver・WebKitWebDriver・Xvfb が要る。[下](#実機の-e2e-テスト)) |
| 配布物の適合テスト | `tests/conformance` (`ugoe-conformance`)。C ABI・wasm (npm パッケージ。WebGL2 と WebGPU)・Unity (C# の層)・Godot の頂点と画像を参照と比べる。`<ugoe-avatar>` の作り直しのシナリオと、問い合わせの JSON の出力のキーと TS の型の一致も本物のブラウザで確かめる | `just conformance` (C コンパイラ・Chrome・GPU が要る。dotnet・Godot は無ければ飛ばす。[下](#配布物の適合テスト)) |
| PuppetSurface のブラウザのテスト | `packages/player/browser-test`。エディタと Player が共有する `PuppetSurface` と `Player` を本物の wasm とヘッドレスの Chrome の WebGL2 で動かし、作成・描画・WebGL2 の喪失からの作り直し・諦め・破棄・devicePixelRatio の追従を確かめる | `just player-browser` (Chrome と wasm-bindgen-cli が要る。CI の `conformance` ジョブで流す。[下](#puppetsurface-のブラウザのテスト)) |
| エディタのブラウザのテスト | `apps/editor/browser-test`。ビルドしたエディタの画面を `just ui-audit` の偽のホストと本物の wasm とヘッドレスの Chrome の WebGL2 で動かし、GPU の喪失のまわり (プレビューの着せ替えの読み込みの途中の喪失と、失っている間の付け外し) を確かめる | `just editor-browser` (Chrome・wasm-bindgen-cli・cargo が要る。CI の `conformance` ジョブで流す。[下](#エディタのブラウザのテスト)) |
| fuzz | `fuzz/` (cargo-fuzz、ワークスペースの外)。バンドル・モジュール・画像・JSON の読み込みと、読めたモデルの評価 | `just fuzz <target> [秒]` (nightly と cargo-fuzz が要る。`just ci` に入れず、CI は週に 1 度の `fuzz.yml`。[下](#fuzz)) |
| tier 3 のターゲットの build-std | `core`・`alloc` が配られない tier 3 のターゲット (`aarch64-nintendo-switch-freestanding` と、64 ビットのアトミック操作の無い `armv6k-nintendo-3ds`) で、nightly の `-Zbuild-std` で `ugoe-ffi` の `std` の無い静的ライブラリを作り、未解決のシンボルが決めた一覧と同じかを確かめる | `just nostd-build-std [ターゲット...]` (nightly と rust-src と llvm-tools が要る。`just ci` に入れず、CI は週に 1 度の `nostd-build-std.yml`。[下](#tier-3-のターゲットの-build-std)) |
| C ABI の検査 | ヘッダ `crates/ugoe-ffi/include/ugoe.h` が最新か、C の見本 `crates/ugoe-ffi/examples/c/sample.c` のビルドと実行。`std` の無い形の静的ライブラリをホストの C ライブラリとリンクした見本 (`examples/c/nostd.c`) の実行 | `just ffi-check` (cbindgen が要る。[crates/ugoe-ffi/README.md](../crates/ugoe-ffi/README.md#ヘッダの生成)) |

`just ci` はこれらを (GPU テスト・C ABI の検査・配布物の適合テスト・fuzz・tier 3 のターゲットの build-std 以外) すべて実行します。
C ABI の検査はワークスペースの外のツール (cbindgen) と C コンパイラが要るので `just ci` に入れず、CI の `rust (bindings)` ジョブで別に実行します ([CI のジョブ](#ci-のジョブ))。
配布物の適合テストも同じ理由で、CI の `conformance` ジョブで別に実行します (比べ合わせの単体テストは `just ci` に入る)。
PuppetSurface とエディタのブラウザのテストも wasm のビルドと Chrome が要るので `just ci` に入れず、同じ `conformance` ジョブで流します。
実機の e2e テストは 3 つのアプリをビルドし直すので、PR ごとの CI に入れず、別のワークフロー `e2e.yml` で流します ([下](#実機の-e2e-テスト))。
UI の監査も 3 つのアプリをビルドして 200 ほどの場面を撮るので `just ci` に入れず、画面を変える PR で人とエージェントが流します ([下](#ui-の監査))。

## wasm の生成物の古さ

エディタ・ビューア・スタジオの wasm の生成物 (`apps/editor/src/lib/viewport/generated`・`apps/viewer/src/lib/wasm/generated`・
`apps/studio/src/lib/wasm/generated`) は git の管理外で、`just wasm <out>` (各アプリの `pnpm wasm`) で作ります。pull や merge で
ugoe-wasm の API が変わっても作業ツリーには前の生成物が残るので、そのままだと svelte-check がコードの誤りでなく生成物の古さで落ちます
(T0062)。そこで:

- `just wasm` (`scripts/wasm.mjs build`) は生成物の隣に、入力の指紋 `ugoe_wasm.fingerprint` を書きます。入力は ugoe-wasm と、
  それが (dev 以外で) 依存するワークスペースのクレートの `Cargo.toml`・`build.rs`・`src/` の下のすべてのファイルと、
  ルートの `Cargo.toml`・`Cargo.lock` (wasm-bindgen の版を含む)・`rust-toolchain.toml` です。テスト・例・README は入れません。
- `just wasm-fresh` (`scripts/wasm.mjs fresh`) は、ビルドせずに今の指紋を求めて (100 ほどのファイルを読むだけで、数十ミリ秒) 3 つの生成物と比べ、
  古いものだけを作り直します。指紋の無い生成物 (指紋を書く前に作ったもの) も古いとみなします。生成物の無いアプリは飛ばします
  (型検査は `bindings/web/stub` を読む)。確かめるだけなら `node scripts/wasm.mjs check` (古ければ終了コード 1)。
- `just web-check` (`just ci`) と lefthook の pre-push の web-check は、先に `just wasm-fresh` を流します。作り直しは止めずに行います。
  ugoe-wasm の wasm32 の release ビルド (ほかの差分が無ければ手元で数秒〜数十秒。mbx の置き場に当たれば数秒) と wasm-bindgen だけで、
  pre-push の clippy と全テストより軽いためです。pre-push のように just を通らずに `scripts/wasm.mjs` を呼んでも、mbx の shim が入っていれば
  cargo は mbx を通ります (`exportMbxPath`。[docs/workflow/build-cache.md](workflow/build-cache.md))。
- 作り直すには wasm-bindgen-cli (`Cargo.lock` と同じ版) が要ります。無ければ入れ方と、代わりに古い生成物を消す (`rm -r <生成物>`。
  型検査はスタブを読む) コマンドを出して止まります。

判定の単体テストは `scripts/wasm.test.mjs` (`just script-check`) です。小さな偽のワークスペースで、指紋の書き換え・指紋の無い生成物・
入力の変更 (ソース・シェーダ・Cargo.toml・Cargo.lock・rust-toolchain.toml) を古いと見つけ、テスト・README・dev の依存だけのクレートの
変更と、生成物の無いアプリを飛ばすことを確かめます。

## CI のジョブ

`.github/workflows/ci.yml` の Rust の検査は、クレートのまとまり (グループ) ごとの別のジョブで行います。
ワークスペース全体のテストを 1 つのジョブでビルドすると、GitHub Actions のランナーのディスク (空きは約 14GB) を超えるためです
(デバッグ情報ありで約 17GB。T0002)。グループの定義の正は `scripts/ci-groups.mjs` の `GROUPS` で、CI の matrix もここから作ります。

| ジョブ | 内容 |
| --- | --- |
| `rust-checks` | ワークスペースをビルドしない検査: fmt・`just layers`・`just refs`・`just groups`・`just licenses`・`just publish-check`・`just tasks`・`just app-build-check`・`just script-check` |
| `rust (runtime)` | ランタイム層 (`ugoe-core` から `ugoe` まで) の clippy とテスト、参照レンダラを外した `ugoe` の clippy とテストと wgpu に依存しないことの検査 (`just sdk-no-render-check`。[portability.md](portability.md#ugoe-クレートの-feature))、`std` 無しのビルド (`just nostd-check`。`aarch64-unknown-none` と、64 ビットのアトミック操作の無い `thumbv7em-none-eabihf`。[portability.md](portability.md#ランタイム層が守る制約-移植性のため))。`ugoe-format`・`ugoe-ffi` の `std` の無い形の単体テスト (ホストで `--no-default-features`。下の「std の無い形の単体テスト」) と、`ugoe-ffi` の `std` の無い静的ライブラリを C の見本とリンクし、未解決のシンボルを調べる検査を含む ([crates/ugoe-ffi/README.md](../crates/ugoe-ffi/README.md#std-の無い環境))。fuzz のクレート (`fuzz/`) の stable での検査 (`just fuzz-lint`。[fuzz](#fuzz)) も |
| `rust (bindings)` | `ugoe-ffi`・`ugoe-wasm`・`ugoe-conformance` の clippy とテスト、wasm32 の clippy と rustdoc (`just wasm-check`)、C ABI (`just ffi-check`) |
| `rust (assets)` | `ugoe-psd`・`ugoe-mesher`・`ugoe-rigging`・`ugoe-headless` の clippy とテスト |
| `rust (tools)` | `ugoe-fs`・`ugoe-editor-core`・`ugoe-ai`・`ugoe-cli`・`ugoe-mcp` の clippy とテスト |
| `rust (apps)` | エディタ・ビューア・スタジオ (Tauri) の clippy とテスト、生成物の検査 (`just gen && git diff --exit-code`)。Tauri のシステムの依存はこのジョブでだけ入れる |
| `rust (cross)` | 手元の OS 以外のターゲットでの clippy (`just cross-check`)。下の「別のターゲットの検査」 |
| `rust (windows)`・`rust (macos)` | Windows と macOS のランナーでの、ファイルを扱うクレートの clippy とテスト (`just lint-os`・`just test-os`)。C ABI (`just ffi-check`。Windows は `std` がある形だけ)。macOS ではアプリの clippy (`just lint-group apps`) も。下の「Windows と macOS」 |
| `rust` | 上の全ジョブの結果をまとめる (ブランチの保護で必須にする検査の名前を、グループが増えても変えないため) |

- 手元では `just lint-group <グループ>` と `just test-group <グループ>` で、CI の 1 つのジョブと同じ clippy とテストを走らせられます。`just ci` は今までどおりワークスペース全体を検査します。
- `rust (<グループ>)` の各ジョブは、clippy とテストの後に `just doc-group <グループ>` で、そのグループのクレートの API ドキュメントを rustdoc の警告をエラーにして作ります (`just ci` の `just doc-check` のグループの分。T0027)。依存はテストのビルドと同じ check の成果物を使うので、ディスクはあまり増えません。角括弧 (`[0,1]²` など) は intra-doc link と読まれるので、バッククォートで囲むか `\[` と書きます。非公開の項目は公開の文書からリンクせず、バッククォートだけで書きます。
- `ugoe-cli` の bin は名前が `ugoe` で SDK (`ugoe` クレート) の lib と出力先 (`target/doc/ugoe/`) が同じなので、`doc = false` で API ドキュメントを作りません。cargo は rustdoc の前に `target/doc/<クレート>` を消すため、衝突したままだと `cargo doc --workspace` の中で 2 つの rustdoc が互いの出力を消し、`just doc-check` がまれに失敗し、`just doc` で開く `ugoe` のページが CLI のものになることがありました (T0033。[cargo#6313](https://github.com/rust-lang/cargo/issues/6313))。bin の名前を lib と同じにするときは同じく `doc = false` を付けます。
- `ugoe-wasm` の `src/web.rs`・`src/web/` (`WebPuppet` など) は `cfg(target_arch = "wasm32")` の中にあり、ホストの `doc-check` ではコンパイルされません。`just wasm-check` が wasm32 のターゲットで `cargo doc -p ugoe-wasm --no-deps` を rustdoc の警告をエラーにして流します (CI は `rust (bindings)`。T0033)。ホストの文書から wasm32 だけの項目へはリンクできない (ホストの `doc-check` で壊れたリンクになる) ので、バッククォートだけで書きます。
- `just script-check` (scripts/ の単体テスト) は Node だけで動くので、ワークスペースをビルドしない `rust-checks` のジョブで流します (T0027)。`just groups`・`just app-build-check` が流すテストは、それぞれのレシピの側に入っています。
- clippy (`just lint`・`just lint-group`) は、既定の feature と、すべての feature (`--all-features`) の 2 回流します。`gpu-tests` などの feature の下のコード (GPU のテストなど) は既定の feature ではコンパイルされず、警告が溜まっていたためです (T0211・T0013)。グループでの `--all-features` は、選んだクレートの feature だけを有効にします。
- `-p` で選んだクレートの間でだけ feature が統合されるので、ワークスペース全体では別のクレートが有効にする feature (例: `ugoe-editor-core` が有効にする `ugoe-model/schema`) は、グループの `features` に書きます。`required-features` を持つテストや例 (`gpu-tests` を除く) の feature が書かれていなければ `just groups` が失敗します。`just crate <クレート>` も、グループの `features` のうちそのクレートのもの (`ugoe-format/png` など) を有効にして clippy とテストを流します (T0186)。
- 各ジョブは、最初にランナーの使わない SDK (.NET・Android SDK・Haskell・CodeQL) を消してディスクを空けます。
- CI では `CARGO_PROFILE_DEV_DEBUG=0` でデバッグ情報を作りません (ci.yml のワークフロー全体の `env`)。target とキャッシュがおよそ半分になります。パニックの位置 (ファイルと行) はデバッグ情報がなくても表示されますが、バックトレースには行番号が出ません。
- 依存クレートの成果物は `Swatinem/rust-cache` で持ち越します。方針は下の「CI のキャッシュ」。
- nightly が要る検査は PR ごとの CI に入れず、PR を止めない別のワークフローで週に 1 度 (月曜) と手動 (`workflow_dispatch`) で流します:
  `fuzz.yml` ([fuzz](#fuzz)) と `nostd-build-std.yml` ([tier 3 のターゲットの build-std](#tier-3-のターゲットの-build-std))。

各グループの target の大きさの目安 (T0002 で手元で測った値。`CARGO_PROFILE_DEV_DEBUG=0` で clippy とテストのビルドの後):

| グループ | target |
| --- | --- |
| runtime | 約 0.8GB |
| bindings | 約 0.9GB (wasm32 の clippy を足して約 1.2GB) |
| assets | 約 0.7GB |
| tools | 約 1.4GB |
| apps | 約 2.7GB (`just gen` を足して約 2.8GB) |

### CI のキャッシュ

依存クレートの成果物は [`Swatinem/rust-cache`](https://github.com/Swatinem/rust-cache) で持ち越します。リポジトリのキャッシュは合計 10GB が上限で、
超えると使われていないものから消されます ([GitHub の文書](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching#usage-limits-and-eviction-policy))。
上限は引き上げず、キャッシュの数と大きさを抑えます (T0051)。

- **キー:** rust-cache のキーは `v0-rust-<shared-key>-<OS>-<アーキテクチャ>-<環境のハッシュ>-<Cargo.toml と Cargo.lock のハッシュ>` です。
  環境のハッシュは rustc の版と `CARGO`・`RUST`・`CC` などで始まる環境変数から作るので、キーを共有するジョブは環境変数を揃えます
  (`CARGO_PROFILE_DEV_DEBUG` を ci.yml のワークフロー全体に置くのはこのため)。最後のハッシュは `Cargo.toml` が変わるたびに変わり、
  そのときは最後のハッシュを除いた前方一致で、前のキャッシュから部分的に戻します。
- **main でだけ保存する:** `save-if: ${{ github.ref == 'refs/heads/main' }}`。PR やタグの run が保存したキャッシュは、その ref
  (と後の同じ ref の run) からしか読めず、容量を食うだけです ([GitHub の文書](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching#restrictions-for-accessing-a-cache))。
  PR とタグの run は main のキャッシュから始まります。
- **ビルドの重なるジョブはキーを共有し、保存は 1 つのジョブだけにする:** 同じキーで並んで動くジョブは、先に終わったものだけが保存します。
  そこで、ビルドがほかのジョブのビルドに含まれるジョブは、そのキーを `save-if: false` で読むだけにします。

  | 保存するジョブ (`shared-key`) | 読むだけのジョブ |
  | --- | --- |
  | `rust (tools)` (`group-tools`) | `gpu (vulkan)`・`gpu (gl)` (テストのビルドの依存が `rust (tools)` のテストのビルドに含まれることを手元で確かめた) |
  | `conformance` (`conformance`) | `web` と web-runtime-release.yml (`ugoe-wasm` の release と web-dist)、`godot` (参照の画像を描く `ugoe-cli`・`ugoe-conformance`・`ugoe-ffi`)、e2e.yml (`ugoe-wasm` の release) |
  | `rust (apps)` (`group-apps`) | e2e.yml (アプリの dev の依存。下の「e2e.yml は自分のキャッシュを持たない」) |

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
- **e2e.yml は自分のキャッシュを持たない:** rust-cache を 2 つ使い、`conformance` と `group-apps` のキーを `save-if: false` で
  読むだけにします (T0059)。`tests/e2e/run.mjs` は、アプリごとの `tauri build` ではなく、各アプリの beforeBuildCommand
  (`pnpm wasm && pnpm build`) の後に、選んだアプリ (と隣に置く `ugoe-mcp`) を 1 回の
  `cargo build --bins --features tauri/custom-protocol -p ugoe-editor -p ugoe-mcp -p ugoe-viewer -p ugoe-studio` でビルドします
  (引数は `tauri build` が中で流すものと同じ。[tests/e2e/README.md](../tests/e2e/README.md#流れ))。アプリをまとめると feature の統合が
  `rust (apps)` (4 つのクレートを 1 回でビルドする) と揃い、違うのは `tauri/custom-protocol` で変わる tauri・プラグインなどの依存
  17 個だけになります (手元で `rust (apps)` の手順の後に 26 秒。アプリを 1 つずつ `tauri build` していたときは約 170 個)。
  `pnpm wasm` の `ugoe-wasm` の wasm32 の release は `conformance` の `just player-browser` と同じコマンドなので、そのキャッシュに入っています。
  2 つの rust-cache は同じ `target/` に続けて戻し (同じ名前の成果物は同じ中身)、どちらの環境のハッシュにも 1 つ目の restore が置く
  `CARGO_INCREMENTAL=0` が入ります (ci.yml の 1 つの rust-cache の restore も、キーを作る前に置く) ので、キーは保存する側と揃います。
  `Cargo.toml` が変わった main への push では、e2e.yml は ci.yml と並んで動くので、新しいキーはまだ保存されておらず、
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
ubuntu では見つからないので、`rust (windows)` (`windows-latest`) と `rust (macos)` (`macos-latest`) のジョブが、毎回の PR と main への push で
次を流します (T0013)。

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
  `cargo rustc -p ugoe-ffi --lib --crate-type staticlib -- --print native-static-libs` の出力に合わせます。`std` の無い形は MSVC の
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

- nightly が要るので `just ci`・PR ごとの CI には入れず、`.github/workflows/nostd-build-std.yml` が週に 1 度と手動で流します。
  nightly や依存の変更で壊れても PR は止まりません。fuzz.yml とは失敗の意味と直し方が違うので、別のワークフローにしています。
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
- CI では `.github/workflows/fuzz.yml` が週に 1 度 (と手動で) ターゲットごとに 5 分回し、育てた入力 (`fuzz/corpus/`) をキャッシュで
  次の回へ持ち越します。見つかった入力はワークフローの成果物 (`fuzz-artifacts-<target>`) に上げます。
- PR ごとの CI (ci.yml の `rust (runtime)`) は、`just fuzz-lint` で fuzz のクレートを stable で検査します (依存のライセンス・整形・
  clippy。T0072)。ターゲットは `ugoe`・`ugoe-format` の公開 API と `ugoe-ffi` の `fuzzing` を使うので、変えて壊れたら週に 1 度のジョブを待たずに PR で気づきます。
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
- 計装したターゲットのビルド (nightly と cargo-fuzz) まで含めた検査は `just fuzz-check` で、CI では週に 1 度の fuzz.yml が流します。
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

## 描画テスト

- `ugoe-render` の描画テストは、オフスクリーンのテクスチャに描画して画素を読み戻し、期待する画像と比較します (許容誤差付き)。
- GPU を使うテストは各クレートの `gpu-tests` feature の裏に置きます。`just test-gpu` は `scripts/gpu-test-args.mjs` で `gpu-tests` feature を持つワークスペースのクレートを探し、まとめて `cargo test -p <crate> --features <crate>/gpu-tests` します。新しいクレートに描画テストを足すときは feature を定義するだけでよく、justfile や CI を直す必要はありません。
- アダプタが見つからないときは、飛ばさずに失敗させます (feature を明示的に有効にしたときだけ動くため)。
- 期待画像は `crates/ugoe-render/tests/golden/` に置きます。更新するときは PR に新旧の画像を載せます。
- `just test-gpu -- --nocapture` とすると、テストが選んだアダプタ (名前・ドライバ・バックエンド) が表示されます。
- テストは 1 つのデバイスをテストのスレッドの間で共有して並行に使うことが多いです。読み戻しで `Device::poll` が戻っても、`map_async` のコールバックはほかのスレッドの `poll` が集めたまま、まだ呼ばれていないことがあります (wgpu はロックを外してから呼ぶ)。待ち終わったらコールバックが呼ばれている、と決めつけず、呼ばれるまで上限のある回数で poll し直します (`ugoe-headless` の `Gpu::render` と、ugoe-render・ugoe-ai の描画テストの `tests/common/mod.rs` の `map_read` が例。T0963・T0964)。wgpu-core 30 はマップの状態をロックの中で変えるので、コールバックの前でも `get_mapped_range` は読めてしまいますが、それは文書にない実装の都合なので頼りません。手元で GL の揺れを確かめるときは `WGPU_BACKEND=gl LIBGL_ALWAYS_SOFTWARE=1` を付け、`-- --test-threads=16` などで並行を増やして何度か流します。

### CI での実行

GPU のない GitHub Actions の Linux ランナーでは、Mesa のソフトウェア実装で実行します (`.github/workflows/ci.yml` の `gpu` ジョブ)。バックエンドごとに matrix で 2 本走ります。

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
  作り直しでそうなれば待って試し直し、作り直しの回数に数える (`recreate-on-lost-context`。T0475)・作っている間の `detach` と次の `attach`・devicePixelRatio だけが変わったら (1 → 2 → 1.5 → 1)
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
  ポーズの欄 (`poseShown`) は本物の wasm の表示中の要素 (`pose_visible_json`) を読み、見本の `wave` で `HandsDown` → `Wave` → `HandsDown` と
  追従し、モーションが終わった後に選んだ要素は残る (`player-pose-follows-motion`。T0736。追従をやめると落ちることを確かめた)。
  コンテキストを失っている間 (`load_bundle`・`load_module` が `device_lost`) に初めのモデルを開いても、別のモデルとモジュールを開いても、
  `error`・`dressUpError` を出さず、復元の後にそのモデル (とモジュール) を開いて fit で描く (`player-loss-during-load`。T0955。
  直す前には落ちることを確かめた)。
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
- **時間:** 組 (テーマ × 言語 × 窓) ごとに別のページで開き、既定で 3 つの組を並行に流す (`--jobs N`)。エディタの 13 の組は 5 分半ほどで、
  エージェントが前面で流す Bash の上限 (600 秒) に収まる。Tab のたどりで撮った画像は Node の側で読む (T0054)。
- **結果:** `target/ui-audit/<アプリ>/` に PNG・アクセシビリティの木 (`.aria.yml`)・骨組み (`.outline.txt`)・Tab の順・報告 (`report.md`)、`target/ui-audit/summary.md` と
  `summary.json`。既定の終了コードは場面を開けなかったときだけ 1 (`--strict` なら見つけたものがあっても 1)。
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
- 失敗したとき: `target/e2e/<アプリ>/failures/<手順>/` にウィンドウごとと画面全体のスクリーンショット・ページの console、
  `target/e2e/<アプリ>/driver.log` にアプリの出力が残る。CI では成果物として残す。
- CI: `.github/workflows/e2e.yml` が main への push・毎晩・手動で流す (PR ごとの `ci.yml` には入れない)。
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
