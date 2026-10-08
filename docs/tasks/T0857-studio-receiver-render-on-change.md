---
id: T0857
title: "スタジオ: 出力ウィンドウとブラウザソースの受け手は、新しいフレームか見せ方の変化があったときだけ描く"
status: done
milestone: M8
crates: [packages/player, apps/studio, docs, tests/ui-audit]
depends_on: []
assignee: claude-worker-T0857
---

# T0857: スタジオ: 受け手は変わったときだけ描く

## 目的

利用者の要望「GPU の使用量を減らす (描く量で負荷を抑える)」の 1 つ目 (T0076 で調べた)。

出力ウィンドウとブラウザソースのページの受け手 (`@ugoe/player` の Player の受け手のモード) は、届いたフレームを届いたときに
`follow` まで適用し、描くのは `requestAnimationFrame` の `#tick` で行う。ところが `#tick` は **新しいフレームが無くても毎回 `render()` を呼ぶ**
(`packages/player/src/player.svelte.ts` の `#tick`)。`requestAnimationFrame` は画面のリフレッシュレートで回るので、144 Hz の画面で刻みの速さが
30 なら、同じ形を 1 秒に 114 回余計に描いている。受け手は指揮役より速く描いても新しい形は出ない (docs/studio.md#出力ウィンドウ) ので、
これは GPU の無駄で、配信中のゲームや OBS と GPU を取り合う原因になる。

受け手のモードでは、前に描いてから **新しいフレームを適用した・見せ方 (ビュー・背景・光源・大きさ・着せ替え・ポーズ・シーン) が変わった・
GPU を失って作り直した** ときだけ描くようにする。刻みの速さ (30・60・120) がそのまま描く回数の上限になる (「フレームレートの上限」は
今の刻みの速さの設定で足りる。新しい設定は足さない)。

## 受け入れ条件

- [x] Player の受け手のモードで、描く必要があるか (汚れ) の印を持ち、`#tick` は印が立っているときだけ `render()` を呼ぶ。
      印を立てるのは: `follow` でフレームを 1 つ以上適用した・ビュー (fit・ズーム・パン)・大きさ・devicePixelRatio・背景・光源・
      着せ替え・ポーズ・パーツの見え方が変わった・モデルを読んだ・WebPuppet を作り直した。受け手のモードでないとき (ビューアなど) の振る舞いは変えない
- [x] 描く回数の報告 (`startFollowing` のコールバック。「フレームレートと負荷」の描画の回数・1 回の時間・捨てたフレーム) は、描かなかった
      `requestAnimationFrame` でも約 1 秒ごとに届く (描かない間に報告が途絶えて「描いていない」の注意が出ない)。描いた回数は実際に描いた回数を数える
- [x] 「フレームレートと負荷」の注意の判断 (`apps/studio/src/lib/load.ts`。回数が刻みの速さの 95% を下回る) が、変わったときだけ描くことと
      食い違わないことを確かめた (刻みの速さ以下の回数で描くので、目標は刻みの速さのまま。画面のリフレッシュレートが刻みの速さより低い
      ときに注意が出るのは今と同じ。食い違えば判断を直し、docs/studio.md#負荷の表示 に書く)
- [x] テスト: 偽の WebPuppet で、フレームが来ない `requestAnimationFrame` では `render()` を呼ばないこと、フレームを適用した・ビューを変えた・
      背景を変えた・作り直した後の最初の `#tick` では呼ぶこと、受け手のモードでなければ毎回呼ぶこと、報告が描かなくても届くこと
- [x] 出力ウィンドウ・ブラウザソースの画の確かめ (docs/studio.md#画の確かめ-adr-0028-の-10) の手順で、止めた後の画が今と同じであることを確かめた
      (止めた後にフレームが来なくても、最後の形が残っている)
- [x] docs/studio.md の「出力ウィンドウ」と「負荷の表示」に、受け手は変わったときだけ描くこと (描く回数の上限は刻みの速さ) を書いた。
      packages/player/README.md の受け手のモードの説明も直した
- [x] `just ci` が通る (画面の並びとクラス名は変えないので e2e は要らない。変えたら `just e2e --only studio`)

### 確かめていないこと (実機)

- 実際の GPU で、GPU の使用率 (Windows のタスクマネージャーの GPU の列など) が刻みの速さに見合って下がること。実機の確認表
  (docs/manual-check.md の「S. スタジオ」) に手順を足す

## 参考

- docs/studio.md#出力ウィンドウ・#負荷の表示・#隠れても止まらない、packages/player/src/player.svelte.ts (`#tick`・`pushFrame`・`#followPending`)
- apps/studio/src/Output.svelte (`startFollowing`)、apps/studio/src/lib/load.ts
- T0076 のメモ (調べた結果)

## メモ

- T0076 で起票した (利用者の要望 2026-10-08「GPU の優先度」の 3. 描く量で負荷を抑える)。依存・unsafe・OS の設定は要らないので判断は要らない。
- WebGPU の canvas は描かなければ前の画を表示し続ける。WebGL2 の canvas も `preserveDrawingBuffer` に関わらず、描かなかったフレームは
  合成済みの画が残る (描かなければ canvas は「変わっていない」として前の画を出す。HTML の仕様の「canvas を更新する」は描いたときだけ起きる)。
  念のため両方のバックエンドで止めた後の画を確かめる (上の受け入れ条件)。

### やったこと (2026-10-08)

- **Player** (`packages/player/src/player.svelte.ts`): 描き直しの印 `#dirty` を持ち、受け手のモードの `#tick` は印が立っているときだけ `render()` を呼ぶ。
  印を立てるのは `#follow` でフレームを 1 つ以上取り出した・`#syncView` (fit・ズーム・パン・`resize`・devicePixelRatio・読んだ後)・
  `#applyBackground`・`#applyLighting`・`#syncDressUp` (付け外し・ポーズのグループ)・`showPose`・`#afterLoad`・`#setUp` (WebPuppet を作った・作り直した)・
  `setParameter`・`resetAllParameters`。受け手のモードでないときは今までどおり毎フレーム描く。
- **報告:** `FollowRenderStats` に `drawn` を足し、`onRender` は `requestAnimationFrame` のたびに (描かなかった回も) 呼ぶ (描かなければ `renderMs` は 0。
  フレームの適用の時間は次に描いた回に入る)。スタジオの `RenderStatsWindow.record` は描いた回数 (`fps`) と描く機会の回数 (`refreshFps`) を数え、
  何も描かない 1 秒も報告する。
- **注意の判断の食い違いを直した:** 描いた回数は届いたフレームの数に従うので、フレームが届く時機と画面の更新の時機が揺れて 2 つのフレームが 1 回の更新に
  入ると (60 Hz の画面で刻み 60 のとき、時計のずれで周期的に起きる)、描いた回数が刻みの速さの 95% を下回り、描画は遅れていないのに「描画が届いていない」が
  出る。そこで `RenderStats` に `refreshFps` (描く機会の回数。仮想カメラは `null`) を足し (TS の `host.ts`・Rust の `lib.rs`。ブラウザソースの WebSocket の
  `renderStats` も同じ形)、`load.ts` の `renderSlow` は `refreshFps ?? fps` で判断する。前は描いた回数 = `requestAnimationFrame` の回数だったので、
  注意が出る場合は前と同じ (60 Hz の画面で刻み 120 は今も出る。T0076 の気づきのとおり)。注意の文を「描ける回数が … に届いていません」に変えた。
  画面に出す「描画の回数」は実際に描いた回数。docs/studio.md#負荷の表示 に書いた。
- **描く回数を数えた:** `just player-browser` に `player-follower-draws-on-change` を足した (本物の wasm・ヘッドレスの Chrome の WebGL2)。刻み 30 の
  フレームを 60 個 (約 2 秒) 届けると、**変更前は 122 回の `requestAnimationFrame` で 122 回描き、変更後は 121 回で 60 回** (1 秒あたり約 60 回 → 約 30 回)。
  フレームが止まった後の 30 フレームは描かず、ページのスクリーンショットで最後の形が残り (変化 0 画素)、ズームすると 1 回描く。
  変更前の数は、`player.svelte.ts` だけを着手時の版に戻して同じシナリオを流して測った (戻した後で元に戻した)。
- **画の確かめ (docs/studio.md#画の確かめ-adr-0028-の-10):** `source_check.rs` の待ち受けと T0808 の確認のスクリプトの写し (スクラッチ領域の `T0857/check.mjs`。
  止めた後 3 秒と 8 秒の画を比べる手順を足した) で、ブラウザソースと出力ウィンドウのページを止めた後の画を比べた。ページどうしは完全に一致、
  仮想カメラとは回転 8 度で平均の差 2.243・差 16 超 3.95% (T0808 の 2.2・3.9% と同じ)。止めてから 5 秒後の画も変化 0 画素 (不透明 123162 画素)。
  ヘッドレスの Chrome の WebGL2 (SwiftShader) だけで、WebGPU の canvas では確かめていない (下)。
- docs/studio.md (出力ウィンドウ・負荷の表示・隠れても止まらない)、packages/player/README.md、apps/studio/README.md、docs/testing.md
  (ブラウザのテストのシナリオ)、docs/manual-check.md (S32。T0855 の S30・T0856 の S31 の後) を直した。
- 流した検査 (T0499・T0856 の入った main に rebase した後に流し直した): `just ci` (通った)、`just player-browser` (23 シナリオすべて通った。描く回数は 122 回の `requestAnimationFrame` で 60 回)、`just e2e --only studio` (13 件通った)、
  `just ui-audit --only studio` (183 場面、指摘 0)。
- **範囲外の小さな変更:** `tests/ui-audit/lib/studio-host.mjs` (偽のホストの負荷の値に `refreshFps` を足した。`crates:` に足した)。

### やらなかったこと・確かめていないこと

- 実機の GPU の使用率 (Windows の WebView2・macOS の WKWebView・OBS の CEF)。手順は docs/manual-check.md の S32。
- WebGPU の canvas で、描かなかったフレームに前の画が残ること (ヘッドレスの Chrome は WebGPU を使わない設定で確かめた。WebGPU の仕様では
  `getCurrentTexture` を呼ばなかったフレームは合成済みの画をそのまま出す)。実機の Windows の WebView2 は WebGPU で描くので、S32 で見る。
- 「フレームレートと負荷」の欄に描く機会の回数 (`refreshFps`) は出していない (画面の並びを変えないため。判断にだけ使う)。

### 起票の候補

- 「フレームレートと負荷」に描く機会の回数 (画面のリフレッシュレート) を出す (描いた回数との違いが分かるように)。
- `@ugoe/runtime` の `<ugoe-avatar>` も、パラメータが変わらず物理が止まっているときは描かない (ページに埋め込む用途の GPU の節約)。
