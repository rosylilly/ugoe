---
id: T0861
title: "スタジオ: Windows で GPU の処理の優先度を下げ、ゲームや OBS に GPU を譲れるようにする (判断が要る)"
status: todo
milestone: M8
crates: [apps/studio, docs, Cargo.toml, Cargo.lock]
depends_on: [T0857]
assignee:
---

# T0861: スタジオ: Windows で GPU の優先度を下げる (判断が要る)

## 目的

利用者の要望「ゲームや OBS の描画を邪魔しないよう、スタジオの描画の優先度を下げる」(T0076 で調べた)。

GPU の時間の取り合いを OS の側で調整できるのは、調べた範囲では **Windows だけ** (WDDM の GPU のスケジューラの、プロセスごとの優先度のクラス
`D3DKMTSetProcessSchedulingPriorityClass`)。macOS (Metal) と Linux (WebKitGTK・ネイティブの Vulkan) には、普通のアプリがプロセスの GPU の
優先度を下げる口が無い (Vulkan の `VK_EXT_global_priority` はキューの優先度を選べるが、wgpu は出しておらず、ドライバも限られる)。
そのため、macOS と Linux で「譲る」のは、描く量を減らすこと (T0857・T0858) で行う。

スタジオの GPU の処理は 2 つのプロセスにある:

- **仮想カメラ:** スタジオのプロセス (`ugoe-studio.exe`) のネイティブの wgpu (D3D12 か Vulkan)。
- **出力ウィンドウ:** WebView2 の GPU のプロセス (`msedgewebview2.exe`)。スタジオの WebView2 の環境 (データの場所) ごとに 1 つで、操作ウィンドウの描画も同じプロセス。
  プロセスの番号は WebView2 の `ICoreWebView2Environment8::GetProcessInfos` (種類が GPU のもの) で分かる。
- ブラウザソースは OBS のプロセスで描くので、スタジオからは下げられない (OBS 自身は、管理者として動くと自分の GPU の優先度を上げる)。

優先度のクラスは `IDLE`・`BELOW_NORMAL`・`NORMAL`・`ABOVE_NORMAL`・`HIGH`・`REALTIME`。下げるだけなら権限は要らない見込み
(上げるのは管理者の権限が要る。OBS が「管理者として実行」を求める理由)。下げると、ゲームや OBS が GPU を使い切っている間はスタジオの
描画が遅れ、出力のフレームが落ちる (それが「譲る」ことの代償)。既定は今と同じ `NORMAL` にし、利用者が選んだときだけ下げる。

**判断が要る** (`unsafe` と依存の機能の追加、ADR 0028 の 8. の `unsafe` の例外の範囲、ほかのプロセス (WebView2 の GPU のプロセス) の設定を変えること)。
作業者は下の案から勝手に選ばず、利用者に確かめてから進める。

### 案

- **A. 仮想カメラのクレート (`apps/studio/vcam`。`unsafe` を許す) に Windows の GPU の優先度の口を置く:** `windows` クレート (ワークスペースに
  ある。`Wdk_Graphics_Direct3D` などの機能を足す) で `D3DKMTSetProcessSchedulingPriorityClass` を呼ぶ安全な関数
  (`set_gpu_priority(pid, class)`) を出し、スタジオのバックエンドが自分のプロセスと WebView2 の GPU のプロセスに当てる。WebView2 の
  プロセスの一覧は wry が依存する `webview2-com` の COM (Tauri の `with_webview` から `ICoreWebView2Environment8`) で取り、
  `ProcessInfosChanged` で GPU のプロセスが作り直されたら当て直す。COM の呼び出しも `unsafe` なので vcam のクレートに置く。
  ADR 0028 の 8. に「仮想カメラのクレートに、Windows の GPU の優先度の口も置く」と追記する。
- **B. `unsafe` を許す新しいクレート (例: `apps/studio/os`) を作り、ここに OS の境界の小さな口 (GPU の優先度・T0855 の App Nap など) をまとめる:**
  仮想カメラの例外を広げずに済むが、`unsafe` の例外のクレートが 1 つ増える (ADR を書く。AGENTS.md の例外の一覧も直す)。
