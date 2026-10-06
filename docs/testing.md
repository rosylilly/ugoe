# テスト方針

## 種類

| 種類 | 場所 | 実行 |
| --- | --- | --- |
| ユニットテスト | 各クレートの `#[cfg(test)] mod tests` | `just test` |
| 結合テスト | 各クレートの `tests/` | `just test` |
| スナップショットテスト | `insta` (`tests/snapshots/`) | `just test`。更新は `cargo insta review` |
| 互換性テスト | `crates/ugoe-format/tests/compat.rs` と資料 `assets/compat/` | `just test`。資料の作り直しは [互換性テスト](#互換性テスト) |
| 描画テスト (GPU) | `gpu-tests` feature を持つクレート (`ugoe-render` など) | `just test-gpu` |
| フロントエンドの検査 | Biome + svelte-check + Vitest (`apps/*/src/**/*.test.ts`・`packages/*/src/**/*.test.ts`。部品を描くテストは [下](#エディタの部品のテスト)) | `just web-check` |
| wasm の型の写しの検査 | `scripts/check-wasm-types.mjs`。手書きの型の写し (`bindings/web/stub/ugoe_wasm.d.ts`・`@ugoe/player` と `@ugoe/face-tracking` の interface) を `just wasm` の生成物と比べる | `just wasm-types` (生成物が無ければ一部の写しをスタブと比べる。CI の web ジョブは `just wasm` の後に生成物と比べる) |
| 依存方向の検査 | `scripts/check-layers.mjs` | `just layers` |
| 参照の検査 | `scripts/check-doc-refs.mjs` (コードのコメントから `docs/`・`spec/` の見出しへの参照)・`scripts/check-ffi-bindings.mjs` (C ABI のヘッダと Unity の宣言の過不足) | `just refs` |
| CI のグループの検査 | `scripts/ci-groups.mjs` (すべてのクレートがどれか 1 つのグループに入っているか) | `just groups` ([CI のジョブ](#ci-のジョブ)) |
| 補助スクリプトの単体テスト | `scripts/*.test.mjs` のうち、ほかのレシピが流さないもの (mbx の判定・npm のライセンスの例外・worktree の cargo の設定) | `just script-check` |
| API ドキュメントの検査 | rustdoc の警告 (壊れた intra-doc link、公開の文書から非公開の項目へのリンク、URL の書き方など) をエラーにして `cargo doc --workspace --no-deps`。wasm32 でだけコンパイルするコード (`ugoe-wasm` の `src/web.rs`・`src/web/`) は wasm32 のターゲットで `cargo doc -p ugoe-wasm` | `just doc-check` (CI はグループごとに `just doc-group <グループ>`)。wasm32 の分は `just wasm-check` |
| UI の監査 | `tests/ui-audit`。3 つのアプリのビルドした画面を偽のホストとヘッドレスの Chrome で開き、テーマ・言語・幅・devicePixelRatio を変えて撮り、コントラスト・はみ出し・文字の重なり・押せる大きさ・名前・Tab の順を数える | `just ui-audit` (Chrome が要る。重いので `just ci` に入れない。道具の単体テストは `just ui-audit-check`。[下](#ui-の監査)) |
| 実機の e2e テスト | `tests/e2e`。本物の Tauri のアプリ (エディタ・ビューア・スタジオ。WebKitGTK) を起動し、WebDriver で主な流れを操作する | `just e2e` (Linux だけ。tauri-driver・WebKitWebDriver・Xvfb が要る。[下](#実機の-e2e-テスト)) |
| 配布物の適合テスト | `tests/conformance` (`ugoe-conformance`)。C ABI・wasm (npm パッケージ。WebGL2 と WebGPU)・Unity (C# の層)・Godot の頂点と画像を参照と比べる。`<ugoe-avatar>` の作り直しのシナリオと、問い合わせの JSON の出力のキーと TS の型の一致も本物のブラウザで確かめる | `just conformance` (C コンパイラ・Chrome・GPU が要る。dotnet・Godot は無ければ飛ばす。[下](#配布物の適合テスト)) |
| PuppetSurface のブラウザのテスト | `packages/player/browser-test`。エディタと Player が共有する `PuppetSurface` と `Player` を本物の wasm とヘッドレスの Chrome の WebGL2 で動かし、作成・描画・WebGL2 の喪失からの作り直し・諦め・破棄・devicePixelRatio の追従を確かめる | `just player-browser` (Chrome と wasm-bindgen-cli が要る。CI の `conformance` ジョブで流す。[下](#puppetsurface-のブラウザのテスト)) |
| C ABI の検査 | ヘッダ `crates/ugoe-ffi/include/ugoe.h` が最新か、C の見本 `crates/ugoe-ffi/examples/c/sample.c` のビルドと実行 | `just ffi-check` (cbindgen が要る。[crates/ugoe-ffi/README.md](../crates/ugoe-ffi/README.md#ヘッダの生成)) |

`just ci` はこれらを (GPU テスト・C ABI の検査・配布物の適合テスト以外) すべて実行します。
C ABI の検査はワークスペースの外のツール (cbindgen) と C コンパイラが要るので `just ci` に入れず、CI の `rust (bindings)` ジョブで別に実行します ([CI のジョブ](#ci-のジョブ))。
配布物の適合テストも同じ理由で、CI の `conformance` ジョブで別に実行します (比べ合わせの単体テストは `just ci` に入る)。
PuppetSurface のブラウザのテストも wasm のビルドと Chrome が要るので `just ci` に入れず、同じ `conformance` ジョブで流します。
実機の e2e テストは 3 つのアプリをビルドし直すので、PR ごとの CI に入れず、別のワークフロー `e2e.yml` で流します ([下](#実機の-e2e-テスト))。
UI の監査も 3 つのアプリをビルドして 200 ほどの場面を撮るので `just ci` に入れず、画面を変える PR で人とエージェントが流します ([下](#ui-の監査))。

## CI のジョブ

`.github/workflows/ci.yml` の Rust の検査は、クレートのまとまり (グループ) ごとの別のジョブで行います。
ワークスペース全体のテストを 1 つのジョブでビルドすると、GitHub Actions のランナーのディスク (空きは約 14GB) を超えるためです
(デバッグ情報ありで約 17GB。T0002)。グループの定義の正は `scripts/ci-groups.mjs` の `GROUPS` で、CI の matrix もここから作ります。

| ジョブ | 内容 |
| --- | --- |
| `rust-checks` | ワークスペースをビルドしない検査: fmt・`just layers`・`just refs`・`just groups`・`just licenses`・`just publish-check`・`just tasks`・`just app-build-check`・`just script-check` |
| `rust (runtime)` | ランタイム層 (`ugoe-core` から `ugoe` まで) の clippy とテスト、参照レンダラを外した `ugoe` の clippy とテストと wgpu に依存しないことの検査 (`just sdk-no-render-check`。[portability.md](portability.md#ugoe-クレートの-feature)) |
| `rust (bindings)` | `ugoe-ffi`・`ugoe-wasm`・`ugoe-conformance` の clippy とテスト、wasm32 の clippy と rustdoc (`just wasm-check`)、C ABI (`just ffi-check`) |
| `rust (assets)` | `ugoe-psd`・`ugoe-mesher`・`ugoe-rigging`・`ugoe-headless` の clippy とテスト |
| `rust (tools)` | `ugoe-fs`・`ugoe-editor-core`・`ugoe-ai`・`ugoe-cli`・`ugoe-mcp` の clippy とテスト |
| `rust (apps)` | エディタ・ビューア・スタジオ (Tauri) の clippy とテスト、生成物の検査 (`just gen && git diff --exit-code`)。Tauri のシステムの依存はこのジョブでだけ入れる |
| `rust (cross)` | 手元の OS 以外のターゲットでの clippy (`just cross-check`)。下の「別のターゲットの検査」 |
| `rust (windows)`・`rust (macos)` | Windows と macOS のランナーでの、ファイルを扱うクレートの clippy とテスト (`just lint-os`・`just test-os`)。macOS ではアプリの clippy (`just lint-group apps`) も。下の「Windows と macOS」 |
| `rust` | 上の全ジョブの結果をまとめる (ブランチの保護で必須にする検査の名前を、グループが増えても変えないため) |

- 手元では `just lint-group <グループ>` と `just test-group <グループ>` で、CI の 1 つのジョブと同じ clippy とテストを走らせられます。`just ci` は今までどおりワークスペース全体を検査します。
- `rust (<グループ>)` の各ジョブは、clippy とテストの後に `just doc-group <グループ>` で、そのグループのクレートの API ドキュメントを rustdoc の警告をエラーにして作ります (`just ci` の `just doc-check` のグループの分。T0027)。依存はテストのビルドと同じ check の成果物を使うので、ディスクはあまり増えません。角括弧 (`[0,1]²` など) は intra-doc link と読まれるので、バッククォートで囲むか `\[` と書きます。非公開の項目は公開の文書からリンクせず、バッククォートだけで書きます。
- `ugoe-cli` の bin は名前が `ugoe` で SDK (`ugoe` クレート) の lib と出力先 (`target/doc/ugoe/`) が同じなので、`doc = false` で API ドキュメントを作りません。cargo は rustdoc の前に `target/doc/<クレート>` を消すため、衝突したままだと `cargo doc --workspace` の中で 2 つの rustdoc が互いの出力を消し、`just doc-check` がまれに失敗し、`just doc` で開く `ugoe` のページが CLI のものになることがありました (T0033。[cargo#6313](https://github.com/rust-lang/cargo/issues/6313))。bin の名前を lib と同じにするときは同じく `doc = false` を付けます。
- `ugoe-wasm` の `src/web.rs`・`src/web/` (`WebPuppet` など) は `cfg(target_arch = "wasm32")` の中にあり、ホストの `doc-check` ではコンパイルされません。`just wasm-check` が wasm32 のターゲットで `cargo doc -p ugoe-wasm --no-deps` を rustdoc の警告をエラーにして流します (CI は `rust (bindings)`。T0033)。ホストの文書から wasm32 だけの項目へはリンクできない (ホストの `doc-check` で壊れたリンクになる) ので、バッククォートだけで書きます。
- `just script-check` (scripts/ の単体テスト) は Node だけで動くので、ワークスペースをビルドしない `rust-checks` のジョブで流します (T0027)。`just groups`・`just app-build-check` が流すテストは、それぞれのレシピの側に入っています。
- clippy (`just lint`・`just lint-group`) は、既定の feature と、すべての feature (`--all-features`) の 2 回流します。`gpu-tests` などの feature の下のコード (GPU のテストなど) は既定の feature ではコンパイルされず、警告が溜まっていたためです (T0211・T0013)。グループでの `--all-features` は、選んだクレートの feature だけを有効にします。
- `-p` で選んだクレートの間でだけ feature が統合されるので、ワークスペース全体では別のクレートが有効にする feature (例: `ugoe-editor-core` が有効にする `ugoe-model/schema`) は、グループの `features` に書きます。`required-features` を持つテストや例 (`gpu-tests` を除く) の feature が書かれていなければ `just groups` が失敗します。
- 各ジョブは、最初にランナーの使わない SDK (.NET・Android SDK・Haskell・CodeQL) を消してディスクを空けます。
- CI では `CARGO_PROFILE_DEV_DEBUG=0` でデバッグ情報を作りません。target がおよそ半分になります。パニックの位置 (ファイルと行) はデバッグ情報がなくても表示されますが、バックトレースには行番号が出ません。
- `Swatinem/rust-cache` の key はグループごとに分け、キャッシュは main でだけ保存します (PR は main のキャッシュから始まる)。

各グループの target の大きさの目安 (T0002 で手元で測った値。`CARGO_PROFILE_DEV_DEBUG=0` で clippy とテストのビルドの後):

| グループ | target |
| --- | --- |
| runtime | 約 0.8GB |
| bindings | 約 0.9GB (wasm32 の clippy を足して約 1.2GB) |
| assets | 約 0.7GB |
| tools | 約 1.4GB |
| apps | 約 2.7GB (`just gen` を足して約 2.8GB) |

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
| `tools` | `ugoe-ai`・`ugoe-mcp` | `x86_64-pc-windows-gnu` | MinGW の gcc (`gcc-mingw-w64-x86-64`。依存の ring の C) |

アプリ (`apps`) は Tauri と `ugoe-ai` の依存の C (ring) と、tauri-build のリソース (`.rc`) をターゲット向けにコンパイルするので、
ターゲットの標準ライブラリだけでは検査できません (T0013)。`tools` (`ugoe-ai`・`ugoe-mcp`。T0027) も ring の C をコンパイルするので、同じく `windows-gnu` で検査します。

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
- ファイルを扱うクレートを足したら `OS_CRATES` に足します。GPU も Tauri のシステムの依存も入れないので、それらが要るクレートは入れません。
- 手元の Linux で Windows のテストを近似するには、MinGW と Wine で `x86_64-pc-windows-gnu` のテストを流せます (CI の代わりにはならない。
  Wine のソケットやファイルシステムは本物の Windows と細部が違う):

  ```sh
  rustup target add x86_64-pc-windows-gnu   # MinGW (gcc-mingw-w64-x86-64) と wine も要る
  WINEPREFIX=/tmp/ugoe-wine WINEDLLOVERRIDES="mscoree,mshtml=" WINEDEBUG=-all DISPLAY= \
    CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUNNER=wine \
    cargo test -p ugoe-format -p ugoe-fs -p ugoe-editor-core -p ugoe-cli -p ugoe-mcp --target x86_64-pc-windows-gnu
  ```

  `WINEDLLOVERRIDES` は、新しい Wine の環境を作るときの Mono と Gecko の導入 (画面が無いと止まる) を飛ばすためです。

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
- 各版のディレクトリには、モジュールを取り付ける本体 `dress-up-base.ugoe` を必ず置きます (`.ugoemod` はこれに `compose` して評価する)。
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
  `gAMA` などのチャンク・インターレース・サンプルの値) を一覧の `png` に書き、規則どおりに読んだ画素を `rgba` に持ちます。
  参照実装の読み込み (`read_bundle_with_report` / `read_module_with_report`) が一覧のとおりに読むことを
  `crates/ugoe-format/tests/spec_bundles.rs` で確かめます。食い違う例は理由を付けてテストの `KNOWN_MISMATCHES` に挙げます
  (例のファイルを参照実装に合わせて作り直してはいけません)。
- 描画の例 (`spec/examples/render-*/`、公開仕様の [Rendering examples](../spec/examples/README.md#rendering-examples)) は、`model.json` と、
  テクスチャの画素・描画先の大きさ・ビュー変換 (省略するとキャンバスを描画先に引き伸ばす)・光源・調べる画素の期待値
  (乗算済み、0〜255 の尺度) を持つ `render.json` の組です。
  調べる画素の中心は三角形の辺から描画先の画素で 1 以上離し、テクスチャは 1 色を基本にして、GPU によるラスタライズとフィルタの違いに
  頼らないようにしています。許容誤差は各チャンネル 3 (rendering.md の Conformance と同じ) で、調べる画素はすべて満たします。
  期待値は仕様の式から画素ごとに導いた値で、`tools/spec-check` の CPU のラスタライザ (仕様だけから書いたもの) と一致します。
  使える光源が 8 個を超えるケース (レンダラは 8 個だけを使ってよいので互換の基準の外) は、どの 8 個を使っても、すべてを使っても
  調べる画素が同じになるように選んでいます。
  参照実装の描画が満たすことは GPU を使うテスト (`just test-gpu`、`crates/ugoe-render/tests/spec_render.rs`) で確かめます。
  食い違う画素は理由を付けてテストの `KNOWN_MISMATCHES` に挙げます。
- 時間の例 (`timeline.json` を持つもの、公開仕様の [Timeline examples](../spec/examples/README.md#timeline-examples)) は、フレームごとの
  ホストの操作と `dt` と、そのフレームの後のパラメータの値・パーツの不透明度の上書き・イベント・動的物理の加算量・評価の結果の期待値の列です。
  操作はパラメータの設定、モーションの開始と停止、表情の設定・追加・停止、リップシンクの音量、トラッキングの入力 (`null` で外す)、
  ポーズの要素の表示 (`show_pose`)、エフェクト設定の差し替え (`set_effects`)、物理のリセット、段の有効無効です。
  参照実装の `Puppet` が満たすことを `crates/ugoe/tests/spec_timelines.rs` で確かめ、食い違う例は理由を付けて `KNOWN_MISMATCHES` に挙げます。
- 互換性テストの資料 (`assets/compat/`) との違い: あちらはリリースごとのバイナリのファイルで、古い版のファイルが読めることと、
  知らない機能を劣化させて読めることを確かめる (評価の期待値は持たない)。こちらは仕様の一部 (CC BY 4.0) で、
  評価の規則とファイルの中身の並べ方を例で示し、他の実装が自分を確かめるためのもの。

## ランタイムの数値テスト

- 変形の結果は、許容誤差を付けて比較します (目安は `1e-4` px)。
- 仕様書の式に対応するテストには、テスト名かコメントで仕様書の節を示します。
- 評価の出力全体を `insta` の JSON スナップショットにしてよいですが、スナップショットを更新するときは、PR 本文に「なぜ値が変わったのか」を書きます。

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
- **エディタの `ViewportController` は対象にしていません (T0939 で決めた):** `PuppetSurface` を通る経路 (作り直しで表示の設定とモデルを渡し直す・
  諦めたら状態と文を出す・倍率の変化で描き直しを頼む・CSS ピクセルと描画バッファの画素の換算) は Player と同じ形で薄く、本物の wasm で
  確かめたい部分 (作り直し・倍率・換算) は上の `PuppetSurface` と `Player` のシナリオで通ります。順序は偽の wasm の
  `apps/editor/src/lib/viewport/controller.test.ts` が確かめます。ブラウザで動かすには、`@ugoe/player` のテストがアプリ (`apps/editor`) の
  コードを読むか (パッケージからアプリへ向きが逆になる)、エディタの側に同じ仕組みを作り直し、エディタの wasm の読み込み
  (`import.meta.glob` の `generated/`) と文言のカタログを差し替える必要があり、得られる確かさに比べて手間が大きいためです。
  T0938 で足した「作り直す」ボタン (`ViewportController.restart`) も、状態を変えて `PuppetSurface.restart` を呼ぶだけなので、本物の wasm の部分は
  `restart-after-giving-up` で通り、エディタの側には足していません (T0038)。エディタが作り直しのまわりに本物の wasm に頼る自分の処理を持ったら、足すことを考えます。
- **置き場所:** `packages/player/browser-test/` (GPL-3.0-only)。テストは GPL の `surface.ts`・`player.svelte.ts` を読むので、MIT の
  `tests/conformance` (と、ライセンスを書いていないのでリポジトリの既定の MIT になる `tests/ui-audit`) には置きません。Chrome の起動は
  `just ui-audit` と同じ playwright-core (ルートの devDependencies)。`@ugoe/player` を Vite (アプリと同じ版を `@ugoe/player` の
  devDependencies に入れた) と Svelte のプラグインで 1 つの ES モジュールにまとめて配ります (runes と依存を含むため)。
- **CI:** `conformance` ジョブ (Chrome と wasm-bindgen-cli が入っている) で、適合テストの後に流します。落ちたときはページのコンソールを
  `target/player-browser/page.log` に残します。
- WebGPU のデバイスの喪失は、ヘッドレスの Chrome で WebGPU を有効にしていないので確かめていません (偽の wasm のテストだけ。
  `Browser.crashGpuProcess` で起こせる見込みはあるが試していない)。本物の GPU (ドライバのリセットなど) での喪失は docs/manual-check.md の D09。

## エディタの部品のテスト

エディタ (`apps/editor`) の Vitest は既定でノードの環境で動き、純粋な関数と `.svelte.ts` の状態を確かめます。
ポインタやキーの操作から送るコマンドまでの流れ (ビューポートのツールなど) は、部品を描いて確かめます (T0699)。

- **環境:** ファイルの先頭に `// @vitest-environment happy-dom` を書き、そのファイルだけ happy-dom (ノードの DOM の実装) で動かします。
  テストのときは Svelte のブラウザ向けの実装を読みます (`vite.config.ts` の `resolve.conditions`。`packages/ui` の部品のテストと同じ)。
- **描く:** svelte の `mount` で body に描き、テストの後で片付けます。ビューポートのツールの道具は
  `apps/editor/src/lib/viewport/tools/test/harness.svelte.ts` にあります (`render`・`pointer`・`drag`・`press`・`dialogButton`)。
- **偽物:** `ViewportController`・`EditorDocument`・`WebPuppet` は偽物 (`FakeController`・`FakeDocument`・`fakePuppet`) を渡します。
  `FakeDocument.apply` は送られたコマンドを `commands` に記録するので、コマンドの列をそのまま比べます。`pending` に約束を入れると、
  送っている間の操作を試せます。バックエンドの座標変換 (`ipc.localToCanvas`・`ipc.canvasToLocal`) は `vi.mock` で恒等写像にします。
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
  (CSS の窓の幅が半分)。それぞれで空の状態・見本を開いた画面・下まで送った画面・設定・メニュー・エディタの右の列のタブなどを撮る。
  どれも `prefers-reduced-motion: reduce` で開く。
- **検査:** 文字のコントラスト・横のはみ出し・文字の重なりと切れ・24px 未満の押せるもの・名前の無い部品 (axe-core)・そのほかの ARIA の規則
  (axe-core)・reduced motion の下の動き・画面に出たエラー・Tab の止まり (印・名前・画面の外)。同じ部品の行は 1 つの種類にまとめて数える。
- **結果:** `target/ui-audit/<アプリ>/` に PNG・アクセシビリティの木 (`.aria.yml`)・Tab の順・報告 (`report.md`)、`target/ui-audit/summary.md` と
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
```

- 道具: [tauri-driver](https://crates.io/crates/tauri-driver) **2.1.0** (`cargo install tauri-driver --version 2.1.0 --locked`。
  CI の `e2e.yml` の `TAURI_DRIVER_VERSION` と揃える)・WebKitWebDriver (Debian・Ubuntu の `webkit2gtk-driver`)・Xvfb・dbus-run-session。
  WebDriver のクライアントは npm の依存を足さず、使う分だけの小さなもの (`tests/e2e/lib/webdriver.mjs`) を持ち、筋書きは node:test で書く
  (利用者の決定 2026-10-03)。
- 利用者の環境を汚さない: アプリの `XDG_CONFIG_HOME`・`XDG_DATA_HOME`・`XDG_CACHE_HOME`・`XDG_STATE_HOME` を `target/e2e/<アプリ>/xdg/` に向け、
  新しい D-Bus のセッションで動かす。画面は既定で Xvfb (利用者の画面にウィンドウを出さない)。Xvfb の番号は空いているものを選ぶので、
  同じ機械で並行に流せる (T0022)。
- CSP: 各アプリの筋書きの最後に、CSP が止めたもの (`securitypolicyviolation` など) が 0 件であることを確かめる (T0022)。
  CSP が wasm や画像を止めても画面の一部が静かに動かなくなるだけなので、ほかの手順では気づけないため。数えない例外は持たない
  (MediaPipe の利用状況の送信は @ugoe/face-tracking が送る前に断る。T0943)。
- 筋書き: エディタ (見本のプロジェクトを開く・パラメータを動かす・編集して保存・PSD の取り込み・`.ugoe` の書き出し・プレビューの着せ替え・
  モジュールのプロジェクトの `.ugoemod` の書き出し・偽の AI のプロバイダ (この計算機の OpenAI 互換のサーバ) での提案とそのプレビュー (見本の着せ替えの本体に取り込んだ PSD と、「New from PSD…」の新しいプロジェクト)・
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
