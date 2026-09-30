# ugoe タスクランナー。`just` で一覧表示。
# エージェントは作業完了前に必ず `just ci` を通すこと。

set shell := ["bash", "-euo", "pipefail", "-c"]

# Windows では PowerShell で実行する (bash なしでエディタをビルドできるように)。
# Windows で動くのは、1 行に 1 コマンドのレシピと Node のスクリプトを呼ぶレシピ
# (setup・wasm・wasm-bindgen-version・editor・editor-build・viewer・viewer-build・ci など)。
# `@` で始まる引数が PowerShell の splatting と解釈されないよう、`@ugoe/editor` などは引用符で囲む。
# `test-gpu` と `commitlint` は bash の構文を使うので、Windows では Git Bash か WSL で実行する。
set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

default:
    @just --list

# 初回セットアップ (Rust ツールチェインは rust-toolchain.toml で自動導入される)
setup:
    pnpm install  # lefthook の Git フックもここで入る
    rustup show active-toolchain

# CI と同じ検査一式
ci: fmt-check lint test layers licenses spec web-check

# コミットメッセージを検査: `just commitlint "feat(runtime): add foo"`
commitlint message:
    @printf '%s' {{quote(message)}} | pnpm exec commitlint

# Rust / TS のフォーマット
fmt:
    cargo fmt --all
    pnpm biome format --write .

fmt-check:
    cargo fmt --all -- --check
    pnpm biome format .

# clippy (警告はエラー扱い)
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# 全テスト (GPU が必要なテストは除外される。`just test-gpu` を参照)
test *args:
    cargo test --workspace {{args}}

# GPU を使う描画テスト。`gpu-tests` feature を持つ全クレートが対象 (scripts/gpu-test-args.mjs)。
# アダプタ情報も表示するには `just test-gpu -- --nocapture`
test-gpu *args:
    crates="$(node scripts/gpu-test-args.mjs)"; cargo test $crates {{args}}

# 特定クレートだけを素早く検査: `just crate ugoe-runtime`
crate name:
    cargo clippy -p {{name}} --all-targets -- -D warnings
    cargo test -p {{name}}

# タスク用 worktree を作る (main の target/ をコピーしてビルドを省く): `just wt-add T0104`
wt-add id *slug:
    @node scripts/worktree.mjs add {{id}} {{slug}}

# タスク用 worktree を削除する: `just wt-remove T0104 --delete-branch`
wt-remove id *flags:
    @node scripts/worktree.mjs remove {{id}} {{flags}}

# タスク一覧 (`just tasks --ready` で着手可能なものだけ)
tasks *args:
    @node scripts/tasks.mjs {{args}}

# 依存クレートのライセンスの検査 (docs/adr/0018-licensing.md)
licenses:
    @node scripts/check-licenses.mjs

# 公開仕様 (spec/) の検査: Markdown のリンク、例と JSON Schema、docs/spec との同期 (docs/spec/public-spec.md#同期の検査)、
# 公開仕様だけで書いた独立の読み込み器の型検査とテスト (tools/spec-check/README.md)
spec:
    node scripts/check-md-links.mjs docs spec
    node scripts/check-spec.mjs
    pnpm --filter ugoe-spec-check check
    node --test tools/spec-check/test/

# クレート依存方向の検査
layers:
    @node scripts/check-layers.mjs

# C ABI のヘッダ crates/ugoe-ffi/include/ugoe.h を cbindgen で作り直す (crates/ugoe-ffi/README.md#ヘッダの生成)。
# cbindgen は MPL-2.0 なので CLI として使う: cargo install cbindgen --version 0.29.4 --locked
ffi-header:
    @node scripts/ffi.mjs header

# C ABI の検査: ヘッダが最新か、C の見本のビルドと実行。cbindgen と C コンパイラが要るので `just ci` には含めない
ffi-check:
    @node scripts/ffi.mjs check

