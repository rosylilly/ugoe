# apps/editor

ugoe のエディタ (Tauri v2 + Svelte 5)。設計は [docs/editor.md](../../docs/editor.md)。

```
src/              フロントエンド (Svelte 5 + TS)
  lib/ipc.ts        バックエンドとの通信はすべてここを通す
  lib/preview/      プレビューモード。フェイストラッキングは共有の @ugoe/face-tracking (packages/face-tracking/)
public/third-party-notices.txt 同梱する第三者のもの (MediaPipe・JS のライブラリ) のライセンスの表記。プレビューの「Third-party licenses」で表示する。
                  `node scripts/third-party-notices.mjs` が生成する ([docs/release.md](../../docs/release.md#同梱物のライセンスの表記))
public/mediapipe/ MediaPipe の wasm とモデル。`pnpm mediapipe` (dev・build の前にも) が用意する (gitignore 済み)
src-tauri/        バックエンド (Rust クレート `ugoe-editor`)
  src/commands.rs   IPC コマンド。薄く保ち、編集ロジックは ugoe-editor-core に置く
  src/tracking.rs   フェイストラッキングのカメラの許可と記録の保存。Info.plist・Entitlements.plist は macOS のカメラ
  src/mcp.rs        エディタの中の MCP サーバ (エージェントをつなぐ。docs/editor.md#エージェントをつなぐ)
  src/acl.rs        コマンドの名前 (build.rs が読み、コマンドごとの許可を作る)。capabilities/ と CSP の検査 (docs/editor.md#webview-の-csp-とコマンドの許可)
  tauri.bundle.conf.json  インストーラを作るときに重ねる設定 (ugoe-mcp のサイドカー)
scripts/sidecar.mjs 同梱する ugoe-mcp をビルドして src-tauri/binaries/ に置く (`pnpm sidecar`)
```

## 守ること

- 編集ロジックを書かない。ユーザーの入力を `Command` に変換して `apply_command` を呼ぶ。
- IPC コマンドを変えたら、同じ PR で `src/lib/ipc.ts` を更新する。コマンドを足したら `src-tauri/src/acl.rs` の `COMMANDS` と
  `src-tauri/capabilities/default.json` にも書く (書かないと呼べない。テストが検査する)。
- Svelte 5 の runes を使う。状態管理のライブラリは入れない。

## コマンド

```sh
just editor         # 開発モードで起動 (vite + tauri dev)
just editor-build   # リリースビルド (インストーラを作る。`just app::build TARGET=editor` の別名)
just web-check      # Biome + svelte-check
just editor-browser # ビルドした画面を本物の wasm とヘッドレスの Chrome で動かし、GPU の喪失のまわりを確かめる (browser-test/README.md)
```

