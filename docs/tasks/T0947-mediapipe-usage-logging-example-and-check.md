---
id: T0947
title: "MediaPipe の利用状況の送信: web の見本でも止め、版を上げたときに送り方の変化を自動で見つける"
status: done
milestone: M9
crates: [bindings/web, packages/face-tracking, docs]
depends_on: []
assignee: claude-worker-T0947
---

# T0947: MediaPipe の利用状況の送信: web の見本でも止め、版を上げたときに送り方の変化を自動で見つける

## 目的

T0943 で、`@ugoe/face-tracking` はランドマーカーを作る前に MediaPipe の利用状況の送信 (`odml.pa.googleapis.com`) を止めるようになった。残ったもの (T0943 の候補):

- `bindings/web` の見本 (`examples/basic`) の「camera (MediaPipe)」は今も送る。見本でも `fetch` を包むか CSP を付ける。
- MediaPipe の版を上げたとき、送り先と `fetch` の使い方が変わっていないかを手で確かめるだけになっている (README)。

**ライセンスの注意:** `bindings/web` は MIT。GPL の `packages/face-tracking` のコードを import したり写したりしない (AGENTS.md)。見本の止め方は自分で書くか、CSP にする。

## 受け入れ条件

- [x] `bindings/web/examples/basic` で camera (MediaPipe) を使っても利用状況が送られない (`fetch` を包むか CSP)。止めていることを見本の README か
      bindings/web/README.md のトラッキングの説明に書いた
- [x] `packages/face-tracking` に、入れている `@mediapipe/tasks-vision` の配布物 (`vision_bundle.mjs` など) を調べて、送り先 (`odml.pa.googleapis.com`) と
      送り方 (`fetch(`) が T0943 で止めた形のままかを確かめる検査 (Vitest かスクリプト) があり、変わると落ちる。`just ci` から走る
- [x] packages/face-tracking/README.md の版を上げるときの手順を直した
- [x] `just ci` が通る

## 参考

- T0943 のメモ (送信の仕組み・止め方・起票の候補)
- bindings/web/examples/basic、bindings/web/src/tracking.ts、packages/face-tracking

## メモ

- T0040 で起票した (元の候補: T0943 の 2 つ)。
- やったこと:
  - 見本 (`bindings/web/examples/basic/index.html`): 「camera (MediaPipe)」で Face Landmarker を作る前に `refuseMediaPipeUsageLog()` が
    `window.fetch` を包み、ホストが `odml.pa.googleapis.com` の要求だけを `TypeError` で断る (1 回だけ包む。断った回数を
    `window.ugoeExample.usageLogsRefused` に数える)。CSP は付けていない (wasm・モデル・見本のモデルの読み込みを確かめずに
    `connect-src` を絞るのを避けた)。packages/face-tracking のコードは写さず、bindings/web/README.md にある例をもとに書いた (MIT)。
    bindings/web/README.md の「MediaPipe の利用状況の送信」の、見本が送るという記述を直した。
  - 検査: `packages/face-tracking/src/mediapipe-bundle.test.ts` (Vitest。`just ci` の web-check の `pnpm --filter @ugoe/face-tracking test`)。
    入れている `@mediapipe/tasks-vision` の `vision_bundle.{mjs,cjs,js}` について、中の URL が `https://odml.pa.googleapis.com/v1/log`
    の 1 つだけで `isMediaPipeUsageLog` が断ること、`url:"…/v1/log"` の要求を `fetch(<要求>.url, …)` で送ること、識別子の `fetch` が
    すべて直接の呼び出し (変数に取る・bind する・引数に渡すと落ちる) であること、`sendBeacon`・`XMLHttpRequest`・`WebSocket`・
    `EventSource` を含まないことを確かめる。`wasm/` の JS と wasm が送り先のホスト名を含まないことも確かめる。
  - packages/face-tracking/README.md: 「MediaPipe の版を上げるとき」(検査の中身と、版を上げる手順) を足した。
  - 確かめたこと: 見本の包みの部分を Node で切り出して動かし、送り先だけを断り、ほかの要求 (相対パス・`Request`) は通り、
    2 回呼んでも包みが重ならないことを見た。`just ci` が通った。
- やらなかったこと: 見本をブラウザで実際に動かして (カメラと 60 秒の待ち) 送信が断られることは確かめていない
  (`@ugoe/runtime` の tgz を作ってカメラを使う必要がある)。検査が変化で落ちることは、配布物 (pnpm のストアへのハードリンク) を
  書き換えられないので、実際の書き換えでは確かめていない (正規表現の判定は読んで確かめた)。
- 起票の候補:
  - bindings/web: 見本をヘッドレスの Chrome で開き、`createFromOptions` の後に `odml.pa.googleapis.com` への要求がネットワークに
    出ないことを確かめる自動のテスト (今は手で確かめるだけ)。
