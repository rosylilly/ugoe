---
id: T0855
title: "スタジオ: macOS の App Nap で刻みが遅れないようにする (判断が要る)"
status: done
milestone: M8
crates: [apps/studio, docs, Cargo.lock]
depends_on: []
assignee: claude-worker-T0855
---

# T0855: スタジオ: macOS の App Nap を止める (判断が要る)

## 目的

macOS の App Nap は、ウィンドウがすべて隠れた (ほかのウィンドウの裏・最小化・別のデスクトップ) アプリのタイマーとスレッドの
起床を遅らせる。スタジオの指揮役の刻みはバックエンドのスレッドのスリープで回すので、配信中に操作ウィンドウと出力ウィンドウが
隠れると、刻みが遅れて出力 (ウィンドウの取り込み・ブラウザソース・仮想カメラ) が滑らかでなくなりうる。WebView の
`backgroundThrottling` の設定は App Nap を止めない (docs/studio.md#刻みの速さと正確さ)。

App Nap を止めるには `NSProcessInfo` の `beginActivityWithOptions:reason:` を呼ぶ。

**判断が要る** (依存の追加か `unsafe`、ADR 0028 の 8. の `unsafe` の例外の範囲)。作業者は下の案から勝手に選ばず、利用者に確かめてから進める。

### 案

- **A. スタジオのバックエンド (`ugoe-studio`) が macOS でだけ `objc2-foundation` を使い、起動から終わりまで活動を始めたままにする:**
  `objc2-foundation` はワークスペースの依存に既にある (仮想カメラの `ugoe-studio-vcam` が使う。MIT) ので、新しいクレートは増えない
  (Cargo.lock の `ugoe-studio` の依存の行が増えるだけ)。`NSProcessInfo::processInfo().beginActivityWithOptions_reason(…)` は
  objc2-foundation 0.3 で安全な関数なので `unsafe` は要らない。終える `endActivity` は `unsafe` なので呼ばず、返った値をアプリの
  終わりまで持つ。選択肢は `NSActivityUserInitiatedAllowingIdleSystemSleep` (App Nap は止めるが、画面と計算機のスリープは妨げない)。
  配信していない間も App Nap しないが、スタジオは開いている間は刻み続けるアプリなので差は小さい。
- **B. 仮想カメラのクレート (`apps/studio/vcam`。`unsafe` を許す) に、活動の開始と終わり (`endActivity` を含む) を置く:**
  出力ウィンドウを出している・仮想カメラを動かしている間だけ止められる。ADR 0028 の 8. で `unsafe` を許したのは仮想カメラの
  OS の部品のためなので、その範囲を広げることになる (ADR の追記が要る)。
- **C. `Info.plist` の `NSAppSleepDisabled`:** コードを書かない。ただしアプリの設定 (user defaults) の鍵で、`Info.plist` に書いて効くかは
  文書に無く、実機で確かめる必要がある。
- **D. 止めない:** 「出力ウィンドウを前面のどこかに見えるように置く」と docs/studio.md と manual に案内するだけ。

### 推奨

A。新しいクレートも `unsafe` も増えず、ADR の例外も広げない。配信アプリとして、隠れても刻みを守るのが期待どおりの振る舞い。
どの案でも、効いたかどうかは macOS の実機でしか確かめられない (手元の CI と WSL では確かめられない)。

## 受け入れ条件

- [x] 利用者に案を示して方針を決め、決めたことと理由を docs/studio.md#刻みの速さと正確さ に書いた (B なら ADR 0028 の 8. に追記)
- [x] 決めた方針を入れた。macOS の外ではビルドされない・何もしないこと、`just vcam-check` (macOS のターゲットの clippy。A でも
      `#[cfg(target_os = "macos")]` のコードを触るので) が通ることを確かめた
- [x] macOS の実機で、出力ウィンドウと操作ウィンドウを隠したまま数分刻んで、負荷の表示 (実際の刻みの速さと最も長い間隔) に注意が出ないことを
      確かめる手順を docs/manual-check.md に足した (確かめられなければ、確かめていないことをメモと docs/studio.md#実機で確かめていないこと に書く)
- [x] `just licenses` と `just ci` が通る

## 参考

- docs/studio.md#刻みの速さと正確さ・#隠れても止まらない・#実機で確かめていないこと、docs/adr/0028-studio.md の 8.
- apps/studio/src-tauri/src/lib.rs (起動と `run_ticks`)、apps/studio/vcam/src/cmio.rs (`NSProcessInfo` の使い方の例)
- Apple の文書: <https://developer.apple.com/documentation/foundation/processinfo/beginactivity(options:reason:)>

## メモ

- T0075 で起票した (元: docs/roadmap.md の M8 の「起票していない後続の候補」の「macOS の App Nap を止める (依存か unsafe が要るので利用者の判断)」)。
- T0856 (承認の OS への知らせ) と同じくスタジオの `src-tauri/Cargo.toml` と Cargo.lock を触りうるので、T0856 はこのタスクの後にする。
- 判断 (2026-10-08): 利用者が案 A (スタジオのバックエンドから objc2-foundation の beginActivityWithOptions_reason を呼び、起動中ずっと活動を始めたままにする。新しいクレートも unsafe も増やさない) を選んだ。
- やったこと (2026-10-08):
  - `apps/studio/src-tauri/src/app_nap.rs` を足し、`lib.rs` の `run` の初めに `app_nap::prevent()` で活動を始め、`run` の終わりまで持つ
    (`NSActivityOptions::UserInitiatedAllowingIdleSystemSleep`)。macOS では `objc2`・`objc2-foundation` (NSObject・NSProcessInfo・NSString) を
    `cfg(target_os = "macos")` の依存に足した (どちらも仮想カメラが使う版で、Cargo.lock は `ugoe-studio` の依存の行が増えただけ)。`unsafe` は無い
    (`beginActivityWithOptions_reason` は objc2-foundation 0.3.2 で安全な関数)。macOS の外では大きさ 0 の値を返すだけ (テストで確かめる)。
  - docs/studio.md#刻みの速さと正確さ に決めたことと理由・ほかの案を書き、#実機で確かめていないこと の表に足した。docs/manual-check.md に S30 を足した。
- 確かめたこと: `just crate ugoe-studio`・`just vcam-check`・`just licenses`・`just refs`・`just ci` が通る。`ugoe-studio` 全体の
  `aarch64-apple-darwin` の clippy は、tauri の依存 (objc2-exception-helper の Objective-C) が Apple の SDK を要るので WSL では流せない
  (CI の rust (macos) が `just lint-group apps` で見る)。代わりに `app_nap.rs` を `#[path]` で読む使い捨てのクレート (同じ版の objc2・objc2-foundation、
  ワークスペースと同じ lints) を作り、`aarch64-apple-darwin` と `x86_64-unknown-linux-gnu` で `clippy --all-targets -D warnings` が通ることを確かめた。
- 確かめていないこと: macOS の実機で App Nap が止まり、両方のウィンドウを隠しても刻みが遅れないこと (manual-check.md の S30)。
  `tauri::Builder::run` の後で活動の値が落ちるかは見ていない (落ちても落ちなくてもプロセスの終わりで活動は消える)。
- 起票の候補: なし。
