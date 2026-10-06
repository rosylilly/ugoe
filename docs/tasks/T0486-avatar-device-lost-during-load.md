---
id: T0486
title: "<ugoe-avatar>: 読み込みの途中に GPU を失ったとき、作り直せるのに error を送らない (判断が要る)"
status: done
milestone: M9
crates: [bindings/web, docs]
depends_on: []
assignee: claude-worker-T0486
---

# T0486: <ugoe-avatar>: 読み込みの途中に GPU を失ったとき、作り直せるのに error を送らない (判断が要る)

## 目的

T0484 の適合テストで、`<ugoe-avatar>` が読み込み (`load_bundle`) や `#start` のモデルの適用の途中に GPU を失うと、`LoadError` (`device_lost`) の
`error` イベントを送った後に作り直して読み直すことが分かった (T0484 の候補。バグ寄り)。利用者には作り直せたのに `error` が届く。

**判断が要る:** 案:

- A. 読み込み・モデルの適用で `device_lost` を受けたら `error` を送らず、ふつうの喪失として作り直しを待つ (諦めたときだけ `error`)。
- B. `webglcontextlost` などの喪失の知らせを待ってから、喪失なら A、そうでなければ `error` を送る。
- C. 今のまま (`error` を送ってから作り直す) にして、README に「`device_lost` の `error` の後に作り直されることがある」と書く。

推奨は、`@ugoe/player` の `PuppetSurface` (T0946) の扱いを調べて、そろう案を書く。

## 受け入れ条件

- [x] 案と推奨をメモに書き、利用者の判断を得た
- [x] 決めた形で直し、偽物の WebPuppet で「読み込みの途中に喪失 → 作り直して読み直す」の順序のテストが `avatar.test.ts` にある
      (C なら文書だけ)
- [x] bindings/web/README.md (イベントの表) を直した
- [x] `just ci` と、T0484 の適合テスト (`just conformance --only avatar`) が通る

## 参考

- T0484 のメモ (内訳・起票の候補)、T0946 (`GpuLostRepeatedlyError`)
- bindings/web/src (`<ugoe-avatar>`)、bindings/web/README.md

## メモ

- T0041 で起票した (元の候補: T0484 (`load_bundle` の途中の喪失))。
- 調べたこと (T0486。判断のため):
  - wasm の `load_bundle`・`load_json` は、読む前に `is_lost()` を確かめて真なら `device_lost` を投げる (`crates/ugoe-wasm/src/web.rs` の
    `WebPuppet::load`)。`is_lost()` は WebGPU ではデバイスの喪失の知らせ、WebGL2 では canvas の `webglcontextlost` で立つ。つまり
    `device_lost` を受けた時点で喪失の知らせはもう届いている (`<ugoe-avatar>` の `webglcontextlost` のリスナーも同じ配送で動く。
    WebGPU は次のフレームの `is_lost()` で `#onLost` に入る)。
  - `@ugoe/player` の Player (T0938・T0941・T0946): `load` が `device_lost` を受けると、`this.error` に「GPU が止まりました」の文を
    出し、`#file` (前のモデル) は変えない。PuppetSurface は `checkLost` で作り直し、`#setUp` は前のモデル (無ければ何も) を読み直す。
    前のモデルがあれば `#afterLoad` が `error` を消すが、開こうとしたファイルは読み直さない (利用者が開き直す)。前のモデルが無ければ
    作り直した後も error が残る。PuppetSurface は読み込みに関わらず、`GpuLostRepeatedlyError` は諦めたときだけ (`failed`)。
    読み込みの途中の喪失を確かめるテストは player.test.ts に無い。
  - `<ugoe-avatar>` の今: `#fetchSrc`・`load()`・`#start` のモデルの適用が `device_lost` を受けると `error` を送る (`load()` は reject)。
    作り直しでは `#pendingModel ?? #model` を読み直すので、初めのモデルなら `src` を取り直して読める (T0484 の内訳) が、2 つ目以降の
    `src` を変えた途中に失うと、前のモデルが読み直されて `src` と表示が食い違う。
