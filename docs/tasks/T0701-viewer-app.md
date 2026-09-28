---
id: T0701
title: ビューアアプリの土台 (apps/viewer)
status: in-progress
milestone: M3
crates: [apps/viewer, Cargo.toml, pnpm-workspace, justfile, scripts, commitlint.config.mjs, .github/workflows, docs]
depends_on: []
assignee: claude-worker-T0701
---

# T0701: ビューアアプリの土台 (apps/viewer)

## 目的

`.ugoe` を開いて表示し、パラメータを動かして確かめるだけの軽い Tauri v2 アプリ `apps/viewer` を作る。
エディタ (編集) とは別に、ランタイムを組み込むアプリの見本と、フェイストラッキング (T0703) の確認の場にする。

## 方針

- **ライセンスは MIT** (ADR 0018 で GPL はエディタだけ)。エディタ (GPL) のコードを写さない。共有したい部品があれば、MIT の場所 (`packages/` など) に切り出すタスクを起票する。
- 描画は `ugoe-wasm` の `WebPuppet` を webview で使う (エディタのビューポートと同じ仕組み、ADR 0003)。Rust 側 (Tauri) はファイルの読み込みなど最小限にし、依存はランタイム層だけにする (`scripts/check-layers.mjs` に `ugoe-viewer` を登録)。
- フロントエンドは Svelte 5 + Vite (エディタと同じ道具)。

## 受け入れ条件

- [ ] `apps/viewer` (Tauri v2、crate 名 `ugoe-viewer`、npm パッケージ `@ugoe/viewer`、`license = "MIT"`) がワークスペースに入り、`just viewer` (開発) と `just viewer-build` がある
- [ ] `.ugoe` (と `model.json`) を開く (OS のダイアログ、ドラッグ・アンド・ドロップ)。表示、Fit・ズーム・パン、背景色
- [ ] パラメータのスライダーの一覧 (グループごと)、既定値に戻す
- [ ] ヘッドレスのブラウザで確かめられるよう、Tauri の呼び出しを差し替えられる
- [ ] `just ci` (Biome・svelte-check・Vitest を含む)・`just licenses`・`just layers` が通る。CI のエディタのビルドのワークフローにビューアも入れるか決めて書く
- [ ] `apps/viewer/README.md` と `docs/architecture.md`・`AGENTS.md` の構成の説明を更新する。commitlint の scope に `viewer` を足す

## 参考

- `apps/editor/` (構成の参考。コードは写さない)
- `docs/adr/0003-editor-viewport-in-webview.md`、`docs/adr/0018-licensing-and-1.0-scope.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
