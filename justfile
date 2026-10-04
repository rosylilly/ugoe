# ugoe タスクランナー。`just` で一覧表示。
# エージェントは作業完了前に必ず `just ci` を通すこと。

set shell := ["bash", "-euo", "pipefail", "-c"]

# Windows では PowerShell で実行する (bash なしでエディタをビルドできるように)。
# Windows で動くのは、1 行に 1 コマンドのレシピと Node のスクリプトを呼ぶレシピ
# (setup・wasm・wasm-bindgen-version・editor・editor-build・viewer・viewer-build・app::build・ci など)。
# `@` で始まる引数が PowerShell の splatting と解釈されないよう、`@ugoe/editor` などは引用符で囲む。
# `test-gpu` と `commitlint` は bash の構文を使うので、Windows では Git Bash か WSL で実行する。
set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

# デスクトップアプリのビルド: `just app::build TARGET=editor,viewer,studio PLATFORM=windows,macos,linux` (scripts/app.just)
mod app "scripts/app.just"

default:
    @just --list

# 初回セットアップ (Rust ツールチェインは rust-toolchain.toml で自動導入される)
setup:
    pnpm install  # lefthook の Git フックもここで入る
    rustup show active-toolchain

# CI と同じ検査一式
ci: fmt-check lint test layers refs groups licenses publish-check spec web-check i18n app-build-check

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

# CI のジョブのまとまり (グループ) の検査: すべてのクレートがどれか 1 つのグループに入っているか (scripts/ci-groups.mjs)
groups:
    @node --test --test-reporter=dot scripts/ci-groups.test.mjs
    @node scripts/ci-groups.mjs check

# グループの一覧は `node scripts/ci-groups.mjs list`。CI の rust (<グループ>) ジョブと同じ検査。
# lint-group と test-group は bash の構文を使うので、Windows では Git Bash か WSL で実行する。
# 1 つのグループだけの clippy: `just lint-group runtime`
lint-group name:
    args="$(node scripts/ci-groups.mjs args {{name}})"; cargo clippy $args --all-targets -- -D warnings

# 1 つのグループだけのテスト: `just test-group tools`。引数は cargo test に渡す
test-group name *args:
    args="$(node scripts/ci-groups.mjs args {{name}})"; cargo test $args {{args}}

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

# 依存クレートのライセンスの検査 (docs/adr/0018-licensing-and-1.0-scope.md)
licenses:
    @node scripts/check-licenses.mjs

# crates.io に公開するクレートの準備の検査 (メタデータ・LICENSE の写し・内部の依存の版。docs/release.md#cratesio)
publish-check:
    @node --test --test-reporter=dot scripts/crates.test.mjs
    @node scripts/crates.mjs check

# crates.io への公開の予行 (パッケージを作り、crates.io の代わりに一時的なレジストリでビルドする。公開はしない)。
# crates.io の索引を読むのでネットワークが要る。`just ci` には含めない (CI では crates-release.yml が実行する)
publish-dry-run *args:
    cargo publish --workspace --dry-run {{args}}

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

# 参照の突き合わせ: コードのコメントから文書の見出しへの参照の切れと、C ABI のヘッダとバインディング (Unity) の宣言の過不足
# (docs/workflow/code-review.md#m9-の結果)
refs:
    @node --test --test-reporter=dot scripts/check-refs.test.mjs
    @node scripts/check-doc-refs.mjs
    @node scripts/check-ffi-bindings.mjs

# C ABI のヘッダ crates/ugoe-ffi/include/ugoe.h を cbindgen で作り直す (crates/ugoe-ffi/README.md#ヘッダの生成)。
# cbindgen は MPL-2.0 なので CLI として使う: cargo install cbindgen --version 0.29.4 --locked
ffi-header:
    @node scripts/ffi.mjs header

# C ABI の検査: ヘッダが最新か、C の見本のビルドと実行。cbindgen と C コンパイラが要るので `just ci` には含めない
ffi-check:
    @node scripts/ffi.mjs check

# `just godot-build --release` でリリースビルド。Godot は要らない。
# Godot のバインディング (bindings/godot、ADR 0026) をビルドし、見本 (bindings/godot/example) に置く
godot-build *args:
    node bindings/godot/build.mjs {{args}}

# Godot 4.4 以降 (環境変数 GODOT、既定は godot) と、画面が無ければ xvfb-run が要る (無ければ入れ方を示して止まる)。
# `just ci` には含めない (CI では別のジョブ)。`just godot-check --only masks-nested` で比べるケース (tests/conformance/cases.json の名前) を絞れる。
# Godot のバインディングの検査 (ライセンス・ヘッドレスの Godot での動作・`ugoe render` との比べ合わせ)
godot-check *args:
    node bindings/godot/build.mjs --check --conformance {{args}}

