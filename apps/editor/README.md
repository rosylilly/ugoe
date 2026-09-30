# apps/editor

ugoe のエディタ (Tauri v2 + Svelte 5)。設計は [docs/editor.md](../../docs/editor.md)。

```
src/              フロントエンド (Svelte 5 + TS)
  lib/ipc.ts        バックエンドとの通信はすべてここを通す
  lib/preview/      プレビューモード。フェイストラッキングは共有の @ugoe/face-tracking (packages/face-tracking/)
public/mediapipe/ MediaPipe の wasm とモデル。`pnpm mediapipe` (dev・build の前にも) が用意する (gitignore 済み)
src-tauri/        バックエンド (Rust クレート `ugoe-editor`)
  src/commands.rs   IPC コマンド。薄く保ち、編集ロジックは ugoe-editor-core に置く
  src/tracking.rs   フェイストラッキングのカメラの許可と記録の保存。Info.plist・Entitlements.plist は macOS のカメラ
  src/mcp.rs        エディタの中の MCP サーバ (エージェントをつなぐ。docs/editor.md#エージェントをつなぐ)
  tauri.bundle.conf.json  インストーラを作るときに重ねる設定 (ugoe-mcp のサイドカー)
scripts/sidecar.mjs 同梱する ugoe-mcp をビルドして src-tauri/binaries/ に置く (`pnpm sidecar`)
```

## 守ること

- 編集ロジックを書かない。ユーザーの入力を `Command` に変換して `apply_command` を呼ぶ。
- IPC コマンドを変えたら、同じ PR で `src/lib/ipc.ts` を更新する。
- Svelte 5 の runes を使う。状態管理のライブラリは入れない。

## コマンド

```sh
just editor         # 開発モードで起動 (vite + tauri dev)
just editor-build   # リリースビルド (インストーラを作る)
just web-check      # Biome + svelte-check
```

`just editor` と `just editor-build` は、先に `just wasm` (ugoe-wasm のビルドと JS バインディングの生成) を実行する
(`tauri.conf.json` の `beforeDevCommand` / `beforeBuildCommand`)。

`just editor-build` (と `just editor-build-windows`) は `--config src-tauri/tauri.bundle.conf.json` を重ね、MCP サーバの
`ugoe-mcp` をサイドカー (Tauri の `externalBin`) として同梱する。重ねた設定の `beforeBuildCommand` が `pnpm sidecar`
(`scripts/sidecar.mjs`) でビルドのターゲット向けに `ugoe-mcp` をリリースでビルドし、`src-tauri/binaries/ugoe-mcp-<三つ組>[.exe]`
(gitignore 済み) に置く。インストールすると `ugoe-mcp` はエディタの実行ファイルの隣に入り、設定の「Connect agents」が
エージェントの登録のコマンドにそのパスを埋める。`tauri.conf.json` に `externalBin` を書かないのは、tauri-build がファイルの
無い `externalBin` で `cargo check` と `tauri dev` も失敗させるため。`just editor` (開発中) は同梱しないので、画面は PATH の
`ugoe-mcp` を使うコマンドを出す (`cargo build --release -p ugoe-mcp` で作れる)。

## 各 OS でのビルド

Tauri は他の OS 向けのクロスビルドが難しいので、**インストーラは基本的にその OS の上でビルドする**。
WSL の中で `just editor-build` を実行すると Linux 向けのバイナリができる (WSLg で動かせるが、Windows 向けではない)。
Windows 向けは、Windows 上 (PowerShell) で直接ビルドするか、
Linux (WSL2 を含む) から `just editor-build-windows` でクロスビルドする
([Linux (WSL2) から Windows 向けにビルドする](#linux-wsl2-から-windows-向けにビルドする)。NSIS のインストーラだけ)。

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
just editor-build           # Windows・Linux: 手元の CPU 向けのインストーラ
```

macOS では、Apple Silicon と Intel の両方で動く universal バイナリを作る (CI も同じ):

```sh
rustup target add aarch64-apple-darwin x86_64-apple-darwin   # 初回のみ
just editor-build --target universal-apple-darwin
```

`just editor-build` の引数は `tauri build` に渡される (例: `--bundles deb` で .deb だけを作る)。

WSL では、AppImage を作る linuxdeploy が `PATH` に含まれる Windows のディレクトリ (`/mnt/c/...`) を読めずに失敗する。
`PATH` から Windows のディレクトリを外して実行する:

```sh
PATH="$(printf '%s' "$PATH" | tr ':' '\n' | grep -v '^/mnt/' | paste -sd:)" just editor-build
```

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
just editor-build-windows   # 足りない道具があれば、入れ方を示して止まる
```

- できるもの: `target/x86_64-pc-windows-msvc/release/bundle/nsis/ugoe_<版>_x64-setup.exe` (インストーラ、約 4 MiB) と
  `target/x86_64-pc-windows-msvc/release/ugoe-editor.exe` (実行ファイル、約 13 MiB)。
  引数は `tauri build` に渡される (例: `just editor-build-windows --debug`)。
- WSL2 からは、実行ファイルを Windows 側にコピーしてそのまま起動できる
  (例: `cp target/x86_64-pc-windows-msvc/release/ugoe-editor.exe /mnt/c/Users/<名前>/Downloads/` してエクスプローラーから開く)。
- **NSIS だけ。** `.msi` (WiX) は Windows でしか作れない。
- **署名していない** ([配布物の署名](#配布物の署名))。SmartScreen の警告が出る。
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

実行ファイルだけなら `target/release/ugoe-editor` (Windows は `ugoe-editor.exe`) にもできる。

### 配布物の署名

インストーラは**コード署名をしていない** (署名と公証は後続のタスク)。そのため、初回の起動時に OS の警告が出る。

- **Windows:** SmartScreen の「Windows によって PC が保護されました」が出る。「詳細情報」→「実行」で起動できる。
- **macOS:** Gatekeeper が「開発元を検証できない」または「壊れているため開けません」として止める。
  「システム設定」→「プライバシーとセキュリティ」の「このまま開く」を押すか、
  `xattr -dr com.apple.quarantine /Applications/ugoe.app` で隔離属性を外す。
- **Linux:** 警告はない。AppImage は `chmod +x` してから実行する。
