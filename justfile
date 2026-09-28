# ugoe タスクランナー。`just` で一覧表示。
# エージェントは作業完了前に必ず `just ci` を通すこと。

set shell := ["bash", "-euo", "pipefail", "-c"]

default:
    @just --list

# 初回セットアップ (Rust ツールチェインは rust-toolchain.toml で自動導入される)
setup:
    pnpm install  # lefthook の Git フックもここで入る
    rustup show active-toolchain

# CI と同じ検査一式
ci: fmt-check lint test layers licenses web-check

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

# クレート依存方向の検査
layers:
    @node scripts/check-layers.mjs

# wasm ターゲットでビルドできるか (ブラウザ専用のコード `src/web.rs` も clippy で検査する)
wasm-check:
    cargo clippy -p ugoe-wasm --target wasm32-unknown-unknown -- -D warnings

# ugoe-wasm をビルドし、エディタが読む JS バインディングを生成する (出力は gitignore 済み)。
# wasm-bindgen-cli は Cargo.lock の wasm-bindgen と同じ版が必要:
#   cargo install wasm-bindgen-cli --version "$(just wasm-bindgen-version)" --locked
wasm out="apps/editor/src/lib/viewport/generated":
    #!/usr/bin/env bash
    set -euo pipefail
    want="$(just wasm-bindgen-version)"
    have="$(wasm-bindgen --version 2>/dev/null | awk '{print $2}' || true)"
    if [ "$have" != "$want" ]; then
        echo "error: wasm-bindgen-cli $want is required (found: ${have:-none})." >&2
        echo "  cargo install wasm-bindgen-cli --version $want --locked" >&2
        exit 1
    fi
    cargo build -p ugoe-wasm --target wasm32-unknown-unknown --release
    wasm-bindgen --target web --out-dir {{quote(out)}} \
        target/wasm32-unknown-unknown/release/ugoe_wasm.wasm

# Cargo.lock に記録された wasm-bindgen の版 (wasm-bindgen-cli はこれと一致させる)
wasm-bindgen-version:
    @awk '/^name = "wasm-bindgen"$/ { getline; gsub(/"/, "", $3); print $3 }' Cargo.lock

# フロントエンドの lint / 型検査 / 単体テスト (Vitest)
web-check:
    pnpm biome check .
    pnpm --filter @ugoe/editor check
    pnpm --filter @ugoe/editor test

# エディタを開発モードで起動
editor:
    pnpm --filter @ugoe/editor tauri dev

# エディタのリリースビルド
editor-build:
    pnpm --filter @ugoe/editor tauri build

# CLI 実行: `just cli info model.json`
cli *args:
    @cargo run -q -p ugoe-cli -- {{args}}

# API ドキュメント
doc:
    cargo doc --workspace --no-deps --open