- 案 (利用者の判断を待つ):
  - A. 読み込み・`#start` のモデルの適用で `device_lost` を受けたら `error` を送らず (`load()` は reject せず)、そのモデルを
    `#pendingModel` に置いて作り直しを待つ。作り直した後にそのモデルを読む (`load()` の Promise はそこで解決)。諦めたときだけ
    `error` (`#fail` が `load()` も reject)。上の 2 つ目の食い違いも直る。
  - B. `webglcontextlost` などを待ってから判断する。ただし `device_lost` は `is_lost()` が真のときだけ投げられ、そのときには知らせは
    もう届いているので、待つものが無く A と同じになる (別の知らせを待つ分だけ複雑)。
  - C. 今のまま (`error` の後に作り直す) にして README に書く。利用者には作り直せたのに `error` が届き、`load()` は reject されるのに
    後から別の (前の) モデルが読まれることがある。
- 推奨: **A**。理由: 喪失は利用者のモデルの誤りではなく、`<ugoe-avatar>` は描画中の喪失を `gpulost` と作り直しで扱い `error` は諦めた
  ときだけ、という約束 (README) にそろう。PuppetSurface も `failed` は諦めたときだけで、喪失は `lost`/`waiting` で知らせる。B は A と
  同じ結果になり、C は約束と食い違う。Player は今「error の文を出して、開こうとしたファイルを捨てる」で、A とも C とも違う。
  A にするなら Player も同じに (読み込みの `device_lost` では error を出さず、開こうとしたファイルを作り直しの後に開く) そろえるのが
  よい (packages/player は `crates:` の外なので別タスク)。
- 起票の候補: (A に決めたら) Player: `load` が `device_lost` を受けたら error を出さずに、開こうとしたファイルを作り直しの後に開く
  (packages/player。player.test.ts に偽物の順序のテスト)。
- 利用者の判断 (T0486): 案 A。読み込み・モデルの適用で `device_lost` を受けたら `error` を送らず `load()` も reject しない。モデルを
  `#pendingModel` に置き、作り直した後に読み、`load()` の Promise はそこで解決する。`error` は作り直しを諦めたときだけ。`src` と表示の
  食い違いも直す。Player をそろえる件はオーケストレーターが別のタスクにする。
- やったこと (T0486、案 A):
  - `<ugoe-avatar>` (bindings/web/src/avatar.ts): 読み込み (`load()`・`src` の取得の結果・`#start` のモデルの適用) が投げたとき、
    `puppet.is_lost()` が真なら (`device_lost`) 失敗とせず、モデルを `#pendingModel` に置いて `requestRender` する (`#applyModelOrDefer`)。
    作り直しは描画の喪失と同じ道 (WebGL2 は `webglcontextlost` からの復元の待ち、WebGPU は次のフレームの `is_lost()`) で始まり、
    `#start` がそのモデルを読む。`load()` の Promise はそこで解決し、諦めたら `#fail` が reject する。`#recreate` は `#pendingModel`
    があればパラメータの値を覚え直さない (別のモデルに前の値を移さない。読み直しの途中にまた失ったときは、前に覚えた値を保つ)。
  - avatar.test.ts: 偽物の `loadModel` が失った WebPuppet で `device_lost` を投げるようにし、5 つのテスト (`load()` の待ちと解決・
    失っている間に終わった `src` を作り直した後に読む・WebGPU・読み直しの途中にまた失っても値を保つ・諦めたら `load()` を reject)。
    修正の前には 5 つとも失敗することを確かめた。
  - 適合テスト (`crates:` の外の小さな変更): tests/conformance/web/avatar.js にシナリオ `context-loss-during-load` (本物の wasm で
    `loseContext()` の間の `load()` と `src`) を足し、tests/conformance/README.md の表と注記を直した。
  - bindings/web/README.md (イベントと失敗とやり直しの節) を今の形に書き直した。
  - `just ci` と `just conformance` (全部。avatar 10/10) が通った。
- 確かめていないこと: 本物の WebGPU のデバイスの喪失 (ページから起こせない。偽物のテストだけ)。
- 起票の候補: (オーケストレーターが起票する) Player をそろえる (読み込みの `device_lost` では error を出さずに、開こうとしたファイルを作り直しの後に開く)。
