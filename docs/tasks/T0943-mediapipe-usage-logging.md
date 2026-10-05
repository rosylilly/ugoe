---
id: T0943
title: "MediaPipe の利用状況の送信 (odml.pa.googleapis.com) を送らせない方法を調べ、プライバシーの説明に書く"
status: done
milestone: M9
crates: [packages/face-tracking, docs]
depends_on: []
assignee: claude-worker-T0943
---

# T0943: MediaPipe の利用状況の送信 (odml.pa.googleapis.com) を送らせない方法を調べ、プライバシーの説明に書く

## 目的

T0029 で、トラッキングが 60 秒を超えると MediaPipe (@mediapipe/tasks-vision 1.0.1) が利用状況を `https://odml.pa.googleapis.com/v1/log` へ
送ろうとし (60 秒ごとと閉じたとき)、アプリの CSP (`connect-src`) が止めていることが分かった。e2e は止まって正しいものとして数えない
(`tests/e2e/lib/harness.mjs` の `EXPECTED_BLOCKS`)。止める設定は見つかっていない (T0029 の候補)。
3 つのアプリと `@ugoe/runtime` を使うページ (CSP の無いホスト) では、送信が止まらない場合がある。

## 受け入れ条件

- [x] 送信の仕組み (どこで・何を・いつ送るか。ライブラリの設定・`ModuleFactory` の差し替え・ビルドの選択で止められるか) を調べ、結果と出典をメモに書いた
      (MediaPipe のソースはリポジトリの外で読む。AGENTS.md の外部のコードの規則)
- [x] 送らせない方法があれば `packages/face-tracking` で使い、CSP の違反が出ないことを e2e (`EXPECTED_BLOCKS` から外して) で確かめた。無ければ理由を書いた
- [ ] 送らせられないなら、CSP で止めていること (アプリ) と、ホストのページで止めるための CSP の例 (使う側) を、docs (face-tracking の README・
      docs/manual のトラッキングの章などプライバシーを説明する場所) に書いた
- [x] `just ci` が通る

## 参考

- T0029 のメモ (CSP・起票の候補)
- tests/e2e/lib/harness.mjs (`EXPECTED_BLOCKS`)、packages/face-tracking

## メモ

- T0038 で起票した (元の候補: T0029 (MediaPipe の利用状況))。
- 調べたこと (`@mediapipe/tasks-vision` 1.0.1 の `vision_bundle.mjs` を `node_modules` の中で読んだ。リポジトリには写していない。
  出典: npm パッケージの README の「Privacy Notice」・https://goo.gle/mediapipe-privacy):
  - どこで: JS の側。`createFromOptions` の中で、タスクごとに記録の係を必ず作る (オプションで切れない)。送り先
    `https://odml.pa.googleapis.com/v1/log` は決め打ちで、API キーだけを wasm (`_mediapipeLoggerGetEncodedApiKey`) から取る。
    3 つの wasm の版 (SIMD・SIMD 無し・module) で JS は同じなので、ビルドの選択や `ModuleFactory` の差し替えでは止まらない。
  - 何を: タスクの種類・実行モード・OS の種類 (user agent)・版・検出の回数と処理時間の統計・落としたフレームの数 (protobuf)。
    映像・画像・ランドマークの値は含まない。
  - いつ: 作った直後に 1 件溜め、60 秒ごとの `setInterval` と `close()` で、溜まっていれば `fetch` で POST する。1 回失敗すると
    (例外か 200 以外) 溜めたものを捨て、タイマーを止めて以後は送らない。`fetch` は送るたびにグローバルから引く。
  - Landmarker ごとに係があるので、顔と体で 2 つ送ろうとする (e2e で 2 件を見た)。face-tracking は Landmarker を `close()` しない。
- やったこと:
  - `packages/face-tracking/src/usage-logging.ts`: `blockMediaPipeUsageLogging()` がグローバルの `fetch` を包み、
    `https://odml.pa.googleapis.com` への要求だけを `TypeError` で断る (1 回だけ包む)。`FaceTracker` が顔と体の Landmarker を作る前に呼ぶ。
    要求はネットワークに出ないので CSP の違反にもならない。アプリの CSP はそのまま (二重の守り)。単体テスト (`usage-logging.test.ts`)。
  - e2e: `EXPECTED_BLOCKS` を消した (CSP の違反は例外なしで数える)。`checkImageTracking` (3 つのアプリ) で、MediaPipe が読めた後に
    ページの `fetch` がこの送り先を断ることを確かめる。
  - 確かめたこと: 一時的にビューアの筋書きに 80 秒待つ手順を足して (コミットしていない) `just e2e --only viewer` を流し、
    MediaPipe 自身の送信 2 件 (顔と体) が包みで断られ、`has no CSP violations` が通ることを見た。足した手順を外して
    `just e2e` (エディタ・ビューア・スタジオ) がすべて通った。`just ci` が通った。
  - 文書: packages/face-tracking/README.md (「利用状況の送信」)、apps/viewer/README.md、apps/studio/README.md、docs/editor.md、
    docs/studio.md、docs/manual/preview.md・studio.md (利用者向けに、映像は送らず利用状況の送信も止めていること)、
    tests/e2e/README.md、docs/testing.md。
  - 範囲の外の小さな変更: tests/e2e (受け入れ条件が求める)、apps/*/README.md、bindings/web/README.md (「MediaPipe の利用状況の送信」:
    MediaPipe を自分で使うホストのページ向けに、CSP の `connect-src` の例と `fetch` を包む例)。
- やらなかったこと: 受け入れ条件 3 (送らせられないときの文書) は、送らせない方法があったので当てはまらない (止め方とホスト向けの CSP の例は書いた)。
  Windows (WebView2)・macOS (WKWebView) では確かめていない (仕組みは `fetch` を包むだけで WebView に依らない)。
- 起票の候補:
  - bindings/web: 見本 (`examples/basic`) の「camera (MediaPipe)」が利用状況を送る。見本でも `fetch` を包むか CSP を付ける。
  - face-tracking: MediaPipe の版を上げるときに、送り先と `fetch` の使い方が変わっていないかを自動で確かめる (例: `vision_bundle.mjs` に
    `odml.pa.googleapis.com` があり `fetch(` で送ることを調べるスクリプト。今は README に手で確かめると書いただけ)。
