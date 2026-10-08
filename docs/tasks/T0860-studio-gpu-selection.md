---
id: T0860
title: "スタジオ: 使う GPU (自動・省電力・高性能) を選べるようにし、受け手ごとに使っている GPU を見せる (判断が要る)"
status: done
milestone: M8
crates: [apps/studio, ugoe-headless, packages/player, docs]
depends_on: [T0499]
assignee: claude-worker-T0860
---

# T0860: スタジオ: 使う GPU を選ぶ (判断が要る)

## 目的

利用者の要望「ノート PC などで、外付け GPU (高性能) と内蔵 GPU (省電力) のどちらで描くかを選ぶ」(T0076 で調べた)。

スタジオで GPU を使うのは 3 か所で、選び方がそれぞれ違う (詳しくは T0076 のメモ):

| 受け手 | どこで描くか | 今の選び方 | 選ぶ手段 |
| --- | --- | --- | --- |
| 出力ウィンドウ | WebView の GPU のプロセス (Windows は WebView2 の `msedgewebview2.exe`、macOS は WebKit の GPU のプロセス、Linux は WebKitGTK のプロセス) の WebGPU か WebGL2 | ブラウザ任せ (希望を渡していない) | `powerPreference` (T0499)。Windows の WebView2 は無視するので、起動の引数 `--force-high-performance-gpu` |
| 仮想カメラ | スタジオのプロセスのネイティブの wgpu (`ugoe-headless` の `Gpu`) | 環境変数 `WGPU_POWER_PREF` だけ (無ければ列挙の順の先頭) | wgpu の `PowerPreference` (DX12・Vulkan・Metal で効く) |
| ブラウザソース | OBS のプロセス (CEF) | OBS が決める | スタジオからは選べない (OBS の設定と OS の設定) |

設定は「自動」(今と同じ)・「省電力」・「高性能」の 3 つ。どの GPU で描いているかを「フレームレートと負荷」に受け手ごとに出し、
選んだとおりになったかを利用者が確かめられるようにする。

**判断が要る** (OS の設定の書き換えの有無・設定の置き場所・起動し直しが要ること)。作業者は下の案から勝手に選ばず、利用者に確かめてから進める。

### 判断 1: Windows の WebView2 (出力ウィンドウ) の GPU をどう選ぶか

Windows の Chromium は `powerPreference` を無視し、ブラウザ全体で 1 つのアダプタ (ノート PC ではたいてい内蔵) を使う。
アプリごとのグラフィックスの設定 (設定 → ディスプレイ → グラフィック) は実行ファイルごとに効くが、WebView2 の描画は `ugoe-studio.exe` ではなく
WebView2 Runtime の `msedgewebview2.exe` (版ごとにパスが変わり、ほかの WebView2 のアプリと同じ実行ファイル) の GPU のプロセスで行う。

