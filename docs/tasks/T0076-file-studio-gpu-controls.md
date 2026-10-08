---
id: T0076
title: スタジオの GPU の設定 (使う GPU を選ぶ・ほかのアプリに譲る・描く量で負荷を抑える) を調べてタスクにする
status: done
milestone: M8
crates: [docs]
depends_on: []
assignee: claude-worker-T0076
---

# T0076: スタジオの GPU の設定 (使う GPU を選ぶ・ほかのアプリに譲る・描く量で負荷を抑える) を調べてタスクにする

## 目的

利用者の要望 (2026-10-08): スタジオで GPU の優先度を変えられるようにしたい。利用者が選んだ中身は次の 3 つ。

1. **使う GPU を選ぶ:** ノート PC などで、外付け GPU (高性能) と内蔵 GPU (省電力) のどちらで描くかを選ぶ。
2. **ほかのアプリに GPU を譲る:** ゲームや OBS の描画を邪魔しないよう、スタジオの描画の優先度を下げる。
3. **描く量で負荷を抑える:** フレームレートの上限・描く解像度・アンチエイリアスなどを下げ、GPU の使用量そのものを減らす。

スタジオの描画が今どこで行われているか (WebView の中の wasm の WebGL2 / WebGPU、出力ウィンドウ、ブラウザソース、仮想カメラの読み戻し) によって、
できることが OS と WebView (Windows の WebView2・macOS の WKWebView・Linux の WebKitGTK) ごとに変わる。調べてから、実現できる形でタスクにする。

## 受け入れ条件

- [x] 今のスタジオの描画の経路 (どのウィンドウ・どの API (WebGL2 / WebGPU)・アダプタの選び方・読み戻し) と、すでにある設定 (「フレームレートと負荷」のパネルなど) を調べてメモに書いた
- [x] 3 つの中身ごとに、OS と WebView ごとに取れる手段 (例: WebGPU の `powerPreference`・WebGL の `powerPreference` の属性・Windows のアプリごとのグラフィックスの設定・プロセスの優先度・D3D12 / Metal の優先度・WebView2 の起動の引数・フレームレートの上限・描く解像度の倍率) と、
      効く範囲・制約・要る依存や unsafe を、出典 (公式の文書など) の URL と一緒にメモに書いた
- [x] 実現できるものを docs/tasks/README.md の形でタスクにした (milestone は M8 の後続。受け入れ条件は検証できる形。実機でしか確かめられないことは「確かめていないこと」として分ける)
- [x] 依存の追加・unsafe・OS の設定を書き換える操作・ADR が要るものは、本文に「判断が要る」と書いて案と推奨を並べた
- [x] docs/roadmap.md の M8 の後続のタスクに起票した ID を書いた
- [x] `node scripts/tasks.mjs` と `just ci` が通る

## 参考

- docs/studio.md、docs/adr/0028-studio.md、apps/studio、packages/player、crates/ugoe-wasm、docs/portability.md
- リリース前なので、移行の説明や互換の別名は要らない (利用者の方針)

## メモ

- 利用者の要望で起票した (2026-10-08)。

### 今の描画の経路 (2026-10-08 に調べた)

| 受け手 | プロセス | API とアダプタの選び方 | 描く回数 | 読み戻し |
| --- | --- | --- | --- | --- |
| 出力ウィンドウ | WebView (Windows は WebView2 の `msedgewebview2.exe` の GPU のプロセス、macOS は WebKit の GPU のプロセス、Linux は WebKitGTK) | `@ugoe/player` の `PuppetSurface` が `WebPuppet.create(canvas, "auto")` で作る。WebGPU が使えればそれ、無ければ WebGL2 (`crates/ugoe-wasm/src/web.rs`)。`request_adapter` は `RequestAdapterOptions` を既定のまま (電力の希望なし)。WebGL2 は wgpu-hal が `getContext("webgl2", { antialias: false })` で作り、`powerPreference` を渡さない (wgpu-hal 30 の gles/web.rs) | `requestAnimationFrame` ごとに **新しいフレームが無くても** `render()` (`player.svelte.ts` の `#tick`)。表示はリフレッシュレート、新しい形は刻みの速さ (30・60・120) | なし (ウィンドウの取り込みは OBS が行う) |
| ブラウザソース | OBS のプロセス (CEF) | 出力ウィンドウと同じページ (`source.html`)。アダプタは OBS (Chromium) が決める | 出力ウィンドウと同じ (OBS のソースのフレームレートの設定にも従う) | なし |
| 仮想カメラ | スタジオのプロセス | `ugoe-headless` の `Gpu::new(None)`。電力の希望は環境変数 `WGPU_POWER_PREF` だけ。`PowerPreference::None` では wgpu は列挙の順のまま選ぶ (wgpu-core 30 の instance.rs) | 仮想カメラのフレームレート (30・60) の回数だけ (`virtual_camera.rs`) | GPU で NV12 / YUY2 に変換して 3 枚を回して待たずに読み戻す |
| 操作ウィンドウ | 出力ウィンドウと同じ WebView の GPU のプロセス | モデルは描かない (UI と MediaPipe のトラッキング) | — | — |