`just editor` と `just editor-build` は、先に `just wasm` (ugoe-wasm のビルドと JS バインディングの生成) を実行する
(`tauri.conf.json` の `beforeDevCommand` / `beforeBuildCommand`)。`just web-check` と pre-push は、ugoe-wasm が変わって
生成物が古くなっていれば先に作り直す (`just wasm-fresh`。[docs/testing.md](../../docs/testing.md#wasm-の生成物の古さ))。

`just editor-build` (`just app::build TARGET=editor`) は `--config src-tauri/tauri.bundle.conf.json` を重ね、MCP サーバの
`ugoe-mcp` をサイドカー (Tauri の `externalBin`) として同梱する。重ねた設定の `beforeBuildCommand` が `pnpm sidecar`
(`scripts/sidecar.mjs`) でビルドのターゲット向けに `ugoe-mcp` をリリースでビルドし、`src-tauri/binaries/ugoe-mcp-<三つ組>[.exe]`
(gitignore 済み) に置く。インストールすると `ugoe-mcp` はエディタの実行ファイルの隣に入り、設定の「Connect agents」が
エージェントの登録のコマンドにそのパスを埋める。`tauri.conf.json` に `externalBin` を書かないのは、tauri-build がファイルの
無い `externalBin` で `cargo check` と `tauri dev` も失敗させるため。`just editor` (開発中) は同梱しないので、画面は PATH の
`ugoe-mcp` を使うコマンドを出す (`cargo build --release -p ugoe-mcp` で作れる)。

## 各 OS でのビルド

エディタ・ビューア・スタジオのインストーラは、1 つの入口 `just app::build` で作る (`scripts/app-build.mjs`)。
ここに書く前提と手順は 3 つのアプリで同じ。

```sh
just app::build                                        # すべてのアプリを今の OS 向けに
just app::build TARGET=editor                          # エディタだけ (`just editor-build` と同じ)
just app::build TARGET=editor,viewer PLATFORM=linux,windows   # Linux (WSL2) から Linux 向けと Windows 向けを
just app::build TARGET=studio --bundles deb            # KEY=値 でない引数は tauri build に渡す
```

| 指定 | 意味 |
| --- | --- |
| `TARGET=` | アプリ。`editor`・`viewer`・`studio` をカンマ区切りで (`all` か省略ですべて) |
| `PLATFORM=` | 向ける OS。`windows`・`macos`・`linux` をカンマ区切りで (省略で今の OS) |
| `VCAM=1` | macOS のスタジオに仮想カメラの Camera Extension を入れる ([docs/release.md#macos-の仮想カメラ](../../docs/release.md#macos-の仮想カメラ)。署名が要る) |
| `DRY_RUN=1` | 道具を調べ、実行するコマンドを表示するだけ |
| (環境変数) | 上の指定は環境変数でも書ける (`DRY_RUN=1 just app::build ...`)。引数に書いた方が優先 |
| それ以外 | すべての `tauri build` にそのまま渡す (例: `--debug`・`--bundles deb`・`--target aarch64-apple-darwin`) |

- **書き方:** just のレシピの名前には `:` を使えないので、`app:build` ではなくモジュール (`justfile` の `mod app`、`scripts/app.just`) の
  `app::build` にしている (`just app build` でも同じ)。`TARGET=...` は just の変数の上書きではなく、レシピの引数としてスクリプトが読む
  (just の変数の上書きはモジュールのレシピに届かないため)。そのため `just app::build` の **後ろ**に書く。
- **作れる組み合わせ:** 今の OS 向けはその OS の `tauri build`。それに加えて、Linux (WSL2 を含む) からは Windows 向けを
  cargo-xwin でクロスビルドできる ([Linux (WSL2) から Windows 向けにビルドする](#linux-wsl2-から-windows-向けにビルドする)。NSIS だけ)。
  それ以外 (Linux から macOS 向け・Windows から Linux 向けなど) は、何も作らずに理由と代わりの方法 (その OS の上で実行する・
  CI の [Editor build](../../.github/workflows/editor-build.yml)) を示して止まる。足りない道具があれば、入れ方を示して止まる。
- **アプリごとの違い** は `scripts/app-build.mjs` の `APPS` だけに書く: エディタは MCP サーバのサイドカー (`tauri.bundle.conf.json`)、
  スタジオは Windows 向けで仮想カメラの DLL と NSIS のフック (`tauri.windows.conf.json`。Tauri がターゲットの OS を見て自動で重ねる)、
  macOS 向けで `VCAM=1` のときの Camera Extension (`tauri.macos-vcam.conf.json`)。
- **できたもの** はアプリと OS ごとに `target/dist/<アプリ>/<OS>/` へ写し、最後に一覧を出す (前の中身は消す)。
  tauri のバンドルのディレクトリ ([できるもの](#できるもの)) にも残る。
- 複数のアプリと OS を指定すると、OS ごとにアプリを順にビルドする。1 つ失敗したらそこで止まる。
- 前からある `just editor-build`・`just editor-build-windows`・`just viewer-build`・`just studio-build` は、
  `just app::build TARGET=<アプリ> [PLATFORM=windows]` の別名として残している (引数も同じように渡せる)。
- Windows の PowerShell は引数の `editor,viewer` を 2 つの引数に分けるが、スクリプトが続きとして読むのでそのまま書ける。

ビルドせずに試すだけなら、GitHub Actions の [Editor build](../../.github/workflows/editor-build.yml) の成果物を使う
(Actions のタブで実行を選び、Artifacts から `ugoe-editor-<OS>` をダウンロードする。手動実行か `v*` タグで動く)。

### 共通の前提

| ツール | 入れ方 |
| --- | --- |
| Rust | [rustup](https://rustup.rs/)。版と wasm32 ターゲットは `rust-toolchain.toml` で固定され、初回の `cargo` で自動的に入る |
| Node.js | `.tool-versions` の版 (asdf / mise / 公式インストーラなど) |
| pnpm | `package.json` の `packageManager` の版。Node 25 以降は corepack が同梱されないので `npm install -g corepack` してから `corepack enable` |
| just | `.tool-versions` の版。`cargo install just --locked`、`winget install Casey.Just`、`brew install just` など |
| wasm-bindgen-cli | `Cargo.lock` の wasm-bindgen と同じ版が必要 (違うと `just wasm` が止まる)。`just wasm-bindgen-version` で版を調べて `cargo install wasm-bindgen-cli --version <版> --locked` |

OS ごとの前提パッケージは [Tauri の Prerequisites](https://v2.tauri.app/start/prerequisites/) が詳しい。要点は次のとおり。

- **Windows:** Visual Studio Build Tools の「C++ によるデスクトップ開発」(MSVC と Windows SDK)。
  Rust は MSVC のツールチェイン (`x86_64-pc-windows-msvc`、rustup の既定) を使う。
  WebView2 は Windows 10 (1803 以降)・11 には入っている。
  `.msi` を作るには、Windows の「オプション機能」で VBSCRIPT が有効である必要がある (WiX は Tauri が自動で取得する)。
- **macOS:** Xcode Command Line Tools (`xcode-select --install`)。
- **Linux (Debian / Ubuntu):**
  `sudo apt install build-essential libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`

### ビルドの手順

リポジトリのルートで実行する。Windows でも PowerShell からそのまま `just` を使える
(justfile の `windows-shell` で PowerShell を使う。`test-gpu` と `commitlint` だけは bash が要る)。

```sh
just setup                  # pnpm install
cargo install wasm-bindgen-cli --version "$(just wasm-bindgen-version)" --locked   # 初回のみ (PowerShell でも同じ)
just editor                 # 開発モードで起動して確かめる
just app::build             # Windows・Linux: 手元の CPU 向けのインストーラ (TARGET= で絞れる)
```

macOS では、既定で Apple Silicon と Intel の両方で動く universal バイナリ (`--target universal-apple-darwin`) を作る (CI も同じ)。
片方だけなら `--target aarch64-apple-darwin` のように渡す:

```sh
rustup target add aarch64-apple-darwin x86_64-apple-darwin   # 初回のみ
just app::build PLATFORM=macos
```

WSL では、AppImage を作る linuxdeploy が `PATH` に含まれる Windows のディレクトリ (`/mnt/c/...`) を読めずに失敗するので、
Linux 向けのビルドでは `PATH` から `/mnt/` で始まるディレクトリを外して `tauri build` を実行する (スクリプトが自動で行う)。

### Linux (WSL2) から Windows 向けにビルドする

Windows に切り替えずに、Linux から Windows x86_64 向けの実行ファイルと NSIS のインストーラを作れる
([Tauri の Windows Installer の「Build Windows apps on Linux and macOS」](https://v2.tauri.app/distribute/windows-installer/#build-windows-apps-on-linux-and-macos))。
[cargo-xwin](https://github.com/rust-cross/cargo-xwin) が MSVC の CRT と Windows SDK を用意し、LLVM の `clang` と `lld-link` でビルドする。

前提 (「共通の前提」に加えて。Debian / Ubuntu の例):

```sh
sudo apt install clang lld llvm nsis                     # clang・lld-link・llvm-rc・llvm-lib・makensis
rustup target add x86_64-pc-windows-msvc                 # リポジトリの中で実行する (rust-toolchain.toml の版に入る)
cargo install cargo-xwin --locked
```

```sh
just app::build PLATFORM=windows                 # 3 つのアプリ。足りない道具があれば、入れ方を示して止まる
just app::build TARGET=editor PLATFORM=windows   # エディタだけ (`just editor-build-windows` と同じ)
```

- できるもの: `target/dist/<アプリ>/windows/<製品名>_<版>_x64-setup.exe` (インストーラ。エディタは約 29 MiB・ビューアは約 21 MiB・スタジオは約 20 MiB。
  元は `target/x86_64-pc-windows-msvc/release/bundle/nsis/`) と、`target/x86_64-pc-windows-msvc/release/ugoe-editor.exe`
  (実行ファイル、約 13 MiB。ビューアは `ugoe-viewer.exe`、スタジオは `ugoe-studio.exe`)。
  `KEY=値` でない引数は `tauri build` に渡される (例: `just app::build PLATFORM=windows --debug`)。
- スタジオは仮想カメラの DLL (`ugoe_studio_vcam.dll`) も cargo-xwin で作ってインストーラに入れる (`tauri.windows.conf.json`)。
  インストーラは管理者の権限で入れる (perMachine)。
- WSL2 からは、実行ファイルを Windows 側にコピーしてそのまま起動できる
  (例: `cp target/x86_64-pc-windows-msvc/release/ugoe-editor.exe /mnt/c/Users/<名前>/Downloads/` してエクスプローラーから開く)。
- **NSIS だけ。** `.msi` (WiX) は Windows でしか作れない。
- **署名しない** (手元で作るものは署名の道具が無いため。署名して配る版は CI の Windows ランナーで作る。[配布物の署名](#配布物の署名))。SmartScreen の警告が出る。
- **WebView2:** インストーラは、WebView2 Runtime がなければ Microsoft からブートストラッパーを
  ダウンロードして入れる (Tauri の既定の `downloadBootstrapper`。インターネット接続が要る)。
  Windows 10 (1803 以降)・11 には通常入っているので、何も起きない。実行ファイルを直接起動するときも WebView2 Runtime が要る。
- 初回のビルドで cargo-xwin が **MSVC の CRT と Windows SDK をダウンロード**して `~/.cache/cargo-xwin` に置く (約 1 GiB)。
  これらは Microsoft のライセンス (Visual Studio のライセンス条項) の下にあり、同意するかは利用者の責任で判断する。
  リポジトリには置かない (コミットしない)。
- リンク時に `lld-link: Cannot use debug info for 'libcmt.lib(...)' [LNK4099]` の警告が大量に出るが、
  CRT のデバッグ情報 (PDB) がないというだけで、できるものには影響しない。
- Tauri は Linux からのクロスビルドを実験的な機能としている (`Cross-platform compilation is experimental` の警告が出る)。
  配布する版は、CI の Windows ランナーのネイティブビルド ([Editor build](../../.github/workflows/editor-build.yml)) を使う。

### できるもの

| OS | 場所 (リポジトリのルートから) | 形式 |
| --- | --- | --- |
| Windows | `target/release/bundle/msi/`・`target/release/bundle/nsis/` | `.msi`・セットアップ用の `.exe` (NSIS) |
| Windows (Linux からのクロスビルド) | `target/x86_64-pc-windows-msvc/release/bundle/nsis/` | セットアップ用の `.exe` (NSIS) |
| macOS | `target/universal-apple-darwin/release/bundle/dmg/`・`.../bundle/macos/` | `.dmg`・`ugoe.app` |
| Linux | `target/release/bundle/deb/`・`rpm/`・`appimage/` | `.deb`・`.rpm`・`.AppImage` |

`just app::build` は、このうちインストーラ (`.deb`・`.rpm`・`.AppImage`・`.msi`・NSIS の `.exe`・`.dmg`) を
`target/dist/<アプリ>/<OS>/` (`<OS>` は `windows`・`macos`・`linux`) に写す。バンドルのディレクトリはアプリの間で共有なので、
製品名 (`tauri.conf.json` の `productName`。エディタは `ugoe`、ビューアは `ugoe-viewer`、スタジオは `ugoe-studio`) と版で始まり、
そのビルドでできたファイルだけを選ぶ。CI はこのディレクトリを成果物として上げる。

実行ファイルだけなら `target/release/ugoe-editor` (Windows は `ugoe-editor.exe`) にもできる。

### 配布物の署名

署名と公証は、署名の環境変数 (CI では GitHub の Secrets) があるときだけ行う。手順・環境変数・Secrets の名前は
[docs/release.md#デスクトップアプリの署名](../../docs/release.md#デスクトップアプリの署名)。3 つのアプリで同じ。
署名したかは `just app::build` の最後の表示と、CI のジョブの要約の「Code signing」の表に出る。

**署名したもの** (Secrets を置いたリポジトリの CI・署名の環境変数を設定した手元のビルド):

- **Windows:** 実行ファイル・インストーラ (NSIS の `.exe` と `.msi`)・エディタのサイドカー `ugoe-mcp.exe`・スタジオの仮想カメラの DLL に
  Authenticode の署名とタイムスタンプが付く。SmartScreen の画面には発行元の名義が出る。評価はダウンロードの実績で付くので、
  新しい版では、署名していても初めのうちは「Windows によって PC が保護されました」が出ることがある (越え方は下と同じ)。
- **macOS:** Developer ID Application で署名し、公証してステープルした `.app` と `.dmg`。Gatekeeper の警告は出ない。
- **Linux:** 署名しない (下と同じ)。

**署名しないもの** (Secrets の無いビルド: フォーク・Secrets を置く前の CI・手元のビルド)。初回の起動時に OS の警告が出る。

- **Windows:** SmartScreen の「Windows によって PC が保護されました」が出る。「詳細情報」→「実行」で起動できる。
- **macOS:** Gatekeeper が「開発元を検証できない」または「壊れているため開けません」として止める。
  「システム設定」→「プライバシーとセキュリティ」の「このまま開く」を押すか、
  `xattr -dr com.apple.quarantine /Applications/ugoe.app` で隔離属性を外す。
- **Linux:** 警告はない。AppImage は `chmod +x` してから実行する。
