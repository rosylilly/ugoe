---
id: T0619
title: エディタの実機での動作確認 (手動)
status: todo
milestone: M2
crates: []
depends_on: [T0617, T0618]
assignee:
---

# T0619: エディタの実機での動作確認 (手動)

## 目的

M2 の機能は、コマンドによる自動テスト (T0616) と、ヘッドレスの Chrome (WebGL2、Tauri の IPC を模擬または実際の `Document` に中継) で確かめた。
実際の Tauri のウィンドウ (WebView) では、まだ誰も動かしていない。人が画面で動作を確かめ、見つかった問題をタスクにする。

## 手順

```sh
just setup
cargo install wasm-bindgen-cli --version "$(just wasm-bindgen-version)" --locked   # 初回のみ
just editor
```

## 確認項目

- [ ] ビューポートにモデルが表示される (WebGPU が使える環境では WebGPU、使えない環境では WebGL2。どちらで描いているかは開発者ツールのコンソールに出る)
- [ ] File > Import PSD… で `assets/fixtures/layers.psd` を取り込み、ビューポートとツリーに表示される (T0605)
- [ ] File > Save As / Open / Export が動き、`.ugoeproj` を開き直すと同じ状態に戻る (T0604)
- [ ] ツリーでの選択・名前の変更・ドラッグでの付け替え (見た目が変わらない) (T0602, T0607)
- [ ] ビューポートでのメッシュの選択と、メッシュ編集モード (頂点の移動・追加・削除) (T0606)
- [ ] デフォーマで包む操作と、ワープの制御点・回転のギズモ (T0607)
- [ ] パラメータの作成・キーの追加・キーの上での形の編集、斜めのセルが自動で推定されること (T0608, T0617)
- [ ] ミラー・深度のプリセット・深度からの回転 (T0613, T0617)
- [ ] Undo / Redo
- [ ] Windows (WebView2)・macOS (WKWebView)・Linux (WebKitGTK) のうち、手元で使える環境

## メモ

(確認した環境と結果、見つかった問題と起票したタスクを書く)