すでにある設定: 刻みの速さ (30・60・120。プロファイルの `output.tickRate`。「フレームレートと負荷」)、出力ウィンドウの大きさのプリセット (物理ピクセル)、
仮想カメラの解像度 (1920×1080・1280×720) とフレームレート (30・60)。「フレームレートと負荷」は 1 刻みの時間・受け手ごとの描画の回数と
1 回の時間 (CPU の時間。GPU の実行の時間は含まない)・捨てたフレームを出す (`apps/studio/src/lib/load.ts`・`LoadPanel.svelte`)。
アンチエイリアスは使っていない (レンダラは `sample_count: 1`、WebGL2 は `antialias: false`) ので、下げる設定は無い。
`present_mode` は `Fifo`、`desired_maximum_frame_latency` は 2。

### 1. 使う GPU を選ぶ

| OS / WebView | 手段 | 効く範囲・制約 | 要るもの |
| --- | --- | --- | --- |
| すべて (WebGPU) | `requestAdapter({ powerPreference })` (wgpu の `PowerPreference` がそのまま渡る) | 希望 (hint)。Windows の Chrome は無視し、ブラウザ全体で 1 つのアダプタ (ノート PC ではたいてい内蔵) を使う。2 つの GPU の macOS の Chrome は、希望が無ければ電源につながっているとき外付けを返す | ugoe-wasm に引数を足す (T0499) |
| すべて (WebGL2) | `getContext("webgl2", { powerPreference })`。同じ canvas の 2 回目からの `getContext` は最初のコンテキストを返し属性を無視するので、ugoe-wasm が wgpu より先に作れば渡せる | 希望。Chrome 80 から既定は `low-power`。Chromium の GPU の切り替えは Mac だけ (`gpu_switching.cc` は `IS_MAC` の中) | ugoe-wasm (T0499) |
| Windows (WebView2) | 起動の引数 `--force-high-performance-gpu` (`chrome://flags/#force-high-performance-gpu` と同じスイッチ) | WebView2 の環境ごと (スタジオの WebView だけ)。次の起動から。2 つのウィンドウの引数を揃える必要がある。WebView2 で効くかは未確認 | ウィンドウをコードで作る (T0860) |
| Windows | アプリごとのグラフィックスの設定 (設定 → ディスプレイ → グラフィック。`ms-settings:display-advancedgraphics`。レジストリ `HKCU\Software\Microsoft\DirectX\UserGpuPreferences` の、実行ファイルのパスの名前に `GpuPreference=1;` (省電力)・`2;` (高性能)) | 実行ファイルごと。`ugoe-studio.exe` に効く (仮想カメラ) が、WebView2 の描画は `msedgewebview2.exe` (Runtime の版ごとにパスが変わり、ほかのアプリと共有) なので出力ウィンドウには効かない。書き換えは OS の設定の書き換え | 書かない (案内とボタンだけ。T0860 の判断 1) |
| すべて (ネイティブ) | wgpu の `PowerPreference` (`LowPower` は内蔵を、`HighPerformance` は外付けを先に並べる) | 仮想カメラ。DX12・Vulkan・Metal で効く | `ugoe-headless` に引数 (T0860) |
| Linux | 環境変数 `DRI_PRIME=1` (Mesa)、NVIDIA の `__NV_PRIME_RENDER_OFFLOAD=1`・`__GLX_VENDOR_LIBRARY_NAME=nvidia`。デスクトップのエントリの `PrefersNonDefaultGPU=true` | プロセスの起動時に決まる (WebKitGTK の子のプロセスは引き継ぐ)。起動後に変えるには起動し直すか `std::env::set_var` (edition 2024 では `unsafe`)。`PrefersNonDefaultGPU` は高性能の希望だけで、ランチャーが対応するときだけ | 案内だけ (T0860 の判断 3) |
| macOS | Apple Silicon は GPU が 1 つ (`MTLDevice.isLowPower` は偽)。Intel の 2 つの GPU の Mac では WebGL / WebGPU の希望と、Metal の機器の選択。`Info.plist` の `NSSupportsAutomaticGraphicsSwitching` | 対象は古い Intel の Mac だけ | 書かない (T0860 の判断 3) |

