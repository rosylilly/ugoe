---
id: T0727
title: ビューアの CSP を決め、コマンドが読み書きできるパスを絞る
status: todo
milestone: M9
crates: [apps/viewer]
depends_on: [T0724]
assignee:
---

# T0727: ビューアの CSP を決め、コマンドが読み書きできるパスを絞る

## 目的

コードレビュー (T0724) で、ビューアの webview を守るものが薄いことが分かった。

- `apps/viewer/src-tauri/tauri.conf.json` の `app.security.csp` が `null` (CSP なし)。
- アプリのコマンド (`read_model_file`・`write_json_file`・`write_png_file`・`initial_file`) は、権限 (capabilities) で絞っていない
  (`build.rs` で `AppManifest` を使っていないので、どのウィンドウからも呼べる)。
- `read_model_file` は拡張子が `.ugoe`・`.json`・`.ugoemod` ならどのパスでも読み、`write_json_file` は `.json` ならどのパスにも書く。
  webview で任意のスクリプトが動けば、利用者のホームの `.json` (ほかのアプリの設定など) を読み・上書きできる。

いまは同梱のページだけを読み、`{@html}` も使っていないので、すぐに突ける穴は見つけていない。エディタ (T0685・T0686) と
スタジオ (T0819・T0820) の同じ形の所見と揃えて、守りを足す。

## 受け入れ条件

- [ ] `csp` を決める (wasm・WebGPU/WebGL・MediaPipe の wasm とモデル・`blob:`・同梱の `samples/` が動くもの。
  `'wasm-unsafe-eval'` など要るものを README に書く)
- [ ] アプリのコマンドを権限で許すものにする (`tauri_build::Attributes::app_manifest` と capabilities の `allow-*`)
- [ ] 書き込み (`write_json_file`・`write_png_file`) は、保存のダイアログで選んだパスだけを受ける
  (ダイアログをバックエンドで開く、またはバックエンドが選んだパスを覚えて照らし合わせる)。
  読み込みは、ダイアログ・ドロップ・起動時の引数・覚えた場所 (`path:`) で来るパスに限るかを決め、決めたことを README に書く
- [ ] フェイストラッキング (カメラ)・画像の保存・ドロップ・見本・ライセンスの画面が、手で確かめて動く
- [ ] `just ci` が通る

## 参考

- T0724 のメモの所見の表
- エディタの T0685・T0686、スタジオの T0819・T0820 (同じ所見)
- https://v2.tauri.app/security/csp/ ・ https://v2.tauri.app/security/capabilities/

## メモ