# 配布物の適合テスト (tests/conformance/README.md): C ABI・wasm (npm パッケージ、WebGL2 と WebGPU)・Unity の C# の層・Godot の頂点と画像を、ModelInstance と ugoe render の参照と比べる。
# C コンパイラ・Chrome・wasm-bindgen-cli・GPU (無ければ Mesa) が要るので `just ci` には含めない (CI の conformance ジョブで実行する)。
# 引数は run.mjs に渡す: `just conformance --only c`、`just conformance --backend webgpu`
conformance *args:
    @node tests/conformance/run.mjs {{args}}

# `just unity-build --release` でリリースビルド。Unity は要らない。
# Unity のパッケージ (bindings/unity) にホスト向けの ugoe-ffi と見本のモデルを置く (.meta も作る)
unity-build *args:
    node bindings/unity/build.mjs {{args}}

# .NET SDK 8 以降が要る (無ければ入れ方を示して止まる)。`just ci` には含めない (CI では unity.yml の check)。
# Unity のパッケージの検査 (.meta・Runtime/Core の dotnet test・Unity の部分のスタブでの型検査を netstandard2.1 と net48 で)
unity-check *args:
    node bindings/unity/build.mjs --check {{args}}

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
    pnpm --filter '@ugoe/studio' check
    pnpm --filter '@ugoe/studio' test
    pnpm --filter '@ugoe/face-tracking' check
    pnpm --filter '@ugoe/face-tracking' test
    pnpm --filter '@ugoe/player' check
    pnpm --filter '@ugoe/player' test
    pnpm --filter '@ugoe/i18n' check
    pnpm --filter '@ugoe/i18n' test
    pnpm --filter '@ugoe/runtime' check
    pnpm --filter '@ugoe/runtime' test
    pnpm --filter '@ugoe/ui' check
    pnpm --filter '@ugoe/ui' test

# 翻訳のカタログの検査: 言語の間のキーの過不足・複数形・引数・未使用のキー (packages/i18n/README.md)
# と、画面の直書きの文言の検査 (docs/translation.md#直書きの文言の検査)
i18n:
    @node packages/i18n/scripts/check-catalogs.mjs
    @node --test --test-reporter=dot scripts/check-hardcoded-strings.test.mjs
    @node scripts/check-hardcoded-strings.mjs

# エディタを開発モードで起動
editor:
    pnpm --filter '@ugoe/editor' tauri dev

# エディタのリリースビルド。`just app::build TARGET=editor` の別名 (引数も同じ。apps/editor/README.md#各-os-でのビルド)
editor-build *args:
    @node scripts/app-build.mjs TARGET=editor {{args}}

# Linux (WSL2 を含む) から Windows 向けのエディタをクロスビルドする。`just app::build TARGET=editor PLATFORM=windows` の別名
editor-build-windows *args:
    @node scripts/app-build.mjs TARGET=editor PLATFORM=windows {{args}}

# ビューアを開発モードで起動 (apps/viewer/README.md)
viewer:
    pnpm --filter '@ugoe/viewer' tauri dev

# ビューアのリリースビルド。`just app::build TARGET=viewer` の別名: `just viewer-build --bundles deb`
viewer-build *args:
    @node scripts/app-build.mjs TARGET=viewer {{args}}

# スタジオを開発モードで起動 (apps/studio/README.md)
studio:
    pnpm --filter '@ugoe/studio' tauri dev

# スタジオのリリースビルド。`just app::build TARGET=studio` の別名: `just studio-build --bundles deb`
studio-build *args:
    @node scripts/app-build.mjs TARGET=studio {{args}}

# アプリのビルドの入口 (scripts/app-build.mjs) の引数と組み合わせ、手で確かめる一式 (scripts/check-pack.mjs) のパスの検査
app-build-check:
    @node --test --test-reporter=dot scripts/app-build.test.mjs scripts/check-pack.test.mjs

# CLI 実行: `just cli info model.json`
cli *args:
    @cargo run -q -p ugoe-cli -- {{args}}

# API ドキュメント
doc:
    cargo doc --workspace --no-deps --open

# @ugoe/ui の部品の見本の画面をヘッドレスの Chrome で撮って検査する (packages/ui/README.md): `just ui-shots /tmp/shots`
ui-shots out:
    node packages/ui/scripts/shots.mjs {{out}}

# 実機で手で確かめる一式 (Windows のインストーラ・見本・見本のプロファイル・確認表) を Windows のフォルダーにまとめる (docs/manual-check.md)
check-pack *args:
    @node scripts/check-pack.mjs {{args}}

# 本物の Tauri のアプリ (エディタ・ビューア・スタジオ) を起動して操作する e2e テスト (tests/e2e/README.md。Linux だけ)。
# tauri-driver・WebKitWebDriver・Xvfb が要り、アプリをビルドするので `just ci` には含めない: `just e2e --only studio`
e2e *args:
    @node tests/e2e/run.mjs {{args}}
