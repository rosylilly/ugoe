---
id: T0465
title: wasm の描画・読み戻しの GPU の喪失にコードを付けるか決め、@ugoe/player の着せ替えの失敗の訳に device_lost を足す (判断が要る)
status: done
milestone: M9
crates: [ugoe-wasm, bindings/web, packages/player, docs]
depends_on: [T0934]
assignee: claude-worker-T0465
---

# T0465: wasm の描画・読み戻しの GPU の喪失にコードを付けるか決め、@ugoe/player の着せ替えの失敗の訳に device_lost を足す (判断が要る)

## 目的

T0457 で wasm の着せ替えの GPU の喪失に `device_lost` のコードを付けた。次が残っている (T0457 の候補)。

1. `@ugoe/player` の着せ替えの失敗の訳 (`packages/player/src/dressUp.ts`) に `device_lost` が無い (読み込みの `loadError.deviceLost` と同じ文でよい)。
2. **判断が要る:** wasm の `render`・`read_pixels` など、ほかの GPU の喪失の例外にもコード (と JS のエラーの名前) を付けるか。付けると `@ugoe/runtime` の公開の型が増える。

訳の表を触る T0934 の後に着手する。

## 受け入れ条件

- [x] `@ugoe/player` の着せ替えの失敗の訳に `device_lost` があり、テストがある
- [x] ほかの GPU の喪失の例外にコードを付けるかの案と推奨をメモに書き、利用者の判断を得た。付けるなら crates/ugoe-wasm/README.md・`bindings/web` の型・`codes.json` を揃え、テストがある
- [x] `just ci` と `just wasm-check` が通る

## 参考

- T0457 のメモ (起票の候補)
- crates/ugoe-wasm/README.md、packages/player/src/loadError.ts

## メモ

- T0032 で起票した (元の候補: T0457 (player・wasm の GPU の喪失))。
- 判断 (2026-10-05、利用者の判断): wasm の `render`・`read_pixels` などの GPU の喪失の例外にもコードと JS のエラーの名前を付ける (`@ugoe/runtime` の型も足す)。
- やったこと (2026-10-05):
  - 着せ替えの訳の `device_lost` は T0934 で `dressUp.ts` に入っていた (`loadError.deviceLost` と同じ文。全コードの訳は `codes.test.ts` が見る)。
    重ねずに、`dressUp.test.ts` に読み込みと同じ文になることのテストだけを足した。
  - wasm: `src/frame.rs` に `FrameError` (今は `DeviceLost` だけ。JS の `name` は `"RenderError"`、`code` は `device_lost`) を置き、
    `render`・`read_pixels` (呼んだときと、読み戻しを待つ間に失ったとき) がそれを投げる (戻り値の型を `Result<_, JsValue>` に。
    生成される `.d.ts` は変わらない。`just wasm` の後の `just wasm-types` で確かめた)。文は前と同じ `the GPU device was lost`。
    ほかの描画の失敗 (canvas のテクスチャ・レンダラ・読み戻しの形式・時間切れ) はコードを持たない `Error` のまま (README に書いた)。
  - `codes.json` に `render_error` を足し、`src/codes.rs` の見本と網羅の `match` を足した。
  - `@ugoe/runtime`: `src/render-errors.ts` (`RENDER_ERROR_CODES`・`UgoeRenderErrorCode`・`UgoeRenderError`・`isRenderError`) と
    テスト、`codes.test.ts` の突き合わせ、README の節。
  - `@ugoe/player`: `describeRenderError` (`loadError.ts`。`device_lost` は読み込みと同じ文) とテスト、`codes.test.ts` で全言語の訳を確かめる。
  - 適合テスト (`tests/conformance/web/avatar.js` の `context-loss`。`crates:` の外の小さな変更): WebGL2 のコンテキストを失った後の
    `render`・`read_pixels` が `RenderError`/`device_lost` を投げることを本物の wasm で確かめる。`just conformance` は全部通った。
  - `just ci`・`just wasm-check`・`just wasm-types` (生成物と比べて)・`just conformance` が通る。
- 確かめていないこと: WebGPU の実際のデバイスの喪失 (GPU プロセスのクラッシュ) での `RenderError` (喪失の印は WebGL2 と同じ `lost` なので同じ道を通る)。
- 起票の候補:
  - ビューアとエディタのスクリーンショットの失敗 (`apps/viewer/src/App.svelte` の `saveScreenshot`・エディタの `saveScreenshot.ts`) で
    `RenderError` を訳す (ビューアは `describeRenderError`、エディタは `preview.dressUp.problem.deviceLost` のような自分のカタログの文)。今は英語の文のまま出る。
