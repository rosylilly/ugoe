# AGENTS.md — ugoe で作業するエージェント (と人間) のためのガイド

このファイルはリポジトリで作業する **すべてのコーディングエージェントが最初に読む** 規約です。
短く保ち、詳細は `docs/` へリンクします。内容が古くなっていたら、気づいたエージェントが直してください。

## プロジェクト概要

ugoe は Live2D Cubism と **機能的に互換** な 2D リギング・アニメーションのエンジンとツールチェインです。

- 実装言語は Rust。エディタは Tauri v2 + Svelte 5。
- コアの描画は wgpu (WebGPU API) で、ネイティブ・Web・ゲームエンジンに移植できるようにする。
- Live2D の **ファイル形式や内部機構との互換は目指さない**。できることは同等にするが、データ形式とアルゴリズムは独自のもの。
- 将来は AI 連携 (PSD 取り込み → レイヤー意味分類 → 自動メッシュ → 自動パラメータ設定) を持つ。

目標と非目標の詳細は [docs/vision.md](docs/vision.md)、全体構成は [docs/architecture.md](docs/architecture.md) にあります。

## 最初に読むもの (作業内容別)

| 作業 | 読むもの |
| --- | --- |
| どの作業でも | この文書、[docs/architecture.md](docs/architecture.md)、[docs/glossary.md](docs/glossary.md)、触るクレートの `README.md` |
| タスクに着手する | [docs/workflow/multi-agent.md](docs/workflow/multi-agent.md)、[docs/tasks/README.md](docs/tasks/README.md) |
| モデルのデータ構造 | [docs/spec/model.md](docs/spec/model.md) |
| 変形・補間 | [docs/spec/evaluation.md](docs/spec/evaluation.md) |
| 描画 | [docs/spec/rendering.md](docs/spec/rendering.md) |
| 物理 / モーション | [docs/spec/physics.md](docs/spec/physics.md) / [docs/spec/motion.md](docs/spec/motion.md) |
| ファイル形式 | [docs/spec/file-formats.md](docs/spec/file-formats.md) |
| エディタ | [docs/editor.md](docs/editor.md) |
| スタジオ | [docs/studio.md](docs/studio.md) |
| 画面の部品・翻訳 | [docs/ui.md](docs/ui.md)、[ADR 0029](docs/adr/0029-shared-ui-and-i18n.md) |
| AI 連携 | [docs/ai-integration.md](docs/ai-integration.md) |
| エンジン組み込み | [docs/portability.md](docs/portability.md) |
| テスト | [docs/testing.md](docs/testing.md) |
| 設計判断の経緯 | [docs/adr/](docs/adr/) |

## リポジトリ構成

```
crates/             Rust クレート (ランタイム層とツール層。依存方向は docs/architecture.md)
  ugoe-core           数学型・ID・色
  ugoe-model          データ定義 (スキーマ)。アルゴリズムは持たない
  ugoe-runtime        キーフォーム補間・デフォーマ変形 (CPU・決定的)
  ugoe-physics        振り子物理
  ugoe-motion         モーション・表情・目パチ・呼吸・リップシンク・ポーズ
  ugoe-render         wgpu 参照レンダラ
  ugoe-format         .ugoe バンドルと JSON 形式の読み書き
  ugoe                ランタイム SDK ファサード (Puppet)
  ugoe-ffi            C ABI
  ugoe-wasm           WebAssembly バインディング
  ugoe-psd            PSD 取り込み
  ugoe-mesher         自動メッシュ生成
  ugoe-rigging        キーフォームの推定 (加算合成・ミラー・深度からの回転)
  ugoe-headless       ヘッドレス描画 (GPU の用意・大きさと fit・時間を進める・PNG。CLI と MCP サーバで共有)
  ugoe-editor-core    ヘッドレスなエディタ中核 (Document / Command / Undo)
  ugoe-ai             AI プロバイダ抽象と自動リギング
  ugoe-cli            `ugoe` コマンド
  ugoe-mcp            MCP サーバ (外部のエージェントが .ugoeproj を読み、編集を提案・適用する)
apps/editor/        Tauri エディタ (src/ = Svelte フロントエンド, src-tauri/ = Rust バックエンド)
apps/viewer/        Tauri ビューア (.ugoe を開いて表示しパラメータを動かす。構成はエディタと同じ)
apps/studio/        Tauri スタジオ (配信用。操作ウィンドウと出力ウィンドウ、指揮役と受け手、シーンとプロファイル、入力とホットキー。ADR 0028。vcam/ = 仮想カメラ (unsafe を許す唯一のアプリのクレート))
packages/           アプリ (エディタ・ビューア・スタジオ) で共有するフロントエンドの部品 (face-tracking・ui・i18n・player。GPL-3.0-only)
bindings/           ランタイムを別の環境へ出すパッケージ (MIT。web = npm の @ugoe/runtime と <ugoe-avatar>、godot = Godot 4 の GDExtension (ワークスペースの外、ADR 0026)、unity = UPM パッケージ (C# から C ABI を呼ぶ))
tests/conformance/  配布物の適合テスト (ugoe-conformance。C ABI・wasm の頂点と画像を参照と比べる。`just conformance`)
docs/               設計・仕様・ADR・タスク
scripts/            補助スクリプト (check-layers.mjs 等)
assets/fixtures/    テスト用アセット (小さいものだけ)
assets/compat/      互換性テストの資料 (リリースごとのファイルと未来の版を模したファイル。docs/testing.md#互換性テスト)
```

