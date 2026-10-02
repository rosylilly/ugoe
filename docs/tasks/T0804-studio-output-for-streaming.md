---
id: T0804
title: スタジオ: 配信向けの出力 (隠れても止まらない描画・フレームレート・負荷の表示・OBS の取り込み)
status: done
milestone: M8
crates: [apps/studio, packages/player, docs]
depends_on: [T0801, T0133]
assignee: claude-worker-T0804
---

# T0804: スタジオ: 配信向けの出力 (隠れても止まらない描画・フレームレート・負荷の表示・OBS の取り込み)

## 目的

配信ソフトに取り込んだときに、止まらず・重くならずに出せるようにする。

## 受け入れ条件

- [x] 出力ウィンドウと、入力を取る操作ウィンドウが、隠れたり最小化されたりしても止まらない (両方の WebView のバックグラウンドの抑制を切る)
- [x] 指揮役の Puppet の刻みの速さ (30 / 60 / 120。ADR 0028)、描画の負荷と 1 フレームの時間の表示 (操作ウィンドウだけ)
- [x] OBS でのウィンドウの取り込み (透過を含む) の手順を Windows・macOS・Linux ごとに docs に書く。確かめられない OS は利用者の確認に残す
- [x] `just ci` が通る

## 参考

- docs/studio.md

## メモ

### やったこと (2026-10-02)

- **隠れても止まらない:** 両方のウィンドウの `tauri.conf.json` に、WebView2 の引数 (`--disable-background-timer-throttling`・
  `--disable-backgrounding-occluded-windows`・`--disable-renderer-backgrounding`・`--disable-features` に `CalculateNativeWinOcclusion`・
  `IntensiveWakeUpThrottling`。wry の既定の引数も書き写す) と macOS の `backgroundThrottling: "disabled"` を入れた。2 つのウィンドウの引数が同じことを
  `lib.rs` のテストで確かめる (WebView2 は食い違うと作れない)。Linux の WebKitGTK には切る口が無い。
- **受け手は描画に頼らない (@ugoe/player):** 受け手のモードで、`pushFrame`・`followScene` のその場で `follow` まで適用し、`requestAnimationFrame` は描くだけにした。
  `startFollowing(onRender)` で、描くたびに描画の CPU の時間と捨てたフレームの合計を渡す。FrameQueue の revision の扱いと覚えるもの・文言には触っていない (T0802・T0723 との約束)。
- **刻みの速さ:** `src-tauri/src/pacing.rs` (30 / 60 / 120・取り戻さない次の刻み・実際の速さと最も長い間隔の測定)。`set_tick_rate`・`tick_rate`・
  `report_render_stats` のコマンド。状態の通知に実際の速さと出力ごとの描画の負荷 (2 秒途絶えたら「描いていない」) を足した。
  Windows のスリープの粒度は、Rust の `thread::sleep` が高精度の待機タイマーを使うので spin はしない。揺れは注意として操作ウィンドウに出す (docs/studio.md#刻みの速さと正確さ)。
- **負荷の表示 (操作ウィンドウだけ):** `LoadPanel.svelte` (刻みの速さの切り替え・1 刻みの時間・実際の速さ・出力ごとの描画・注意)、ツールバーの要約、
  判断は `src/lib/load.ts`、出力ウィンドウの集計は `src/lib/renderStats.ts`。選んだ刻みの速さは `ugoe-studio.tick-rate` に覚える。
- **docs/studio.md:** 「隠れても止まらない」「刻みの速さと正確さ」「負荷の表示」「OBS でのウィンドウの取り込み」(Windows・macOS・Linux)。README も更新。

### 確かめたこと

- `just ci`、`cargo test -p ugoe-studio`、スタジオと @ugoe/player の vitest と svelte-check。
- ヘッドレスの Chrome と使い捨てのブリッジ (本物の指揮役。scratchpad/T0804/bridge・stall.mjs) で、出力ウィンドウの `requestAnimationFrame` を 12 秒止めても
  フレームを捨てないこと (捨てたフレーム 0。変更前の Player では 136 を捨てて物理がリセットされた) と、再開後に 60 回/秒で描くことを確かめた。
- 操作ウィンドウの負荷の表示を、偽のホストで英語・日本語・ダーク・狭い幅で撮った。刻みの速さを 120 にするとホストに渡り、覚えることも確かめた
  (scratchpad/T0804/shots/)。

### 確かめていないこと (利用者の確認に残す)

- 実際の Tauri のウィンドウで、隠れた・最小化した出力ウィンドウと操作ウィンドウが止まらないか (Windows の WebView2 の引数、macOS の
  `backgroundThrottling`)。Windows・macOS・Linux のどれも実機では未確認 (WSL の上)。
- Windows で 60・120 刻みの揺れが小さいか (高精度の待機タイマー)。
- OBS のウィンドウの取り込みで透過がアルファとして届くか、裏に隠れても取り込みが止まらないか (docs/studio.md の「未確認」の 3 か所)。

### 範囲外の変更

- `packages/player` (team-lead が承認、2026-10-02): 受け手のモードでフレームを届いたときに適用する (隠れて `requestAnimationFrame` が止まっても
  物理を止めないため) のと、描画の負荷を渡す `startFollowing(onRender)`。crates: に足した。

### 後続の候補

- macOS の App Nap を止める (`NSProcessInfo` の `beginActivity`。Objective-C の呼び出しが要るので、依存か `unsafe` の扱いを決めてから)。
- 刻みの速さをプロファイル (T0802) へ移す (いまは localStorage)。
- 入力 (T0803) の処理を `requestAnimationFrame` に頼らない形にする (Linux で操作ウィンドウを最小化すると止まる)。
- ブラウザソース (T0808)・仮想カメラ (T0807) の受け手も `report_render_stats` と同じ形で負荷を報告する。
