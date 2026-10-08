# 0034: スタジオの GPU の優先度を、`unsafe` を許す専用のクレートで Windows でだけ下げる

- 状態: Accepted
- 日付: 2026-10-08
- 関連: T0861 (この ADR)、ADR 0028 の 8. (`unsafe` の例外は仮想カメラの 1 クレートだけ、を置き換える)、T0076 (調べた結果)、
  T0857 (変わったときだけ描く)、T0860 (使う GPU と機械ごとの設定ファイル)、[studio.md](../studio.md#gpu-の優先度)

## 背景

利用者の要望 (2026-10-08): ゲームや OBS の描画を邪魔しないよう、スタジオの描画の優先度を下げたい。

GPU の時間の取り合いを OS の側で調整できるのは、調べた範囲では Windows だけ (WDDM の GPU のスケジューラの、プロセスごとの優先度のクラス
`D3DKMTSetProcessSchedulingPriorityClass`。gdi32.dll。
[文書](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/d3dkmthk/nf-d3dkmthk-d3dkmtsetprocessschedulingpriorityclass))。
macOS (Metal) と Linux (WebKitGTK・ネイティブの Vulkan) には、普通のアプリがプロセスの GPU の優先度を下げる口が無い。

スタジオの GPU の処理は、スタジオのプロセス (仮想カメラのネイティブの wgpu) と、WebView2 の GPU のプロセス (`msedgewebview2.exe`。
出力ウィンドウと操作ウィンドウ) にある。後者の番号は WebView2 の `ICoreWebView2Environment8::GetProcessInfos` で知る。どちらの呼び出しも
`unsafe` (Windows の関数と COM のメソッド) が要るが、ADR 0028 の 8. は `unsafe` の例外を `ugoe-ffi` と仮想カメラの 1 クレートだけに許し、
「新しい例外を作らない」と決めていた。

案は 4 つあった (T0861)。

- A. 仮想カメラのクレート (`apps/studio/vcam`) に GPU の優先度の口を置き、ADR 0028 の 8. に追記する。
- B. `unsafe` を許す新しいクレートを作り、ここに GPU の優先度の口を置く (例外のクレートが 1 つ増える)。
- C. 自分のプロセス (仮想カメラ) だけ下げる (出力ウィンドウは下がらない)。
- D. 実装しない (描く量を減らす T0857・T0858 と、案内だけにする)。

利用者の決定 (2026-10-08): 案 B。GPU の優先度だけを受け持つ新しいクレートにする。既定は通常の優先度。

## 決定

1. **新しいクレート `ugoe-studio-gpu-priority` (`apps/studio/gpu-priority`):** GPU の優先度だけを受け持ち、`unsafe` を許す。
   ワークスペースで `unsafe` を許すのは `ugoe-ffi`・仮想カメラのクレート (ADR 0028 の 8.)・このクレートの 3 つになる
   (ADR 0028 の 8. の「例外は仮想カメラの 1 クレートだけ」「新しい例外を作らない」を、この 1 つについて置き換える)。
   このクレートは仮想カメラのクレートと同じ規則を守る。
   - `unsafe` は Windows の呼び出し (`D3DKMTSetProcessSchedulingPriorityClass`・`OpenProcess`・`CloseHandle`・WebView2 の COM) に閉じ込め、
     外へは安全な関数 (`set_current_process_priority`・`set_process_priority`・`webview2::gpu_process_ids`・`webview2::watch_gpu_processes`)
     だけを出す。`unsafe` のブロックには前提を `// SAFETY:` で書く (`undocumented_unsafe_blocks = "deny"`)。
   - Windows の外では OS を呼ぶコードをビルドしない (`#[cfg(windows)]`。優先度の型とエラーの型だけを出す)。
   - ワークスペースの lint (`unsafe_code = "deny"`) は、このクレートの `Cargo.toml` だけで上書きする。
   - 内部のクレートに依存しない。ライセンスは GPL-3.0-only (スタジオの一部)。
   - ほかの OS の境界 (App Nap など) はこのクレートに入れない (GPU の優先度だけにして、`unsafe` の範囲を小さく保つ)。
2. **依存:** Windows でだけ `windows` (`Wdk_Graphics_Direct3D`・`Win32_Foundation`・`Win32_System_Threading` の機能) と `webview2-com` (MIT。
   WebView2 の COM の束縛。wry (Tauri) が使っている版と同じにし、Tauri から受け取る `ICoreWebView2Environment` と同じ型にする) を使う。
   どちらもワークスペースの依存に既にあった (`webview2-com` は wry の推移的な依存) ので、ビルドするクレートは増えない。
3. **選べる優先度:** 「通常」(`NORMAL`。既定。OS を呼ばない) と「低い」(`BELOW_NORMAL`)。`IDLE` (出力が止まって見えるほど遅れうる) と、
   上げるもの (`ABOVE_NORMAL` 以上。管理者の権限が要る) は選ばせない。CPU の優先度は下げない (指揮役の刻みが遅れる)。
4. **当てる先:** スタジオのプロセスと WebView2 の GPU のプロセス。WebView2 が GPU のプロセスを作り直したら (`ProcessInfosChanged`) 当て直す。
   当てられなかった理由はエラーのコードで操作ウィンドウに出す (ADR 0029)。設定は機械ごとの設定ファイル (T0860) に持つ。
5. **macOS と Linux:** 優先度は扱わない (パネルにも出さない)。GPU を譲るのは描く量を減らすこと (T0857・刻みの速さ) で行う。

## 結果

- `unsafe` の例外のクレートが 3 つになる。AGENTS.md の規則と、コードレビューの観点 ([code-review.md](../workflow/code-review.md)) の
  `unsafe` の行にこのクレートを足した。Windows のコードは `just cross-check gpu-priority` (x86_64-pc-windows-msvc の clippy) で検査する。
- 仮想カメラのクレート (Frame Server が読み込む DLL にもなる) に、GPU の優先度のコードと `webview2-com` の依存が入らない。
- ほかのプロセス (WebView2 の GPU のプロセス) の設定を変える。WebView2 の環境 (データの場所) はスタジオ専用なので、ほかのアプリの WebView2 には
  効かない。操作ウィンドウの描画も同じプロセスなので一緒に下がる。
- 効き目 (ゲームの fps・OBS のフレームの欠落・スタジオの出力の落ち方) と、`PROCESS_SET_INFORMATION` で WebView2 の GPU のプロセスを開けるかは、
  Windows の実機でしか確かめられない ([studio.md](../studio.md#実機で確かめていないこと)、[実機の確認表](../manual-check.md) の S35)。
- 見直すきっかけ: 実機で効き目が無いと分かったとき (クレートごと外す)、wgpu が Vulkan の `VK_EXT_global_priority` などのキューの優先度を
  出したとき (ネイティブの描画はそちらで下げられる)、ほかの OS に同じ口ができたとき。
