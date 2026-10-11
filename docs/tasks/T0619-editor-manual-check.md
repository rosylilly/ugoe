---
id: T0619
title: エディタの実機での動作確認 (手動)
status: done
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

> 2026-10-03: 残りの項目は [実機の確認表](../manual-check.md) の「E. エディタ」に吸収した (T0007)。以後の結果はそちらに書く。

- [x] ビューポートにモデルが表示される (WebGPU が使える環境では WebGPU、使えない環境では WebGL2。どちらで描いているかは開発者ツールのコンソールに出る)
- [ ] File > Import PSD… で `assets/fixtures/layers.psd` を取り込み、ビューポートとツリーに表示される (T0605)
- [x] File > Import PSD… で `assets/fixtures/character.psd` を取り込み、キャラクターの絵 (髪・顔・両目・頬・口・服) が崩れずに表示され、ツリーが `体` / `頭` / `目` のグループになる (T0620)
- [x] Photoshop (あれば) で `assets/fixtures/layers.psd` と `assets/fixtures/character.psd` を開ける。レイヤーパネルのグループ・クリッピング・非表示・不透明度が `assets/fixtures/README.md` の説明どおりか。開けなければエラーの文言を T0620 のメモに書く (T0620)
- [x] File > Save As / Open / Export が動き、`.ugoeproj` を開き直すと同じ状態に戻る (T0604)
- [ ] ツリーでの選択・名前の変更・ドラッグでの付け替え (見た目が変わらない) (T0602, T0607)
- [x] ビューポートでのメッシュの選択と、メッシュ編集モード (頂点の移動・追加・削除) (T0606)
- [ ] デフォーマで包む操作と、ワープの制御点・回転のギズモ (T0607)
- [ ] パラメータの作成・キーの追加・キーの上での形の編集、斜めのセルが自動で推定されること (T0608, T0617)
- [ ] ミラー・深度のプリセット・深度からの回転 (T0613, T0617)
- [x] Undo / Redo
- [ ] Windows (WebView2)・macOS (WKWebView)・Linux (WebKitGTK) のうち、手元で使える環境
  - Windows・macOS 向けは、GitHub Actions の Editor build の成果物 (インストーラ) をダウンロードするか、
    その OS の上でビルドする。Windows 向けは WSL2 から `just editor-build-windows` でもクロスビルドできる
    (NSIS のインストーラと実行ファイル。T0625)。手順は
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

### 2026-09-28 WSL2 での 2 回目の確認 (利用者)

- `layers.psd` と `character.psd` を Photoshop でもエディタでも開けた (T0620 で解決)。
- Import PSD で警告が出た: `the PSD canvas (320x400) differs from the model canvas (256x256); layers keep their PSD pixel positions`。
  起動時にサンプルのモデル (256x256) を開いているため、PSD がそこに追加された。 → T0623
- Save As → Open、メッシュ編集、Undo / Redo は正常に動いた。
- パラメータの作成などの操作方法が分からなかった。操作マニュアルが欲しい。 → T0624
- Windows 向けのビルドは WSL2 上でもできるはず (署名なし)。 → T0625

### 2026-09-28 Windows での確認 (利用者)

- WSL2 からクロスビルドした NSIS のインストーラ (`just editor-build-windows`、T0625) で Windows にインストールし、起動できた。
- `assets/fixtures/character.psd` の読み込みまで成功した (WebView2)。


### 閉じた (T0083、2026-10-11)

- 残りの確認項目は T0007 で [実機の確認表](../manual-check.md) の「E. エディタ」(E01〜E15) と U05 (WebView2・WKWebView) に移し、
  以後の結果はそちらに書く流れになっているので、このタスクは「確認表に移した」として done にした (確認そのものは済んでいない)。
  チェックの無い項目は、確認表の E03〜E07 の結果の欄で追う。
