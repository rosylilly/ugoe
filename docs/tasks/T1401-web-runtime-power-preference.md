---
id: T1401
title: "@ugoe/runtime: createPuppet の options と <ugoe-avatar> の属性に GPU の電力の希望 (powerPreference) を足す (判断が要る)"
status: in-progress
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

- [ ] 決めたことを bindings/web/README.md と crates/ugoe-wasm/README.md の `powerPreference` の節 (「`@ugoe/runtime` の … はまだ渡さない」の行) に書いた
- [ ] `createPuppet` の options に `powerPreference` を足し、`WebPuppet.create` に渡す。省いたときは今までどおり (`"default"`)
- [ ] `<ugoe-avatar>` に `power-preference` の属性と `powerPreference` のプロパティを足した。作るときと、GPU を失って作り直すときに渡し、
      決めたとおりに変更を扱う。決めたなら `adapterName` も足した
- [ ] テスト (偽の wasm): 属性・プロパティの値が `WebPuppet.create` に渡ること、作り直しでも同じ値が渡ること、変えたときの振る舞い、
      知らない値の扱い。型 (index.ts の公開の型) に出ていること
- [ ] `just ci` が通る

## 参考

- T0499 のメモ (起票の候補)、T0076 のメモ (どの環境で効くかの表)
- crates/ugoe-wasm/README.md (`powerPreference` の節)、bindings/web/src/runtime.ts (`createPuppet`)・avatar.ts・attributes.ts、
  packages/player/src/surface.ts (`powerPreference` の渡し方の見本)

## メモ

- T0077 で起票した (元の候補: T0499 と T0076 (`@ugoe/runtime` の `createPuppet` と `<ugoe-avatar>` に `powerPreference` を足す))。同じ候補なので 1 つにした。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- 効くかはブラウザと OS が決める (Windows の Chromium は WebGPU の希望を無視する、など)。2 つの GPU の実機では確かめられない。
- 判断 (2026-10-08): 利用者が推奨の形 (名前は powerPreference / power-preference、作った後に変えたら作り直す、adapterName を足す、知らない値は default として警告) を選んだ。