### 2. ほかのアプリに GPU を譲る

| OS | 手段 | 効く範囲・制約 | 要るもの |
| --- | --- | --- | --- |
| Windows | `D3DKMTSetProcessSchedulingPriorityClass` (WDDM の GPU のスケジューラのプロセスごとのクラス: `IDLE`〜`REALTIME`) | プロセスごと。自分 (仮想カメラ) と、WebView2 の GPU のプロセス (`ICoreWebView2Environment8::GetProcessInfos` で番号が分かる) に当てられる。OBS は管理者として動くと自分のクラスを上げる (配信ソフトの側で譲られる形) | `unsafe` と `windows` の機能 (T0861。判断が要る) |
| Windows (D3D11) | `IDXGIDevice::SetGPUThreadPriority` (-7〜7) | D3D11 のデバイスごと。wgpu は D3D12 で、WebView2 の中のデバイスにも触れないので使えない | — |
| Windows (D3D12) | `D3D12_COMMAND_QUEUE_PRIORITY` | `NORMAL`・`HIGH`・`GLOBAL_REALTIME` だけで、下げる値が無い。wgpu も出していない | — |
| Windows | CPU のプロセスの優先度 (`SetPriorityClass`) | CPU だけ。指揮役の刻みのスレッドが遅れて出力の滑らかさが崩れるので、採らない | — |
| macOS (Metal) | 普通のアプリがプロセスやキューの GPU の優先度を下げる公開の口は見つからなかった | — | — |
| Linux | `VK_EXT_global_priority` (キューの優先度。LOW を選べる) | ネイティブの Vulkan だけで、wgpu は出しておらず、ドライバも限られる。WebKitGTK には口が無い | 採らない |
| すべて | 描く量を減らす (下の 3.) | どの OS でも効き、OBS のブラウザソース (OBS のプロセス) にも効く | T0857・T0858 |

### 3. 描く量で負荷を抑える

| 手段 | 効く範囲・制約 | タスク |
| --- | --- | --- |
| 受け手は新しいフレームか見せ方の変化があったときだけ描く | 出力ウィンドウ・ブラウザソース。高いリフレッシュレートの画面ほど効く (144 Hz・刻み 30 なら描く回数は約 1/5)。描く回数の上限が刻みの速さになるので、「フレームレートの上限」は今の刻みの速さの設定で足りる | T0857 |
| 描く解像度の倍率 (1・0.75・0.5) | 出力ウィンドウ・ブラウザソース。取り込む画は粗くなる。仮想カメラは解像度を既に選べる | T0858 |
| 刻みの速さ・仮想カメラの解像度とフレームレート | すでにある | — |
| アンチエイリアス | 使っていないので下げる余地なし | — |

### 起票したタスク

| ID | 内容 | 判断 |
| --- | --- | --- |
| T0857 | 受け手は変わったときだけ描く | 要らない |
| T0858 | 描く解像度の倍率 (T0857 の後) | 要らない |
| T0499 | ugoe-wasm: `WebPuppet.create` の `powerPreference` とアダプタの名前 | 要らない (省ける引数の追加) |
| T0860 | スタジオ: 使う GPU を選ぶ・受け手ごとの GPU を見せる (T0499 の後) | **要る** (Windows の WebView2 の選び方・設定の置き場所・Linux と macOS の扱い。推奨は起動の引数 + 案内、機械ごとの設定、Linux と macOS は案内だけ) |
| T0861 | スタジオ: Windows で GPU の優先度を下げる (T0857 の後) | **要る** (`unsafe` と WebView2 の GPU のプロセスの設定。推奨は仮想カメラのクレートに置いて ADR 0028 の 8. に追記) |