- **C. 自分のプロセス (仮想カメラ) だけ下げる:** WebView2 の GPU のプロセスには触らない。出力ウィンドウは下がらないので効きは小さい。A か B の置き場所は要る。
- **D. 実装しない:** 描く量を減らす (T0857・T0858) と案内 (OBS を管理者として実行すると OBS の GPU の優先度が上がる・Windows の
  「ハードウェア アクセラレータによる GPU スケジューリング」) だけにする。

### 推奨

A。新しいクレートも `unsafe` の例外のクレートも増やさず (ADR 0028 の 8. の追記だけ)、出力ウィンドウ (WebView2) と仮想カメラの両方に効く。
Windows だけの機能で、既定は今と同じ (下げない) にする。ただし、効き目 (ゲームの fps が上がるか・スタジオの出力がどのくらい落ちるか) は
実機で測るまで分からないので、**先に T0857 (変わったときだけ描く) を入れ、それでも足りないときに入れる** ことも推奨に含める
(T0857 で受け手の GPU の量は刻みの速さまで減る)。CPU のプロセスの優先度 (`SetPriorityClass`) は、指揮役の刻みのスレッドを遅らせて
出力の滑らかさを崩すので、下げない (案に含めない)。

## 受け入れ条件

- [ ] 利用者に案を示して方針を決め、決めたことと理由を docs/studio.md に書いた (A なら ADR 0028 の 8. に追記、B なら新しい ADR と AGENTS.md の例外の一覧)
- [ ] 決めた方針を入れた: 「フレームレートと負荷」に「GPU の優先度」(通常・低い) を Windows でだけ出し、選んだら自分のプロセスと
      (A・B なら) WebView2 の GPU のプロセスに当てる。当てられなかった (権限・古い WebView2 Runtime・プロセスが見つからない) ときは理由をコードで
      操作ウィンドウに出す (ADR 0029)。設定は T0860 の判断 2 と同じ置き場所 (機械ごとの設定) に持つ
- [ ] `unsafe` のブロックには前提を `// SAFETY:` で書き、境界の外へは安全な関数だけを出した。Windows の外ではビルドされない・何もしないことを確かめた
- [ ] `just vcam-check`・`just cross-check` (Windows のターゲットの clippy)・`just licenses`・`just ci` が通る

### 確かめていないこと (実機)

- Windows の実機で、GPU を使い切るゲーム (かベンチマーク) と OBS を動かしながら、優先度を「低い」にするとゲームの fps と OBS の
  「レンダリングの遅延によるフレームの欠落」が改善するか、スタジオの出力 (「フレームレートと負荷」の描画の回数) がどのくらい落ちるか。
  WebView2 Runtime が GPU のプロセスを作り直したときに当て直されるか。docs/manual-check.md の「S. スタジオ」に手順を足す

## 参考

- docs/adr/0028-studio.md の 8.、apps/studio/vcam (Windows の COM と `unsafe` の境界の例)、apps/studio/src-tauri/src/lib.rs
- `D3DKMTSetProcessSchedulingPriorityClass`: https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/d3dkmthk/nf-d3dkmthk-d3dkmtsetprocessschedulingpriorityclass
- 優先度のクラス: https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/d3dkmthk/ne-d3dkmthk-_d3dkmt_schedulingpriorityclass
- WebView2 のプロセス: https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/process-model 、
  `GetProcessInfos`: https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2environment8
- T0076 のメモ (調べた結果)

## メモ

- T0076 で起票した (利用者の要望 2026-10-08「GPU の優先度」の 2. ほかのアプリに GPU を譲る)。
- D3D11 の `IDXGIDevice::SetGPUThreadPriority` (-7〜7) はデバイスごとの口だが、wgpu は D3D12 (と Vulkan) を使い、WebView2 の中のデバイスにも
  触れないので使えない。D3D12 のキューの優先度 (`D3D12_COMMAND_QUEUE_PRIORITY`) は「通常」より下が無い。
- `windows` クレートの `D3DKMTSetProcessSchedulingPriorityClass` の機能の名前 (`Wdk_Graphics_Direct3D` の見込み) は、作業のときに 0.62 で確かめる。
- 判断 (2026-10-08): 利用者が「新しい unsafe のクレート」を選んだ。GPU の優先度だけを受け持つ新しいクレートを作り、unsafe の例外を 1 つ増やす。AGENTS.md の unsafe の例外の一覧と、ADR (新しく書くか ADR 0028 の 8. の見直し) を同じ PR で直す。既定は通常の優先度。T0857 の後に、まだ要るかを測ってから作る。
