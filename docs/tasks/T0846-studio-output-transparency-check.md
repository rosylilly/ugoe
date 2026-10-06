---
id: T0846
title: "スタジオ: 出力ウィンドウの透過が効いているかを調べ、効かない環境で注意を出す"
status: done
milestone: M9
crates: [apps/studio, docs]
depends_on: []
assignee: claude-worker-T0846
---

# T0846: スタジオ: 出力ウィンドウの透過が効いているかを調べ、効かない環境で注意を出す

## 目的

スタジオの出力ウィンドウは常に透過で、背景はシーンが決める。T0818 で「出力ウィンドウ」のパネルの「背景」の欄に、配信しているシーンの背景を出すようにしたが、
透過が実際に効いているか (Linux のコンポジタが無い・WebView が透過を描かない・Windows の取り込みが BitBlt で透過を拾わないなど) はスタジオが知らないので、
透過のシーンでも黒い地が配信に乗ることがある (T0818 の候補)。

## 受け入れ条件

- [x] OS ごとに、透過が効かない場合と、それをバックエンドで知る方法 (例: X11 の `_NET_WM_CM_S<n>` の選択の持ち主の有無、Wayland、Windows・macOS は常に合成) を調べ、
      結果と出典をメモに書いた。知れないものは知れないと書く
- [x] 知れる場合について、透過 (とクロマキーでない) のシーンを出しているのに透過が効かないとき、「出力ウィンドウ」のパネルの「背景」の欄に注意 (`Notice` か印の三角) と
      代わりの案 (クロマキーの背景・ブラウザソース) を出す。判定の関数の単体テストと部品のテストがある
- [x] 実機での確かめの手順 (コンポジタの有無・各 OS の取り込み) を docs/manual-check.md に足した
- [x] docs/studio.md と docs/manual/studio.md (困ったとき) を直した
- [x] `just ci` が通る (`#[cfg(windows)]` などを触ったら `just cross-check` も)

## 参考

- T0818 のメモ (出力ウィンドウの様子・やらなかったこと・起票の候補)
- docs/studio.md (出力ウィンドウ)、apps/studio/src (`OutputPanel.svelte`・`lib/status.ts`)

## メモ

- T0040 で起票した (元の候補: T0818 (出力ウィンドウの透過))。
- **調べた結果 (出典は docs/studio.md#出力ウィンドウの透過 に書いた):**
  - Linux (X11): コンポジタが無い (i3・Openbox だけ、Xfce・KDE で合成を切った、Xvfb) と透過の所が黒い。GDK の `gdk_screen_is_composited` が
    `_NET_WM_CM_S<n>` の選択の持ち主の有無を見る (EWMH の Compositing Managers。GTK 3 の gdkscreen-x11.c)。加えて出力ウィンドウのビジュアルの深さ (32) を見る
    (tao は作るときに画面のアルファ付きのビジュアルを当てる)。→ **知れる**。
  - Linux (Wayland): 常に合成 (GDK の Wayland の実装は常に真)。→ `supported`。
  - Windows: 8 以降は DWM が常に合成 (`DwmIsCompositionEnabled` は常に TRUE)。macOS: 常に合成 (透過は `macOSPrivateApi`、入れている)。→ 調べずに `supported`。
  - **知れないもの:** 配信ソフトの取り込みがアルファを拾うか (OBS の BitBlt・WGC・ゲームキャプチャ、macOS の画面キャプチャ、Xcomposite・PipeWire)、
    WebView が透過を描くか (WebView の不具合や設定)。注意は出さず、マニュアルの「困ったとき」を 2 行に分けた (注意が出る場合と出ない場合)。
- **やったこと:**
  - バックエンド (`lib.rs`): `OutputState` に `transparency` (`supported`・`unsupported`・`unknown`)。Linux は GDK に聞く (`gtk` を Linux だけの直接の依存にした。
    tauri が使っている 0.18 で新しいクレートではない。ルートの `Cargo.toml` の `[workspace.dependencies]` に足した (範囲の外だが 1 行))。
    GDK の `composited-changed` で様子を送り直す (コンポジタの起動・終了)。判定の単体テストと JSON の形のテスト。
  - フロントエンド: `lib/status.ts` の `transparencyLost` (透過のシーン かつ `unsupported`。単色・クロマキー・画像・`unknown` では出さない)、
    出力の印を三角に (`status.output.opaqueTip`)、`OutputPanel.svelte` の「背景」の欄に `Notice` (warning) と代わりの案 (クロマキー・ブラウザソース)。
    単体テスト (`status.test.ts`) と部品テスト (`OutputPanel.svelte.test.ts`)。
  - 範囲の外の小さな変更: tests/ui-audit の偽のホストの出力の様子に `transparency: "supported"`、tests/e2e の studio に手順を 1 つ
    (Xvfb にはコンポジタが無いので、本物の Tauri と WebKitGTK で `unsupported` になり「背景」の欄に注意が出ることを確かめた。README の筋書きも直した)。
  - 文書: docs/studio.md (状態の印の表・出力ウィンドウ・新しい節「出力ウィンドウの透過」(OS ごとの表と出典)・Linux の取り込み・実機で確かめていないこと)、
    docs/manual/studio.md (はじめての配信の準備・Linux の取り込み・困ったとき)、docs/manual-check.md (S28・S29、要るもの)、apps/studio/README.md。
- **確かめたこと:** WSLg の Wayland と XWayland では合成あり、Xvfb では `gdk_screen_is_composited` が偽でアルファ付きのビジュアル (深さ 32) はある (python の GDK で)。
  `just e2e --only studio` で、本物のスタジオ (Xvfb) が `unsupported` を返し「背景」の欄に注意が出た。`just cross-check` (Windows) が通った
  (macOS の分岐は Windows と同じ関数なので、apps の macOS の clippy は CI の macOS のジョブ任せ)。
- **確かめていないこと:** 実際の X11 のデスクトップでコンポジタを止める・起動したときの注意の出入りと、起動の後で透けるようになるか (S28)。
  Wayland・Windows・macOS で注意が出ないことと、各 OS の取り込みでの透過 (S29・S14)。`--headed` で WSLg に出す確かめは利用者の画面に出るのでしていない。
- **`just e2e --only studio` の既存の失敗 (この変更と無関係):** 「connects a client to the studio API」と「serves the browser source」が、
  もう無い `.studio__api` の印 (T0817 で「いまの状態」に移した) を待って時間切れになる。main でも同じ (スペックのセレクタが古い)。
- **`just ci`:** main (rebase 後) で通った。
- **起票の候補:**
  - tests/e2e の studio のスペックの `.studio__api` を今の「いまの状態」の印に合わせる (上の 2 つの失敗)。
  - 注意の Notice に「背景をクロマキーにする」ボタン (今のシーンの背景を押すだけで替える)。
  - tests/ui-audit の偽のホストに、透過が効かない場面 (`-opaque`) を足す (今は `supported` 固定)。
