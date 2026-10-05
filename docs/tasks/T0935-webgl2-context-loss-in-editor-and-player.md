---
id: T0935
title: エディタと Player でも WebGL2 の喪失で復元を待つか、続けて失敗したときの諦め方を決める (判断が要る)
status: done
milestone: M9
crates: [packages/player, apps/editor, docs]
depends_on: []
assignee: claude-worker-T0935
---

# T0935: エディタと Player でも WebGL2 の喪失で復元を待つか、続けて失敗したときの諦め方を決める (判断が要る)

## 目的

T0932 で WebPuppet の作り直しを共有したが、WebGL2 のコンテキストを失ったときの扱いが `<ugoe-avatar>` (復元を待つ) と、エディタ・Player で違う。
作り直しが続けて失敗したときの諦め方 (回数・知らせ方) も決まっていない (T0932 の候補)。

**判断が要る:** エディタと Player を `<ugoe-avatar>` に揃えるか。諦めるまでの回数と、諦めたときの画面の知らせ。

## 受け入れ条件

- [x] 方針を決め、docs (docs/ui.md か packages/player の README) に書いた (利用者の判断を待つなら blocked にしてメモに案を書く)
- [x] 方針どおりに直し、喪失・復元・続けての失敗をテストする (偽の WebGL2 のコンテキストで)
- [x] `just ci` が通る

## 参考

- T0932 のメモ (起票の候補)、T0697

## メモ

- T0026 で起票した (元の候補: T0932 の WebGL2 の喪失)。

### 決めたこと (利用者の判断を待たずに決めた根拠)

- **`<ugoe-avatar>` に揃えた。** 既に決まっている前例 (T0425・T0439 の `<ugoe-avatar>`: 復元を待つ・前の作り直しから 1 分以内に
  続けて 4 回目で諦める) があり、T0932 のメモが「本物の GPU のリセットでは、復元の前に新しいコンテキストを作れないことがある
  (そのときは「描画を始められない」になる)」と、すぐ作り直す方式の欠点を挙げていた。揃えない理由 (エディタ・Player 固有の事情) は
  見当たらなかったので、新しい方針を作らずに前例に合わせた。数 (`MAX_RESTARTS` = 3) と時間 (`RESTART_WINDOW_MS` = 60 秒) も同じ。
- **諦めたときの画面の知らせ:** Player は `status = "unavailable"`、`error` にカタログの `errors.gpuLostRepeatedly`
  (「GPU を続けて失ったので、描画を止めました。描き直すには、アプリを起動し直してください (ブラウザではページを読み込み直してください)」。
  既にある `loadError.deviceLost` の文の流儀)。エディタはビューポートの `status = "error"`、`viewport.gpuLostRepeatedly`
  (「… エディタを起動し直してください」)。作り直しのボタンは置かない (下の候補)。

### やったこと

- `PuppetSurface` (packages/player/src/surface.ts): `webglcontextlost` を `preventDefault` して `awaitingRestore` を立て、
  `webglcontextrestored` で作り直す。待つ間は `checkLost()` が真 (描かない) で作り直しを始めない。WebGPU (`is_lost()` だけ) はすぐ
  作り直す (前のまま)。作り直しを数え、上限を超えたら `failed(new GpuLostRepeatedlyError())`。`attach` で数え直す。
  `MAX_RESTARTS`・`RESTART_WINDOW_MS`・`GpuLostRepeatedlyError` を公開した。
- Player と エディタの `ViewportController` の `failed` で、`GpuLostRepeatedlyError` ならカタログの文を出す。
- テスト (偽の wasm と canvas): surface.test.ts に復元を待つこと・関係の無い `webglcontextrestored` を無視すること・続けて失ったら諦めて
  次の `attach` で数え直すこと・`GpuLostRepeatedlyError` を渡すこと・1 分より離れた喪失は数えないこと。player.test.ts と
  エディタの controller.test.ts の喪失のテストを、復元を待つ形に直し、続けて失ったときの画面の知らせのテストを足した。
- 文書: docs/ui.md#webpuppet-の寿命 (方針と根拠、`<ugoe-avatar>` との関係)、docs/editor.md#ビューポート、packages/player/README.md、
  docs/workflow/code-review.md の表。

### 確かめていないこと

- 本物の wasm とブラウザ (WebGL2) での喪失と復元は確かめていない (偽のコンテキストのテストだけ)。Tauri の WebView
  (WebKitGTK など) で `preventDefault` した後に `webglcontextrestored` が実際に届くかも確かめていない。届かない環境では、
  喪失の後は描かないまま (知らせも出ない) になる (`<ugoe-avatar>` と同じ)。

### 検査

- `git rebase main` の後に `just ci` (前面、終了コード 0)。ランタイム層は触っていない。

### 起票の候補

- 復元 (`webglcontextrestored`) が届かないまま長く待つときの扱い (時間切れで作り直しを試す・「GPU の復元を待っています」と出す)。
  `<ugoe-avatar>` と一緒に決める。
- 諦めた後に、起動し直さずに作り直せるボタン (Player・エディタのビューポート。`<ugoe-avatar>` の `restart()` に当たるもの)。
- `PuppetSurface` を本物の wasm とブラウザで確かめる自動のテスト (T0932 の候補のまま)。
