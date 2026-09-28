# apps/editor

ugoe のエディタ (Tauri v2 + Svelte 5)。設計は [docs/editor.md](../../docs/editor.md)。

```
src/              フロントエンド (Svelte 5 + TS)
  lib/ipc.ts        バックエンドとの通信はすべてここを通す
src-tauri/        バックエンド (Rust クレート `ugoe-editor`)
  src/commands.rs   IPC コマンド。薄く保ち、編集ロジックは ugoe-editor-core に置く
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

## 各 OS でのビルド

Tauri は他の OS 向けのクロスビルドが難しいので、**インストーラはその OS の上でビルドする**。
WSL の中でビルドすると Linux 向けのバイナリができる (WSLg で動かせるが、Windows 向けではない)。
Windows 向けは、WSL ではなく Windows 上 (PowerShell) で直接ビルドする。

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

### できるもの

| OS | 場所 (リポジトリのルートから) | 形式 |
| --- | --- | --- |
| Windows | `target/release/bundle/msi/`・`target/release/bundle/nsis/` | `.msi`・セットアップ用の `.exe` (NSIS) |
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