- **A. 「高性能」のときだけ WebView2 の起動の引数に `--force-high-performance-gpu` を足す (次の起動から効く) + Windows の「グラフィックの設定」を開くボタン:**
  引数は WebView2 の環境 (データの場所) ごとなので、スタジオの WebView だけに効く。2 つのウィンドウの引数は揃える必要がある
  (docs/studio.md#隠れても止まらない) ので、`tauri.conf.json` の静的な `additionalBrowserArgs` をやめ、起動時に設定を読んでから
  2 つのウィンドウをコードで作る (`WebviewWindowBuilder::additional_browser_args`)。「省電力」は WebView2 の既定 (たいてい内蔵) に任せる。
  仮想カメラの側は Windows の設定が効くので、`ms-settings:display-advancedgraphics` を開くボタンと手順 (ugoe-studio.exe を足して選ぶ) を出す。
  OS の設定は書き換えない。
- **B. A に加えて、レジストリ `HKCU\Software\Microsoft\DirectX\UserGpuPreferences` に `ugoe-studio.exe` の希望を書く:** OS の設定の書き換えで、
  利用者が Windows の設定で決めたことを上書きしうる。効くのは仮想カメラ (スタジオのプロセス) だけで、出力ウィンドウ (msedgewebview2.exe) には効かない。
  msedgewebview2.exe に書くと、ほかのアプリの WebView2 まで変わる。
- **C. 出力ウィンドウは選ばない:** 仮想カメラだけを選べるようにし、出力ウィンドウは案内だけ。

**推奨: A。** OS の設定を書き換えず、スタジオの WebView だけに効く。`--force-high-performance-gpu` が WebView2 で効くことは実機でしか確かめられない
(Chrome の `chrome://flags/#force-high-performance-gpu` と同じスイッチで、Windows でも効くと Chrome の文書にある)。

### 判断 2: 設定をどこに持つか

- **a. 機械ごとの設定 (アプリの設定のディレクトリのファイル。例: `studio-settings.json`):** GPU はその機械のもので、プロファイルを別の機械に持って行っても
  そのまま使えるべきなので、プロファイルに入れない。起動時に読む (WebView2 の引数のため)。
- **b. プロファイルの `output`:** 配信ごとに変えられるが、プロファイルを開き直すたびに起動し直しが要ることになる。

**推奨: a。**

### 判断 3: Linux と macOS

- **Linux:** WebKitGTK が使う GPU は環境変数 (`DRI_PRIME`・NVIDIA の `__NV_PRIME_RENDER_OFFLOAD` と `__GLX_VENDOR_LIBRARY_NAME`) で決まり、
  起動の後で変えるには自分を起動し直すか、`std::env::set_var` (edition 2024 では `unsafe`) が要る。デスクトップのエントリの `PrefersNonDefaultGPU=true`
  は高性能の希望だけで、ランチャー (GNOME など) が対応するときだけ効く。
  **推奨:** 出力ウィンドウは選ばず、マニュアルに環境変数での起動の仕方を書く。仮想カメラは wgpu の `PowerPreference` で選べる (Vulkan はどの GPU も列挙する)。
- **macOS:** Apple Silicon は GPU が 1 つなので差は無い。2 つの GPU を持つ Intel の Mac では、WebKit が WebGL / WebGPU の `powerPreference` を
  見るはずで (未確認)、ネイティブの wgpu (Metal) は `PowerPreference` で低電力の機器を選べる。`Info.plist` の `NSSupportsAutomaticGraphicsSwitching`
  を書けば、スタジオのプロセスが不要に外付け GPU へ切り替えさせることを避けられるが、Intel の Mac だけの話なので **推奨:** 書かない (要望があれば別のタスク)。

## 受け入れ条件

- [x] 利用者に判断 1〜3 の案を示して方針を決め、決めたことと理由を docs/studio.md に「使う GPU」の節を作って書いた
- [x] `ugoe-headless` の `Gpu` に、電力の希望を引数で渡して作る口を足した (省くと今と同じく `WGPU_POWER_PREF` に従う)。
      `crates/ugoe-headless/README.md#gpu` を直した
- [x] 仮想カメラは、始めるときに設定の希望で GPU を作る。設定が変わったら、次に始めるときに作り直す (前の GPU は破棄せず手放す。
      `virtual_camera.rs` の `GpuCell` の規則のまま。手放す回数が増えすぎないよう、同じ希望なら作り直さない)
- [x] 出力ウィンドウは T0499 の `powerPreference` を設定から渡す (作り直しでも同じ値)。決めた案に従って、Windows の WebView2 の起動の引数を足す
      (A なら、2 つのウィンドウをコードで作り、引数が揃うことを `lib.rs` のテストで確かめる。今の `additionalBrowserArgs` の中身と
      `backgroundThrottling` の検査は保つ)
- [x] 「フレームレートと負荷」に「使う GPU」(自動・省電力・高性能) を足し、変えると次の起動から (か、次に始めるときから) 効くことを出す。
      受け手ごとの行に、使っている API とアダプタの名前 (出力ウィンドウ・ブラウザソースは T0499 の `adapter_name()` を描画の負荷の報告に載せる。
      仮想カメラは `Gpu::adapter_info`) を出す。ブラウザソースの GPU は OBS が決めることを説明の文に書く。文言は `apps/studio/locales/*.json`
- [x] 決めた案の OS の案内 (Windows の「グラフィックの設定」の手順・Linux の環境変数) を docs/manual/studio.md に書いた
- [x] `just vcam-check`・`just cross-check` (Windows・macOS の `cfg` を触るなら)・`just e2e --only studio`・`just ci` が通る

### 確かめていないこと (実機)

- 2 つの GPU を持つ Windows のノート PC で、「高性能」で出力ウィンドウ (`--force-high-performance-gpu`) と仮想カメラが外付け GPU を使うこと
  (タスクマネージャーの「GPU エンジン」の列)、「省電力」で内蔵 GPU を使うこと
- Linux の PRIME (DRI_PRIME・NVIDIA のオフロード) と、Intel の Mac での切り替え
- docs/manual-check.md の「S. スタジオ」に手順を足す

## 参考

- docs/studio.md#仮想カメラ・#隠れても止まらない・#負荷の表示、apps/studio/src-tauri/tauri.conf.json、apps/studio/src-tauri/src/lib.rs・virtual_camera.rs
- crates/ugoe-headless/src/gpu.rs (`Gpu::new`)、T0499
- T0076 のメモ (調べた結果と出典)

## メモ

- T0076 で起票した (利用者の要望 2026-10-08「GPU の優先度」の 1. 使う GPU を選ぶ)。
- wgpu は `PowerPreference::None` ではアダプタを列挙の順のまま選び、`LowPower` / `HighPerformance` では内蔵 / 外付けを先に並べ替える
  (wgpu-core 30 の instance.rs)。Windows の DXGI の列挙の先頭は、Windows のアプリごとの設定で選んだ GPU になる見込み (未確認。
  `DXGI_GPU_PREFERENCE` を使う `EnumAdapterByGpuPreference` とは別の口なので、実機で確かめる)。
- 判断 (2026-10-08): 利用者が、Windows の出力ウィンドウは推奨の案 (高性能を選んだときだけ WebView2 を `--force-high-performance-gpu` で起動し、次の起動から効く。Windows のグラフィックスの設定を開くボタンを足す。レジストリは書かない) を、設定の置き場所は機械ごとの設定ファイルを選んだ。Linux と macOS は推奨どおり文書だけにする (利用者には聞いていない。推奨のまま)。
- やったこと (2026-10-08、claude-worker-T0860):
  - `ugoe-headless`: `Gpu::with_power_preference(backend, Option<PowerPreference>)` を足した (`PowerPreference` は wgpu の型の再輸出)。
    `None` と `Gpu::new` は今までどおり `WGPU_POWER_PREF` に従い、渡した希望は環境変数より優先する。README の「GPU」を直した。
  - スタジオ: `src-tauri/src/gpu.rs` に設定 (`auto`・`lowPower`・`highPerformance`) を置き、機械ごとの設定ファイル `studio-settings.json`
    (アプリの設定の場所。`{"format":"ugoe-studio-settings","version":1,"gpu":…}`) に持つ。無い・読めない・知らない値は「自動」。
    コマンドは `gpu_settings`・`set_gpu_preference` (書けなければ変えずに `gpu_settings.save_failed`)・`open_graphics_settings`
    (Windows は `explorer.exe ms-settings:display-advancedgraphics`、ほかは `graphics_settings.unsupported`)・出力ウィンドウ用の
    `output_power_preference` (起動したときの値)。capabilities と `acl.rs` も足した。
  - 2 つのウィンドウは `tauri.conf.json` に `create: false` で書き (`additionalBrowserArgs` は消した)、`setup` で設定を読んでから
    `lib.rs` の `window_configs` で同じ WebView2 の引数 (`gpu::browser_args`。「高性能」だけ `--force-high-performance-gpu` を足す) を付けて作る。
    `lib.rs` のテストで、どの設定でも 2 つの引数が揃うこと・抑制を切る引数と `backgroundThrottling` が残ること・高性能だけ足すことを確かめる。
  - 仮想カメラ: 始めるときの設定で `Gpu::with_power_preference` を呼ぶ。`GpuCell` は作ったときの希望を覚え、違う希望なら前の GPU を
    (破棄せずに) 手放して作り直す。同じ希望なら作り直さない。作れなかったことも同じ希望なら覚え、違う希望なら試し直す (テストあり)。
  - 負荷の報告 (`RenderStats`) に `backend`・`adapter` を足した (出力ウィンドウとブラウザソースは `Player.backend`・`adapterName`、
    仮想カメラは `Gpu::adapter_info` の `backend.to_str()` と名前)。256 バイトを超える名前は報告ごと捨てる (ブラウザソースのページから届くため)。
  - 「フレームレートと負荷」: 「使う GPU」の切り替え・説明 (出力ウィンドウは次の起動から、仮想カメラは次に始めるときから、ブラウザソースは OBS が決める)・
    起動したときの値と違えば「起動し直すと…」の知らせ・Windows では「グラフィックの設定を開く」のボタンと手順。受け手ごとの行に API と GPU の名前
    (伏せられたら「伏せられています」)。文言は英語と日本語。
  - 文書: docs/studio.md に「使う GPU」の節 (判断 1〜3 と理由)、「隠れても止まらない」「負荷の表示」「仮想カメラ」「実機で確かめていないこと」を直した。
    docs/manual/studio.md に「使う GPU」(Windows のグラフィックの設定の手順・Linux の環境変数・macOS)。docs/manual-check.md に S33。
    apps/studio/README.md (コマンド・エラーのコード・ファイル)。マニュアルの画像 `studio-load.png` を撮り直した (WebKitGTK では「WebGL2・Apple GPU」と出る。
    WebKit が GPU の名前を伏せた値)。
  - 確かめたこと: `just ci`・`just cross-check` (apps の Windows の clippy を含む)・`just vcam-check`・`just e2e --only studio` (13 件)・
    `just ui-audit --only studio` (0 件)・`just refs`。
- 確かめていないこと (実機): 上の「確かめていないこと (実機)」のとおり。`--force-high-performance-gpu` が WebView2 で効くか、Windows の
  `explorer.exe ms-settings:…` で設定が開くか、2 つの GPU で受け手ごとの GPU が選んだとおりになるか、Linux の PRIME・Intel の Mac。
  手順は docs/manual-check.md の S33。
- やらなかったこと: Linux と macOS は文書だけ (判断 3)。Linux の出力ウィンドウにも `powerPreference` は渡す (効くかは WebKitGTK 次第)。
  レジストリは書かない (判断 1)。
- 起票の候補:
  - Intel の Mac で `Info.plist` の `NSSupportsAutomaticGraphicsSwitching` を書く (要望があれば。判断 3)。
  - 「フレームレートと負荷」の行の文と GPU の名前が、スクリーンリーダーでは区切り無しに続けて読まれる (`<dd>` の中の 2 つ目の行)。
    ほかの値と合わせて読み上げの区切りを見直す。