## コマンド

`just` がタスクランナーです (`just` だけで一覧を表示)。

| コマンド | 内容 |
| --- | --- |
| `just ci` | **作業完了前に必ず通す。** fmt 検査・clippy (`-D warnings`)・全テスト・依存方向・フロントエンド検査 |
| `just crate <name>` | 1 クレートだけ clippy とテスト (反復中はこれを使う) |
| `just fmt` | Rust と TS を整形 |
| `just layers` | クレートの依存方向を検査 |
| `just test-group <group>` / `just lint-group <group>` | CI の 1 つのグループ (`scripts/ci-groups.mjs`) だけテスト / clippy |
| `just licenses` | 依存するクレートと npm パッケージのライセンスを検査 (MIT のクレートにコピーレフトを入れない。アプリと packages/ の npm の依存は GPL-3.0 と両立するもの) |
| `just publish-check` | crates.io に公開するクレートの準備を検査 (メタデータ・LICENSE の写し・内部の依存の版。[docs/release.md](docs/release.md#cratesio)) |
| `just wasm-check` | wasm32 でビルドできるか (ランタイム層を触ったら実行) |
| `just wt-add <ID>` / `just wt-remove <ID>` | タスク用 worktree の作成・削除 (main の `target/` をコピーしてビルドを省く) |
| `just editor` | エディタを開発モードで起動 |
| `just viewer` | ビューアを開発モードで起動 |
| `just app::build TARGET=editor,viewer,studio PLATFORM=windows,macos,linux` | デスクトップアプリのビルド (既定はすべてのアプリを今の OS 向けに。`DRY_RUN=1` で表示だけ。docs/release.md) |
| `just studio` | スタジオを開発モードで起動 |
| `just cli <args>` | CLI を実行 (例: `just cli fixture single-quad`) |

## 守るべきルール

### アーキテクチャ

1. **依存方向を守る。** ランタイム層 (`core` `model` `runtime` `format` `physics` `motion` `render` `ugoe` `ffi` `wasm`) は、ツール層 (`psd` `mesher` `rigging` `editor-core` `ai` `cli` `editor`) に依存してはならない。`just layers` で検査される。新しいクレートを作ったら `scripts/check-layers.mjs` に登録し、`scripts/ci-groups.mjs` の CI のグループにも入れる ([docs/testing.md#ci-のジョブ](docs/testing.md#ci-のジョブ))。
2. **`ugoe-model` はデータだけを持つ。** 補間・変形・再生などのロジックは下流クレートに置く。
3. **ランタイム層は GPU・ファイル I/O・スレッド・時刻取得に依存しない** (`ugoe-render` の GPU と `ugoe-format` のバイト列処理を除く)。`dt` は引数で受け取る。wasm32 でビルドできる状態を保つ。
4. **エディタの編集はすべて `ugoe_editor_core::Command` を通す。** Tauri コマンドや CLI に編集ロジックを直接書かない。
5. **AI はモデルを直接書き換えない。** `Command` の列を「提案」し、ユーザーが承認してから適用する。
6. **レンダラはデバイスやサーフェスを所有しない。** ホストから `Device` / `Queue` / `TextureView` を受け取る。

### コード

- Rust edition 2024、toolchain は `rust-toolchain.toml` で固定。`unsafe` は禁止 (例外は `ugoe-ffi` と、[ADR 0028](docs/adr/0028-studio.md) で決めた仮想カメラの 1 クレートだけ)。
- 依存クレートはルートの `Cargo.toml` の `[workspace.dependencies]` に追加し、各クレートでは `foo.workspace = true` と書く。1 行に 1 依存、アルファベット順に並べる (マージ衝突を減らすため)。
- 新しい外部依存を入れるときは PR 本文に理由を書く。ランタイム層に重い依存を入れるなら ADR を書く。
- ライブラリコードで `unwrap()` を使わない (テストでは可)。エラー型は `thiserror` で作り、`anyhow` はバイナリ (`ugoe-cli` 等) だけで使う。
- 未実装の箇所は `todo!()` ではなく `// TODO(T0123): 説明` と書き、タスク ID を付ける。
- コメントとドキュメントは日本語、識別子は英語。
- 公開 API には doc コメントを付ける。仕様書と対応する箇所ではファイル名をコメントに書く (例: `docs/spec/evaluation.md`)。

### 外部のコードとライセンス

- **ライセンス:** `crates/`・`bindings/`・`docs/` は MIT、`apps/editor/`・`apps/viewer/`・`apps/studio/`・`packages/` は GPL-3.0-only ([ADR 0018](docs/adr/0018-licensing-and-1.0-scope.md)・[ADR 0019](docs/adr/0019-viewer-license.md)・[ADR 0025](docs/adr/0025-web-runtime-package.md)・[ADR 0028](docs/adr/0028-studio.md))。MIT の `bindings/` から GPL のコードを import したり写したりしない。新しいクレートは `license.workspace = true` を書く。
- **MIT のクレートに、コピーレフト (MPL・LGPL・GPL など) のライブラリを依存させない。** `just licenses` (`just ci` に含まれる) が依存するクレートのライセンスを検査する。例外は `bindings/godot` で、ワークスペースの外に置いて godot-rust (MPL-2.0) にだけ依存を許す ([ADR 0026](docs/adr/0026-godot-binding.md)。検査は `node scripts/check-licenses.mjs --godot`)。

- **Live2D Cubism SDK などの外部 SDK のソースを、リポジトリの中に置かない (ダウンロードしない)。コードも写さない。** 挙動を調べるためにソースを読むのはよいが、読むときはリポジトリの外 (一時ディレクトリなど) で読む。
- 調べて分かった挙動は、仕様書や ADR に「何をするか」として自分の言葉で書き、出典の URL を示す。
- コミットの前に `git status` で、意図しないファイル (調査用のダウンロードなど) が含まれていないことを確認する。`git add -A` を使うときは特に注意する。

### 仕様とコードの同期

- `ugoe-model` の型を変えたら **同じ PR で** `docs/spec/model.md` を更新する。ファイル形式に影響するなら `docs/spec/file-formats.md` と `SCHEMA_VERSION` も更新する。
- 英語の公開仕様 (`spec/`) の対応する節も同じ PR で直す。対応は [docs/spec/public-spec.md](docs/spec/public-spec.md)、食い違ったら英語版が正 ([ADR 0021](docs/adr/0021-public-specification.md))。
- Tauri の IPC コマンドを変えたら、同じ PR で `apps/editor/src/lib/ipc.ts` も更新する。
- 既存の設計判断 (ADR) を覆すときは、新しい ADR を書いて古い ADR を `Superseded` にする。

### テスト

- 振る舞いを変えたらテストを書く。ランタイムの数値結果は `ugoe_model::fixtures` のフィクスチャを入力にしてテストする。
- 浮動小数点の比較は許容誤差付きで行う (完全一致が保証されている場合を除く)。
- 詳細は [docs/testing.md](docs/testing.md)。

## コミット

- 形式は [Conventional Commits](https://www.conventionalcommits.org/)。例: `feat(runtime): implement warp deformer (T0104)`
- **コミットメッセージはすべて英語で書く** (件名・本文・フッターすべて)。日本語などの CJK 文字や全角文字を含むメッセージは拒否される。
- type は `feat` `fix` `docs` `refactor` `perf` `test` `build` `ci` `chore` `style` `revert` のいずれか。
- scope は省略できる。書く場合は `commitlint.config.mjs` の `SCOPES` から選ぶ (クレートは `ugoe-` を除いた名前。`ugoe` クレートは `sdk`)。新しいクレートを作ったら `SCOPES` にも追加する。
- タスクに関するコミットは、件名の末尾にタスク ID を付ける: `(T0104)`。
- 件名は 100 文字以内。命令形・小文字始まり・末尾にピリオドを付けない。
- Git フックは lefthook で管理する (`lefthook.yml`)。`pnpm install` で有効になる。
  - pre-commit: rustfmt と Biome による自動整形、依存方向の検査、タスクファイルの検査
  - commit-msg: commitlint によるメッセージの検査
  - pre-push: clippy、全テスト、svelte-check
- **`--no-verify` でフックを飛ばしてはならない。** フックが失敗したら原因を直す。CI でも同じ検査を行う。
- メッセージは事前に `just commitlint "feat(cli): add render command"` で検査できる。

## マルチエージェント作業の要点

詳細は [docs/workflow/multi-agent.md](docs/workflow/multi-agent.md)。

- 作業は `docs/tasks/` のタスク単位で行う。1 タスク = 1 ブランチ = 1 PR。
- 他のエージェントと並行するときは git worktree で作業する。worktree は `just wt-add T0123` で作る (`.worktrees/T0123` とブランチ `task/T0123-<slug>` を作り、main の `target/` のコピーと `pnpm install --offline` まで行う)。片付けは `just wt-remove T0123 --delete-branch`。
- タスクファイルの `crates:` に書かれた範囲の外を編集しない。範囲外の変更が必要になったら、タスクファイルの「メモ」に書いて止まるか、新しいタスクを起票する。
- `ugoe-core` / `ugoe-model` の公開 API 変更は影響が大きい。「インターフェースだけを先に入れる」小さな PR に分ける。
- コミットの規約は下の「コミット」を参照。
