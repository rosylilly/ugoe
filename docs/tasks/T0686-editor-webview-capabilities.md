---
id: T0686
title: "エディタ: WebView の CSP とコマンドの許可を絞る"
status: done
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

- [x] CSP を付ける (`ugoe-wasm` の wasm (`wasm-unsafe-eval`)・MediaPipe の wasm とモデル (`public/mediapipe/`)・Worker・blob の画像・
      Tauri の IPC・開発時の `devUrl` が動くこと)
- [x] アプリのコマンドを `tauri_build::Attributes::app_manifest` (`AppManifest::commands`) と capabilities で明示する
      (将来ウィンドウを足したときに、既定ですべてを許さないように)。要らなければ、要らない理由を docs/editor.md に書く
- [x] 実機 (`just editor` と T0006 の e2e) で、ビューポート・プレビュー (フェイストラッキング)・スクリーンショット・AI の設定が動く
- [x] `just ci` が通る

## 参考

- `apps/editor/src-tauri/tauri.conf.json`・`capabilities/default.json`・`build.rs`
- スタジオの同じ所見: T0820

## メモ

- T0008: 3 つのアプリに同じ所見がある (同期のコマンド: T0685・T0726・T0819、CSP とコマンドの許可: T0686・T0727・T0820、一時ファイル経由の保存: T0687・T0728)。
  先に終えたタスクの形 (非同期にする関数・CSP の中身・一時ファイルの書き方) に後のタスクが揃える (docs/workflow/code-review.md#m9-の結果)。
- T0686 (やったこと): スタジオ (T0820) とビューア (T0727) の形に揃えた。
  - CSP は T0820 と同じ値に `frame-ancestors 'none'` (T0727) を足したもの (`default-src 'self'`・`script-src 'self' 'wasm-unsafe-eval'`・
    `style-src 'self' 'unsafe-inline'` + `dangerousDisableAssetCspModification: ["style-src"]`・`img-src 'self' blob: data:`・
    `media-src 'self' blob: mediastream:`・`font-src 'self' data:`・`connect-src ipc: http://ipc.localhost 'self'`・`worker-src 'self' blob:`・
    `object-src 'none'`・`base-uri 'self'`・`form-action 'none'`)。`index.html` のインラインのスクリプトは Tauri が hash を足すので動く。
  - コマンドは `src-tauri/src/acl.rs` の `COMMANDS` (T0820 と同じく build.rs が `include!`) を `AppManifest::commands` に渡し、
    `capabilities/default.json` (`main` だけ) が 76 個すべてを許す。`acl.rs` のテストが `generate_handler!`・`COMMANDS`・capability・CSP の要の値を検査する。
  - 文書: docs/editor.md#webview-の-csp-とコマンドの許可、apps/editor/README.md。
- 確かめたこと (Linux・WebKitGTK、`just e2e --only editor` の同梱したページのデバッグビルド): 既存の e2e 5 つが通る。加えて一時的な手順
  (コミットしていない) で、CSP が効いている (`eval` と `https://example.com` への fetch が拒まれ、違反はその 2 つだけ)・
  スクリーンショットのボタンで PNG が書かれる (`write_png_file`)・プレビューに入りトラッキングを画像 (blob の URL) で始めると MediaPipe の
  スクリプト・wasm・モデルを読んで検出が動く (状態 running・"No face found")・AI の設定の画面と `ai_settings`・ライセンスの文の fetch が動く、を確かめた。
- 確かめていないこと: 本物のカメラ (`getUserMedia`。CSP は `srcObject` に掛からない)・`just editor` の開発モード (デスクトップの Tauri は
  `devUrl` のページに CSP を付けないので CSP の影響は無く、コマンドの許可は e2e と同じ)・Windows (WebView2) と macOS (WKWebView)。
- 起票の候補: e2e の筋書きに「CSP の違反が無い」(`securitypolicyviolation` を集める) とプレビューのトラッキング (画像で MediaPipe を読む) の
  手順を足す (tests/e2e は `crates:` の外なので入れなかった。3 つのアプリで共通にできる)。
