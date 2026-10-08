---
id: T1401
title: "@ugoe/runtime: createPuppet の options と <ugoe-avatar> の属性に GPU の電力の希望 (powerPreference) を足す (判断が要る)"
status: done
milestone: M8
crates: [bindings/web, docs]
depends_on: []
assignee: claude-worker-T1401
---

# T1401: @ugoe/runtime: powerPreference を足す

## 目的

ugoe-wasm の `WebPuppet.create` は GPU の電力の希望 (`power_preference`: `"default"`・`"low-power"`・`"high-performance"`) と、使っている GPU の名前
(`adapter_name()`) を持つ (T0499。公開の API)。`@ugoe/player` (スタジオ・ビューア) は渡しているが、npm の `@ugoe/runtime` の `createPuppet` と
`<ugoe-avatar>` はまだ渡さない (crates/ugoe-wasm/README.md の `powerPreference` の節)。ページに埋め込む利用者も、省電力の GPU で描く・
外付けの GPU を希望する、を選べるようにする。

**判断が要る (公開 API に足す)。** 作業者は下の推奨で進めてよいが、違う形にするなら利用者に確かめる。

### 決めること

1. **名前:** `createPuppet(canvas, { powerPreference })` と、`<ugoe-avatar power-preference="low-power">` (JS のプロパティは `powerPreference`)。
   **推奨:** この名前 (WebGPU・WebGL の `powerPreference` と同じ。`backend` の属性と同じ書き方)。
2. **作った後に変えたとき:** 希望はアダプタとコンテキストを作るときにしか効かない (WebGL2 は同じ canvas の 2 回目の `getContext` が属性を無視する)。
   案 A: `backend` の変更と同じく作り直す。案 B: 次に作り直すとき (GPU を失ったときなど) から効く。**推奨:** A (`backend` と同じ振る舞いで、効いたかがすぐ分かる)。
3. **GPU の名前を出すか:** `<ugoe-avatar>` に `adapterName` (読むだけのプロパティ。作る前と切断中は `null`) を足す。**推奨:** 足す
   (希望が効いたかを利用者が確かめる手段。`WebPuppet.adapter_name()` を直接呼べば今でも読めるが、要素から読めるほうがよい)。
4. **知らない値:** `backend` の属性と同じく、知らない値は `"default"` として扱い警告を出す (attributes.ts の `parseBackend` に合わせる)。

## 受け入れ条件

- [x] 決めたことを bindings/web/README.md と crates/ugoe-wasm/README.md の `powerPreference` の節 (「`@ugoe/runtime` の … はまだ渡さない」の行) に書いた
- [x] `createPuppet` の options に `powerPreference` を足し、`WebPuppet.create` に渡す。省いたときは今までどおり (`"default"`)
- [x] `<ugoe-avatar>` に `power-preference` の属性と `powerPreference` のプロパティを足した。作るときと、GPU を失って作り直すときに渡し、
      決めたとおりに変更を扱う。決めたなら `adapterName` も足した
- [x] テスト (偽の wasm): 属性・プロパティの値が `WebPuppet.create` に渡ること、作り直しでも同じ値が渡ること、変えたときの振る舞い、
      知らない値の扱い。型 (index.ts の公開の型) に出ていること
- [x] `just ci` が通る

## 参考

- T0499 のメモ (起票の候補)、T0076 のメモ (どの環境で効くかの表)
- crates/ugoe-wasm/README.md (`powerPreference` の節)、bindings/web/src/runtime.ts (`createPuppet`)・avatar.ts・attributes.ts、
  packages/player/src/surface.ts (`powerPreference` の渡し方の見本)

## メモ

- T0077 で起票した (元の候補: T0499 と T0076 (`@ugoe/runtime` の `createPuppet` と `<ugoe-avatar>` に `powerPreference` を足す))。同じ候補なので 1 つにした。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- 効くかはブラウザと OS が決める (Windows の Chromium は WebGPU の希望を無視する、など)。2 つの GPU の実機では確かめられない。
- 判断 (2026-10-08): 利用者が推奨の形 (名前は powerPreference / power-preference、作った後に変えたら作り直す、adapterName を足す、知らない値は default として警告) を選んだ。
- やったこと (2026-10-08):
  - `createPuppet` の options に `powerPreference` (`PowerPreference` 型。index.ts から出す) を足し、`WebPuppet.create` の 3 つ目の引数に
    渡す (省くと `"default"`)。知らない値は wasm の `Error` のまま (TS の型で弾く)。
  - `<ugoe-avatar>` に `power-preference` 属性と `powerPreference` プロパティ (読むと解釈した値。HTML の列挙の属性の反映と同じく、知らない値は
    `"default"`。代入は属性を書く) を足した。作るとき・`auto` で WebGL2 に切り替えるとき・GPU を失って作り直すときに渡す。
    変えたら作り直す (`backend` と同じ)。ただし解釈した値が変わらない書き換え (大文字小文字・空白・知らない値 ↔ `default`) では作り直さない。
  - 知らない値は `"default"` として `console.warn` (`unknown power-preference "…"`)。同じ値で作り直すたびに繰り返さないよう、値が変わったときに 1 度だけ。
    判定は attributes.ts の `parsePowerPreference`・`isUnknownPowerPreference`。
  - `adapterName` (読むだけ。`WebPuppet.adapter_name()`。作成前と切断中は `null`、ブラウザが伏せると `""`) を足した。
  - テスト: attributes.test.ts (解釈と知らない値)、runtime.test.ts (新規。偽の wasm で `WebPuppet.create` に渡る引数)、avatar.test.ts
    (属性・プロパティ・WebGL2 への切り替え・GPU の喪失からの作り直し・変更での作り直しと引き継ぎ・諦めた後の変更・知らない値と警告・型)。
  - 文書: bindings/web/README.md (属性の表・JS の例・「GPU の電力の希望」の節)、crates/ugoe-wasm/README.md (`powerPreference` の節の
    「まだ渡さない」の行。`crates:` の外だが受け入れ条件にあるので直した)。
  - `just wasm-types`・`just conformance --only avatar,example` が通る。
- やらなかったこと: 見本のページ (examples/basic) に希望を選ぶ部品と GPU の名前の表示は足していない。適合テスト (tests/conformance/web/avatar.js) に
  本物の wasm で `power-preference` を変えて作り直すシナリオも足していない (`crates:` の外。偽の wasm のテストで振る舞いは確かめた)。
  2 つの GPU を持つ実機で希望が効くかは確かめていない。
- 起票の候補:
  - examples/basic に `power-preference` の選択と `adapterName` の表示を足し、tests/conformance (avatar) に `power-preference` の変更で
    作り直して描けるシナリオを足す。
