---
id: T0487
title: "bindings/web: 見本をヘッドレスの Chrome で開き、MediaPipe の利用状況の送信が出ないことを自動で確かめる"
status: done
milestone: M9
crates: [bindings/web, tests/conformance, docs]
depends_on: []
assignee: claude-worker-T0487
---

# T0487: bindings/web: 見本をヘッドレスの Chrome で開き、MediaPipe の利用状況の送信が出ないことを自動で確かめる

## 目的

T0947 で、bindings/web の見本でも MediaPipe の利用状況の送信 (`odml.pa.googleapis.com`) を止め、版を上げたときにバンドルの送り方の変化を見つける検査を足した。
見本を開いて実際に要求が出ないことは手で確かめただけ (T0947 の候補)。

## 受け入れ条件

- [x] bindings/web の見本をヘッドレスの Chrome で開き、顔のトラッキングの `FaceLandmarker` を作った (`createFromOptions` の) 後に、
      `odml.pa.googleapis.com` への要求がネットワークに出ないことを確かめる自動のテストがある (カメラは偽の映像。ネットワークの記録の取り方をメモに書く)
- [x] 止める処理を一時的に外すとテストが落ちることを確かめ、メモに書いた
- [x] テストの走らせ方を docs/testing.md・tests/conformance/README.md (適合テストに置くなら)・bindings/web/README.md に書き、CI のどこで走るか (`conformance` のジョブなど) を決めてメモに書いた
- [x] `just ci` が通る

## 参考

- T0947 のメモ (やったこと・起票の候補)
- T0484 (ヘッドレスの Chrome を使う適合テストの前例)、tests/conformance/web/avatar.js・tests/conformance/README.md

## メモ

- T0041 で起票した (元の候補: T0947 (見本の自動のテスト))。
- やったこと:
  - 適合テストに `example` を足した (`just conformance --only example`。tests/conformance/example.mjs、run.mjs の `WEB`・`NO_REFERENCE` に追加)。
    見本 (bindings/web/examples/basic) は書き換えずに配り (import map の `./node_modules/@ugoe/runtime/` は bindings/web、`/samples/` は assets/samples)、
    ヘッドレスの Chrome (`--use-fake-device-for-media-stream`・`--use-fake-ui-for-media-stream`、WebGL2 は SwiftShader) で `?backend=webgl2` を開き、
    `<ugoe-avatar>` の `load` を待ってから「camera (MediaPipe)」を押し、`cameraFrames` が 1 以上 (= `createFromOptions` の後で、偽の映像を調べている) になるのを待つ。
  - ネットワークの記録の取り方: `--remote-debugging-port=0` のブラウザの CDP からページに `Target.attachToTarget` (`flatten`) で繋ぎ、
    `Network.enable` の `Network.requestWillBeSent` で、ページが出した要求をすべて記録する (Fetch で止めた要求も記録される)。見本の `fetch` の包みが
    断った要求はネットワークの層に届かないので記録されない。送り先 (`odml.pa.googleapis.com`) への記録が 1 件でもあれば失敗。
  - ネットワークに頼らない: CDP の `Fetch.enable` (`https://*`) で外への要求を止め、jsdelivr の `@mediapipe/tasks-vision@1.0.1` は packages/face-tracking が
    入れているパッケージ (見本の版と違えば止まる)、モデルは packages/face-tracking の `mediapipe-assets.mjs` が取得して SHA-256 を確かめたもの
    (`target/conformance-mediapipe/`。run.mjs は毎回 `target/conformance/` を消すのでその外) で応える。GPL の packages/ のコードは写さず、パッケージと
    スクリプトを実行するだけ (tests/conformance は MIT)。
  - MediaPipe は 60 秒ごとの `setInterval` で送るので、`Page.addScriptToEvaluateOnNewDocument` (`Page.enable` が要る) で 60000 ミリ秒の `setInterval` を 1 秒に縮める。
  - 2 回開く: `guarded` (見本のまま。断った回数 `usageLogsRefused` が 1 以上になるのを待ち、さらに 3 秒待って記録が 0 件) と、対照の `unguarded`
    (`fetch` に包んだ印 `refusesMediaPipeUsageLog` を先に付けて見本の包みを外し、送り先への要求が記録されることを確かめる。要求は Fetch で
    `BlockedByClient` にするので実際には送らない)。対照により、テストが送信を見つけられることを毎回確かめる。
  - run.mjs の `browserCdp` を tests/conformance/cdp.mjs に移し、イベントとセッション (`sessionId`) を扱えるようにした (avatar はそのまま通る)。
  - 文書: tests/conformance/README.md (表・実行・出力・CI・「見本の MediaPipe の利用状況の送信」の節)、docs/testing.md#配布物の適合テスト、
    bindings/web/README.md の「MediaPipe の利用状況の送信」。justfile の `conformance` のコメントに 1 行足した (範囲外だが小さなコメントだけ)。
- 壊れを見つけること: 見本の `refuseMediaPipeUsageLog();` の呼び出しをコメントにすると `guarded` が落ちた (送り先への要求が 2 件記録された。
  1 回の送信で 2 件記録される)。戻して通ることを確かめた。
- CI: `conformance` ジョブの `just conformance` (すべてのバインディング) に入るので、ジョブは変えていない。モデルの取得 (最初の 1 回。storage.googleapis.com)
  と `pnpm install` 済みの packages/face-tracking が要る (ジョブはどちらもできる)。
- 確かめたこと: `just conformance --only example` を 6 回ほど (すべて通った)、`just conformance --only avatar,queries,example`、`just ci`。
- 確かめていないこと: CI のランナーの上では動かしていない (push しないため)。本物のカメラ・Chrome 以外のブラウザ。
  最初の版では `window.ugoeExample` (チェックボックスの処理を付ける前にできる) を待ってから押していたため、押す時機によってカメラが始まらないことがあった。
  `<ugoe-avatar>` の `load` を待つように直してからは起きていない。
- 起票の候補:
  - bindings/web の見本: `window.ugoeExample` を、操作の処理をすべて付けた後に置く (または `ready` の印を足す) と、自動のテストが待つものが明確になる。