- 「新しいタスクは起票しない」(ワーカーの共通の指示) と、このタスクの受け入れ条件 (起票する) が食い違うが、このタスクは起票そのものが成果物なので、
  オーケストレーターの補足 (「調べて起票するタスク」) に従って起票した。
- 確かめていないこと: どの手段も実機 (2 つの GPU のノート PC・Windows の WebView2・Intel の Mac・Linux の PRIME) で効くかは確かめていない。
  WSL の上では WebView2・WKWebView・複数の GPU を動かせない。各タスクの「確かめていないこと」に分けて書いた。
- 起票の候補 (起票していない): `@ugoe/runtime` (`<ugoe-avatar>`・`createPuppet`) にも `powerPreference` を出す。Intel の Mac の
  `NSSupportsAutomaticGraphicsSwitching`。Linux で GPU を選んで起動し直す口。
- 気づいたこと: 刻みの速さ 120 を 60 Hz の画面で使うと、出力ウィンドウの描画の回数 (最大 60) が刻みの速さの 95% を下回り、今も「負荷」の注意が出るはず
  (T0857 の受け入れ条件で確かめる)。

### 出典

- WebGPU の `powerPreference`: https://gpuweb.github.io/gpuweb/#dom-gpurequestadapteroptions-powerpreference
- Chrome の WebGPU の文書 (Windows では `powerPreference` が効かない・`#force-high-performance-gpu`・macOS の既定): https://developer.chrome.com/docs/web-platform/webgpu/troubleshooting-tips
- WebGL の `powerPreference` と 2 回目の `getContext`: https://registry.khronos.org/webgl/specs/latest/1.0/#5.2
- Chromium の GPU の切り替え (Mac だけ): https://chromium.googlesource.com/chromium/src.git/+/HEAD/gpu/config/gpu_switching.cc 、
  スイッチ `force-high-performance-gpu`: https://chromium.googlesource.com/chromium/src/+/HEAD/gpu/config/gpu_switches.cc
- WebView2 のプロセス (GPU のプロセス・データの場所ごとのプロセスの組): https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/process-model 、
  `GetProcessInfos`: https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2environment8
- Windows の設定を開く URI (`ms-settings:display-advancedgraphics`): https://learn.microsoft.com/en-us/windows/apps/develop/launch/launch-settings
- `UserGpuPreferences` の値 (Microsoft の公式の文書は見つからず、Microsoft Q&A と解説記事による): https://learn.microsoft.com/en-us/answers/questions/3934761/how-to-force-windows11-to-prioritize-one-gpu-over
- `D3DKMTSetProcessSchedulingPriorityClass`: https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/d3dkmthk/nf-d3dkmthk-d3dkmtsetprocessschedulingpriorityclass 、
  クラス: https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/d3dkmthk/ne-d3dkmthk-_d3dkmt_schedulingpriorityclass
- `IDXGIDevice::SetGPUThreadPriority`: https://learn.microsoft.com/en-us/windows/win32/api/dxgi/nf-dxgi-idxgidevice-setgputhreadpriority
- `MTLDevice.isLowPower` (Apple Silicon では偽): https://developer.apple.com/documentation/metal/mtldevice/islowpower 、
  `NSSupportsAutomaticGraphicsSwitching`: https://developer.apple.com/library/archive/qa/qa1734/_index.html
- Linux の PRIME (環境変数): https://wiki.archlinux.org/title/PRIME 、NVIDIA のオフロード:
  https://download.nvidia.com/XFree86/Linux-x86_64/latest/README/primerenderoffload.html 、
  デスクトップのエントリの `PrefersNonDefaultGPU`: https://specifications.freedesktop.org/desktop-entry-spec/latest/
- `std::env::set_var` が `unsafe` であること (edition 2024): https://doc.rust-lang.org/std/env/fn.set_var.html
- wgpu の `PowerPreference`: https://docs.rs/wgpu/latest/wgpu/enum.PowerPreference.html (並べ替えは wgpu-core 30 の `instance.rs`、
  WebGPU への受け渡しは wgpu 30 の `backend/webgpu.rs`、WebGL2 のコンテキストの属性は wgpu-hal 30 の `gles/web.rs` を読んだ)
