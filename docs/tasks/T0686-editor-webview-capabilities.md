---
id: T0686
title: "エディタ: WebView の CSP とコマンドの許可を絞る"
status: in-progress
milestone: M9
crates: [apps/editor, docs]
depends_on: [T0669]
assignee: claude-worker-T0686
---

# T0686: エディタ: WebView の CSP とコマンドの許可を絞る

## 目的

T0669 のレビューで見つけた (多層の防御)。エディタの `tauri.conf.json` は `app.security.csp` が `null` で、アプリのコマンド
(`build.rs` の `tauri_build::build()`) はすべて呼べる。ページは同梱したものだけで、PSD のレイヤー名・モデルの名前・エージェントの
提案の題名と理由は Svelte が文字として出すので、今すぐ悪用できる穴ではない。ただ、ページに何かを差し込まれると、
`get_mcp_bridge_token` (エディタの文書を編集できる MCP のトークン)・`write_json_file` / `write_png_file` (任意の場所の `.json`・`.png`)・
`read_module_file`・`store_api_key` などをそのまま呼べる。

ウィンドウは `main` の 1 つだけなので、スタジオ (T0820) と違ってウィンドウごとに絞る必要は今は無い。

## 受け入れ条件

- [ ] CSP を付ける (`ugoe-wasm` の wasm (`wasm-unsafe-eval`)・MediaPipe の wasm とモデル (`public/mediapipe/`)・Worker・blob の画像・
      Tauri の IPC・開発時の `devUrl` が動くこと)
- [ ] アプリのコマンドを `tauri_build::Attributes::app_manifest` (`AppManifest::commands`) と capabilities で明示する
      (将来ウィンドウを足したときに、既定ですべてを許さないように)。要らなければ、要らない理由を docs/editor.md に書く
- [ ] 実機 (`just editor` と T0006 の e2e) で、ビューポート・プレビュー (フェイストラッキング)・スクリーンショット・AI の設定が動く
- [ ] `just ci` が通る

## 参考

- `apps/editor/src-tauri/tauri.conf.json`・`capabilities/default.json`・`build.rs`
- スタジオの同じ所見: T0820

## メモ

- T0008: 3 つのアプリに同じ所見がある (同期のコマンド: T0685・T0726・T0819、CSP とコマンドの許可: T0686・T0727・T0820、一時ファイル経由の保存: T0687・T0728)。
  先に終えたタスクの形 (非同期にする関数・CSP の中身・一時ファイルの書き方) に後のタスクが揃える (docs/workflow/code-review.md#m9-の結果)。
