---
id: T0820
title: "スタジオ: WebView の CSP とウィンドウごとのコマンドの許可を絞る"
status: todo
milestone: M9
crates: [apps/studio, docs]
depends_on: [T0814]
assignee:
---

# T0820: スタジオ: WebView の CSP とウィンドウごとのコマンドの許可を絞る

## 目的

T0814 のレビューで見つけた (多層の防御)。スタジオの `tauri.conf.json` は `app.security.csp` が `null` で、アプリのコマンド
(`build.rs` の `tauri_build::build()`) はウィンドウを問わずすべて呼べる。出力ウィンドウ (受け手だけ) からも `api_token`・
`regenerate_api_token`・`approve_api_client`・`open_profile` などを呼べてしまう。ページは同梱したものだけで、外部 API の
クライアントの名前は Svelte が文字として出すので、今すぐ悪用できる穴ではないが、出力ウィンドウやページに何かを差し込まれたときの
被害を小さくしたい。

## 受け入れ条件

- [ ] 操作ウィンドウと出力ウィンドウに CSP を付ける (wasm・blob の画像・Tauri の IPC が動くこと)
- [ ] `tauri_build::Attributes::app_manifest` (`AppManifest::commands`) と capabilities で、出力ウィンドウが呼べるコマンドを受け手に要るもの
      (`subscribe_frames`・`scene_model`・`scene_module`・`scene_background`・`report_render_stats` など) に絞る
- [ ] エディタ・ビューアにも同じことが要るかを確かめ、要るなら起票する
- [ ] `just ci` が通る (T0006 の e2e を含む)

## 参考

- `apps/studio/src-tauri/tauri.conf.json`・`capabilities/`・`build.rs`
- ADR 0028 の 6.

## メモ
