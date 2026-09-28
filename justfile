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

# wasm ターゲットでビルドできるか
wasm-check:
    cargo check -p ugoe-wasm --target wasm32-unknown-unknown

# フロントエンドの lint / 型検査
web-check:
    pnpm biome check .
    pnpm --filter @ugoe/editor check

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
