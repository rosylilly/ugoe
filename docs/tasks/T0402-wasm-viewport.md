---
id: T0402
title: wasm の WebPuppet とエディタのビューポート
status: done
milestone: M2
crates: [ugoe-wasm, apps/editor (src/ のみ)]
depends_on: [T0201]
assignee: claude-worker-T0402
---

# T0402: wasm の WebPuppet とエディタのビューポート

## 目的

`ugoe-wasm` に `WebPuppet` (モデルの読み込み、パラメータの設定、canvas への描画) を作り、エディタのビューポートで表示する。

## 受け入れ条件

- [ ] WebGPU と WebGL2 のどちらでも描画できる
- [x] パラメータスライダーの操作が IPC を経由せずにビューポートに反映される
- [x] wasm のビルド手順を justfile に追加する
- [x] `just ci` が通る

## 参考

- `docs/editor.md#ビューポート`
- `docs/adr/0003-editor-viewport-in-webview.md`

## メモ

- **WebPuppet (`crates/ugoe-wasm/src/web.rs`)**: `create(canvas, "auto" | "webgpu" | "webgl2")`、
  `load_json` / `load_bundle` (PNG は Rust でデコードして乗算済みに。JSON は白のプレースホルダ)、
  `set_parameter` / `parameter` / `reset_parameters` / `update(dt)`、`resize` / `set_background` / `render`、
  `fit` / `set_view` / `zoom_at` / `pan_by` / `view_transform` / `canvas_to_screen` / `screen_to_canvas`、
  `drawable_ids` / `drawable_bounds` / `drawable_positions` / `render_order` / `hit_test` (T0606 用)、`is_lost`。
  読み直すと同じ ID のパラメータ値を引き継ぐ。GPU に触れない部分 (`load` / `view` / `hit`) はネイティブでテストしている。
- **ビルド**: `just wasm` (wasm-bindgen-cli は `just wasm-bindgen-version` の版を
  `cargo install wasm-bindgen-cli --version <版> --locked` で入れる。版が違えば `just wasm` が止まる)。
  出力は `apps/editor/src/lib/viewport/generated/` (gitignore)。`tauri dev` / `tauri build` は
  `beforeDevCommand` / `beforeBuildCommand` で `pnpm wasm` (= `just wasm`) を先に実行する。
  `just wasm-check` は wasm32 向けの clippy (`-D warnings`) に変えた (ブラウザ専用の `web.rs` も検査するため)。
- **生成物が無いとき**: フロントエンドは `import.meta.glob` で探すので、ビルドも `svelte-check` も通り、
  ビューポートに「`just wasm` を実行して」と表示する。型は tsconfig の `paths` で generated/ を優先し、
  無ければ手書きの `src/lib/viewport/ugoe-wasm-stub.d.ts` を使う (web.rs の API を変えたら揃える)。
  CI の `web` ジョブは wasm-bindgen-cli (taiki-e/install-action、版は Cargo.lock から) で `just wasm` してから
  check / build するので、CI では本物の型で検査される。
- **フロントエンドの構成**: `App.svelte` はレイアウトだけ。`lib/document.svelte.ts` (EditorDocument: IPC と
  モデルの写し)、`lib/viewport/` (`wasm.ts` 読み込み、`controller.svelte.ts` ViewportController、
  `Viewport.svelte`)、`lib/panels/` (ParametersPanel)。ギズモ等は `Viewport` の `overlay` スニペットに描く
  (`controller.transform` / `canvasToScreen` / `hitTest` は CSS ピクセル)。左ボタンはツール用に空けてあり、
  ビューポート自身はホイールでズーム、中ボタン / Alt+左ドラッグでパン、Fit ボタンだけを持つ。
- **バックエンド**: `get_model_bundle` を 1 つ足した (`src-tauri/src/project.rs`、`tauri::ipc::Response` で
  `ugoe_editor_core::export_bundle` のバイト列を返す)。EditorDocument は refresh のたびに JSON (パネル用) と
  バンドル (ビューポート用、テクスチャ付き) を取り、バンドルが作れないときは JSON (白いテクスチャ) で表示する。
  ファイルメニューの New / Open の後も `doc.refresh()` でビューポートを読み直す。
- **検証したこと**: `cargo test -p ugoe-wasm` (読み込み・PNG の乗算済み化・パラメータ引き継ぎ・ビュー変換・
  ヒットテスト)、`just wasm`、`pnpm --filter @ugoe/editor check` / `build` (生成物の有無の両方)、`just ci`、
  actionlint。ヘッドレスの Chrome 146 (WSL、SwiftShader) で:
  - WebPuppet 単体 (`"webgl2"` と `"auto"` → WebGL2 にフォールバック): バンドルの PNG テクスチャ
    (赤・青) が正しい色で描かれ、`set_parameter` で四角形が動き、JSON で読み直してもパラメータ値が残り、
    `resize` / `zoom_at` / `view_transform` / `hit_test` / `drawable_bounds` が期待どおり。
  - エディタのフロントエンド (Tauri の IPC をモックして `vite preview`): WebGL2 で描画され、
    バンドルのテクスチャ (赤・青) が表示され、パラメータスライダーで四角形が移動し、Undo 後の読み直し
    (バンドル失敗 → JSON へのフォールバック) でもスライダーの値が保たれた (スクリーンショットで確認)。
  - main (T0604・T0207・T0109) に rebase した後で `just ci`・`just wasm`・`pnpm --filter @ugoe/editor build` を再実行した。
- **検証できなかったこと**: WebGPU での描画 (この環境の Chrome は WebGPU の canvas に描くとデバイスを失う。
  wasm を使わない素の JS でも同じなので環境の問題。デバイス・サーフェスの作成までは通った)。
  実際の Tauri の WebView (WebKitGTK / WebView2 / WKWebView) での動作。そのため「WebGPU と WebGL2 の
  どちらでも描画できる」は未チェックのまま (WebGL2 は確認済み)。
- **残した課題**: wasm が約 6 MB (gzip 1.6 MB。wgpu-core と naga を含むため)。サイズが問題になれば
  `opt-level = "z"` の wasm 用プロファイルや wasm-opt を検討する。物理・モーションの再生
  (毎フレーム `update(dt)` して描き続ける) は T0303 で。KTX2 テクスチャは白で代用している。
