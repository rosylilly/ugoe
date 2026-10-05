---
id: T0941
title: "@ugoe/player とエディタで CreateError を訳し、エディタのビューポートの device_lost をエディタ向けの文にする"
status: done
milestone: M9
crates: [packages/player, apps/editor, docs]
depends_on: []
assignee: claude-worker-T0941
---

# T0941: @ugoe/player とエディタで CreateError を訳し、エディタのビューポートの device_lost をエディタ向けの文にする

## 目的

- T0475 で、wasm の `WebPuppet.create` は GPU・ブラウザの都合の失敗を `CreateError` (`backend_unavailable`・`context_unavailable`・
  `adapter_unavailable`・`device_unavailable`) で投げるようになったが、`Player` は `errors.renderStart` に英語の `message` を埋めて出し、
  エディタのビューポートは `errorText(e)` (英語) を出す (T0475 の候補。T0940 の `RenderError` と同じ形で訳す)。
- エディタのビューポートの読み込みの失敗 (`ViewportController` の `describeLoadError`) は、`device_lost` で `@ugoe/player` のカタログの
  「アプリを起動し直して」の文になる。エディタ向けの文 (エディタを起動し直す) に揃える (T0940 の候補)。

## 受け入れ条件

- [x] `@ugoe/player` に `CreateError` の訳 (例: `describeCreateError`) を足し、`Player` の作成の失敗が `CreateError` なら訳した文を出す
      (`context_unavailable` なら「ほかのタブを閉じるか、少し待って作り直してください」のような文)。Vitest で確かめる
- [x] エディタのビューポートの作成の失敗が `CreateError` なら、エディタのカタログの文を出す。読み込みの失敗の `device_lost` もエディタのカタログの文
      (`viewport.screenshot.deviceLost` と同じ文など) を出す。関数か部品のテストがある
- [x] 文言は日英のカタログにある。docs/ui.md#webpuppet-の寿命 (または docs/editor.md) に、どの文がどこから来るかを書いた
- [x] `just ci` が通る

## 参考

- T0475・T0940 のメモ (起票の候補)
- bindings/web/src/create-errors.ts、packages/player/src/loadError.ts (`describeRenderError`・`describeLoadError`)、apps/editor/src/lib/viewport/controller.svelte.ts・saveScreenshot.ts

## メモ

- T0038 で起票した (元の候補: T0475 (`CreateError` の訳)・T0940 (エディタの `device_lost`))。
- T0941 (やったこと):
  - `@ugoe/player` の `loadError.ts` に `describeCreateError` (知らないコード・ほかの例外は `null`) と `isCreateFailure` を足し、
    カタログ `createError.*` (日英) を足した。`Player` は作れなかったときに `CreateError` ならこの文を `error` に出し (ほかは今までどおり
    `errors.renderStart`)、`CreateError` なら (知らないコードでも) `canRestart` を真にして「作り直す」ボタンを出す (待てば作れることがあるため。
    文も「少し待ってから「作り直す」を押してください」にした)。`codes.test.ts` に `create_error` のすべてのコードの訳の検査を足した。
  - エディタは `src/lib/viewport/viewportErrors.ts` の `createErrorText` (`viewport.createError.*`) と `loadErrorText` (`device_lost` は
    `viewport.deviceLost`、ほかは `describeLoadError`) を `ViewportController` で使う。`CreateError` ならボタンも出す。
    スクリーンショットの `viewport.screenshot.deviceLost` を `viewport.deviceLost` に移して、読み込みと同じキーにした。
    `loadError.test.ts` は `viewportErrors.test.ts` に名前を変え、作成の失敗のすべてのコードの訳と `device_lost` の文を確かめる。
    `controller.test.ts` に作れなかったとき (訳・ボタン・作り直し) とコードの無い例外 (例外の文・ボタンなし) を足した。
  - docs/ui.md#webpuppet-の寿命 に、どの文がどこ (カタログ・関数) から来るかの表と、`CreateError` でボタンを出すことを書いた。
    docs/editor.md・packages/player/README.md も直した。範囲の外の小さな変更: crates/ugoe-wasm/README.md#作成の失敗-t0475 に訳す関数を 1 行足した。
  - T0479 の候補 (`#restoreTimer` に 2 つのタイマー) を `PuppetSurface` で確かめた: 同じ形ではない。`#restoreTimer` を置くのは
    `#onContextLost` だけで、`#awaitingRestore` が偽のときだけ置き、時間切れ・復元・`#release` で消す。作り直しで作れなかったときの待ちは
    `#recreate` の `sleep` (別のタイマー) で、番号で古いものを捨てる。直すところは無かった。
- やらなかったこと:
  - 実際の GPU・ブラウザで `CreateError` を起こしての確認はしていない (偽の wasm でのテストだけ)。
  - 最初の作成の失敗の後に「作り直す」を押してまた作れないと、`#recreate` が待って試し直したあと `GpuLostRepeatedlyError`
    (「GPU を続けて失った」) の文になる。作れなかった理由の文にはならない。
  - ビューポートの描画 (`#render`) の失敗は今までどおり `errorText` (`RenderError` の `device_lost` は先に `checkLost` が作り直すので出にくい)。
- 起票の候補:
  - `PuppetSurface.restart`・`#recreate` が諦めたとき、最後の `CreateError` を `GpuLostRepeatedlyError` の `cause` に持たせ、
    最初の作成の失敗から作り直したときは作れなかった理由の文を出す (Player・エディタ)。
  - `<ugoe-avatar>` (bindings/web) の作り直しが `CreateError` で失敗したときに `#restoreTimer` を上書きする件 (T0479 の候補。範囲外で未確認)。
  - ビューアの `Viewport.svelte` のコメント・ビューアのテストに、`CreateError` でボタンが出ることを足す (apps/viewer は範囲外で、
    振る舞いは `Player.canRestart` を見るので変更は不要)。