# wasm ターゲットでビルドできるか (ブラウザ専用のコード `src/web.rs` も clippy で検査する)
wasm-check:
    cargo clippy -p ugoe-wasm --target wasm32-unknown-unknown -- -D warnings

# ugoe-wasm をビルドし、エディタが読む JS バインディングを生成する (出力は gitignore 済み)。
# ビューアは `just wasm apps/viewer/src/lib/wasm/generated` (apps/viewer の `pnpm wasm`)。
# wasm-bindgen-cli は Cargo.lock の wasm-bindgen と同じ版 (`just wasm-bindgen-version`) が必要:
#   cargo install wasm-bindgen-cli --version <版> --locked
wasm out="apps/editor/src/lib/viewport/generated":
    @node scripts/wasm.mjs build {{quote(out)}}

# npm パッケージ @ugoe/runtime (bindings/web) をビルドする: wasm の生成・tsc・dist/ への出力 (ADR 0025)。
# ローカルで入れるには、続けて `cd bindings/web && pnpm pack --out ugoe-runtime.tgz` (bindings/web/README.md)。
web-runtime:
    pnpm --filter '@ugoe/runtime' build

# Rust の型から生成物を作り直す: エディタの TypeScript の型 (apps/editor/src/lib/generated/) と
# Command の JSON Schema (crates/ugoe-editor-core/schema/)、公開仕様の JSON Schema (spec/schema/)。
# 生成物はコミットする (docs/editor.md#型の生成)。
gen:
    cargo run -q -p ugoe-editor --example gen
    cargo run -q -p ugoe-model --features schema --example spec_files

# Cargo.lock に記録された wasm-bindgen の版 (wasm-bindgen-cli はこれと一致させる)
wasm-bindgen-version:
    @node scripts/wasm.mjs version

# フロントエンドの lint / 型検査 / 単体テスト (Vitest)
web-check:
    pnpm biome check .
    pnpm --filter '@ugoe/editor' check
    pnpm --filter '@ugoe/editor' test
    pnpm --filter '@ugoe/viewer' check
    pnpm --filter '@ugoe/viewer' test
    pnpm --filter '@ugoe/face-tracking' check
    pnpm --filter '@ugoe/face-tracking' test
    pnpm --filter '@ugoe/runtime' check
    pnpm --filter '@ugoe/runtime' test

# エディタを開発モードで起動
editor:
    pnpm --filter '@ugoe/editor' tauri dev

# エディタのリリースビルド (インストーラを作る。手順は apps/editor/README.md)。
# 引数は tauri build に渡す: `just editor-build --target universal-apple-darwin`
# MCP サーバの ugoe-mcp をサイドカーとして同梱する (src-tauri/tauri.bundle.conf.json。docs/editor.md#エージェントをつなぐ)。
editor-build *args:
    pnpm --filter '@ugoe/editor' tauri build --config src-tauri/tauri.bundle.conf.json {{args}}

# Linux (WSL2 を含む) から Windows x86_64 向けのエディタをクロスビルドする (NSIS だけ。署名なし)。
# 必要な道具を調べてから tauri build を cargo-xwin で実行する (scripts/editor-windows.mjs)。
# 手順と前提は apps/editor/README.md の「Linux (WSL2) から Windows 向けにビルドする」
editor-build-windows *args:
    @node scripts/editor-windows.mjs {{args}}

# ビューアを開発モードで起動 (apps/viewer/README.md)
viewer:
    pnpm --filter '@ugoe/viewer' tauri dev

# ビューアのリリースビルド。引数は tauri build に渡す: `just viewer-build --bundles deb`
viewer-build *args:
    pnpm --filter '@ugoe/viewer' tauri build {{args}}

# CLI 実行: `just cli info model.json`
cli *args:
    @cargo run -q -p ugoe-cli -- {{args}}

# API ドキュメント
doc:
    cargo doc --workspace --no-deps --open
