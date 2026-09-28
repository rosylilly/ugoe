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

- [x] ビューポートにモデルが表示される (WebGPU が使える環境では WebGPU、使えない環境では WebGL2。どちらで描いているかは開発者ツールのコンソールに出る)
- [ ] File > Import PSD… で `assets/fixtures/layers.psd` を取り込み、ビューポートとツリーに表示される (T0605)
- [ ] File > Save As / Open / Export が動き、`.ugoeproj` を開き直すと同じ状態に戻る (T0604)
- [ ] ツリーでの選択・名前の変更・ドラッグでの付け替え (見た目が変わらない) (T0602, T0607)
- [ ] ビューポートでのメッシュの選択と、メッシュ編集モード (頂点の移動・追加・削除) (T0606)
- [ ] デフォーマで包む操作と、ワープの制御点・回転のギズモ (T0607)
- [ ] パラメータの作成・キーの追加・キーの上での形の編集、斜めのセルが自動で推定されること (T0608, T0617)
- [ ] ミラー・深度のプリセット・深度からの回転 (T0613, T0617)
- [ ] Undo / Redo
- [ ] Windows (WebView2)・macOS (WKWebView)・Linux (WebKitGTK) のうち、手元で使える環境
  - Windows・macOS 向けは、GitHub Actions の Editor build の成果物 (インストーラ) をダウンロードするか、
    その OS の上でビルドする (WSL の中では Linux 向けしかできない)。手順は
    [apps/editor/README.md の「各 OS でのビルド」](../../apps/editor/README.md#各-os-でのビルド)。
    署名していないので、SmartScreen や Gatekeeper の警告を越えて起動する (同 README の「配布物の署名」)。

## メモ

### 2026-09-28 WSL2 (Linux, WebKitGTK) での確認 (利用者)

- Tauri のエディタが起動し、WebGL2 で描画された。
- `assets/fixtures/layers.psd` を取り込めたが、表示が虹色のテクスチャになった。
  - 原因: このテスト用 PSD は自作の書き出し処理で作った、位置から色を決める模様 (`crates/ugoe-psd/tests/common/mod.rs` の `pattern()`) の絵で、表示は正しい。ただし確認用の絵として不適切。
  - Photoshop で開くと「対応していない形式」になった。取り込みのエラー処理を試すための偽の中身 (未対応の圧縮方式、偽のレイヤー効果・調整レイヤー) と不正確な合成画像を含み、仕様に沿っていないためと考えられる (原因の特定は T0620)。
  - ugoe の PSD の読み込みは、自作のファイルと ImageMagick の出力でしか検証されていない。 → T0620
- Windows・macOS 向けのビルドのコマンドがなく未確認。 → T0621
