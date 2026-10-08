---
id: T0982
title: "@ugoe/player: ビューアのループとエディタのプレビューでも、形が変わらないフレームは描かない"
status: done
milestone: M8
crates: [packages/player, apps/editor, docs]
depends_on: []
assignee: claude-worker-T0982
---

# T0982: @ugoe/player: 形が変わらないフレームは描かない

## 目的

T1402 で `WebPuppet.shape_changed()` (前の `render` の後に形が変わったか) を足し、`<ugoe-avatar>` は形が変わらない間は描かなくなった。
`@ugoe/player` の描画のループ (ビューアと、エディタのプレビュー) は今も `requestAnimationFrame` のたびに描く。止まったモデルや収まった物理で
描くのを省き、GPU と電力の使い方を減らす (スタジオの受け手は T0857 の別の仕組みで省いているので対象外)。

## 受け入れ条件

- [x] `Player` のループ (player.svelte.ts の `#tick`) で、`shape_changed()` が `false` で、見せ方 (描画先の大きさ・DPR・ビュー・背景・光源・テクスチャ・
      着せ替え・スクリーンショット) を変えていないフレームは `render` を呼ばない。見せ方の変化を漏れなく拾う方法 (例: 変えたら印を立てる) を決め、
      player.svelte.ts のコメントに書いた
- [x] テスト (偽の wasm。playerFollower.test.ts などの形) で、止まったモデルは 2 フレーム目から描かない・パラメータを動かす・ビューを動かす・大きさを変える・
      GPU を失って作り直した後は描く、を確かめた
- [x] スクリーンショットや読み戻し (`render` の後に読むもの) が、描かなかったフレームでも正しい画を返す (テストで確かめた)
- [x] 受け手のモード (スタジオ) の振る舞いは変わらない (既存のテストが通る)
- [x] エディタ・ビューアの e2e (`just e2e --only editor`・`just e2e --only viewer`) が通る
- [x] `just ci` が通る

## 参考

- T1402 のメモ (`shape_changed` の意味と起票の候補)、T0857 (受け手は変わったときだけ描く)、bindings/web/README.md#描画のループ
- packages/player/src/player.svelte.ts、crates/ugoe-wasm の `WebPuppet::shape_changed`、docs/spec/evaluation.md#見た目の変化

## メモ

- T0078 で起票した (元の候補: T1402 (`@ugoe/player` (ビューアのループ・エディタのプレビュー) でも `shape_changed` を使って描くのを省く))。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- WebGPU・WebGL2 の canvas は、描かなければ前の画を出し続ける (T0857 のメモ)。`<ugoe-avatar>` (T1402) の扱いに合わせる。
- 実機のブラウザ・Tauri で電力・GPU の使用率が下がることは測れない見込み (描く・描かないの振る舞いだけを確かめる)。

### やったこと (2026-10-08)

- **Player** (`packages/player/src/player.svelte.ts`): ふつうのモードの `#tick` は `update(dt)` の後、見せ方の印 `#dirty` か
  `WebPuppet.shape_changed()` が立っているときだけ `render()` を呼ぶ (`draw = #dirty || (受け手でない && shape_changed())`)。
  見せ方の変化を拾う方法は、T0857 で受け手のモードのために入れた印 `#dirty` をそのまま使う (見せ方を変えるところ ― `#syncView` (ビュー・
  大きさ・devicePixelRatio・描画バッファの倍率)・`#applyBackground`・`#applyLighting`・`#syncDressUp` (着せ替え・ポーズのグループ・
  テクスチャ)・`showPose`・`setParameter`・`resetAllParameters`・`#afterLoad`・`#setUp` (作った・作り直した) ― が自分で立てる)。
  `#dirty` の doc コメントに、印を立てる所と「見せ方を変える口を足すときは印を立てる」を書いた。受け手のモードは今までどおり印だけを見る
  (`shape_changed` は見ない)。GPU の喪失は描かないフレームでも毎フレーム `checkLost` で確かめる (前から描く前に呼んでいる)。
  `src/wasm.ts` の `WebPuppet` に `shape_changed()` を足した。
- **エディタのプレビュー** (`apps/editor/src/lib/viewport/controller.svelte.ts` の `#animate`。物理のプレビューとプレビューモード):
  エディタのプレビューは `Player` ではなく `ViewportController` のループで描いていたので、ここも `shape_changed()` が真のときだけ描くように
  した (見せ方の変化はプレビューの外と同じく `requestRender` が別の `requestAnimationFrame` で描く)。描かないフレームも `checkLost` を呼ぶ
  (WebPuppet の喪失はプレビューの着せ替えが次のフレームの確かめに頼っている。docs/editor.md の T1644)。`animationFrame` (オーバーレイの
  読み直し) は描かなかったフレームも増やす。`crates:` に `apps/editor` を足した (範囲外の小さな変更)。
- **スクリーンショット:** ビューア (`Player.capture`) もエディタ (`ViewportController.capture`) も canvas を写さず `read_pixels` で今の状態を
  別のテクスチャに描くので、描かなかったフレームの後でも今の画になる。アプリとテストで canvas を `drawImage`・`toDataURL` で写す所を探し、
  Player・エディタの canvas を後から写す所は無いことを確かめた (player-browser の `drawnFraction` は `render` の直後に同じタスクで写す)。
- **テスト:** `src/playerDraw.test.ts` (偽の wasm。止まったモデルは 2 フレーム目から描かない・形が動く間は描き収まると描かない・
  パラメータ (`setParameter`・`resetAllParameters`)・ビュー (ズーム・パン・fit)・大きさ・倍率・背景・光源・着せ替えの付け外し・ポーズ・
  devicePixelRatio の変化で描く・GPU を失って作り直した後は (形が変わっていなくても) 描く・モデルが無くても 1 回描く・描かなかった
  フレームの後の `capture` が描き直さずに今の値の画を返す)。前の `#tick` では 8 つとも落ちることを確かめた。偽の WebPuppet
  (`src/testing/fakePuppet.ts`) に `shape_changed`・`update`・`read_pixels` などを足した。`playerFollower.test.ts` の「受け手でなければ毎フレーム
  描く」を、形が動く間は毎フレーム描くことと、受け手は `shape_changed` を見ないことに書き換えた。エディタの `controller.test.ts` に
  物理のプレビューが形の変わる間だけ描くテストを足した (前の `#animate` で落ちることを確かめた)。
- **ブラウザのテスト:** `just player-browser` に `player-viewer-draws-on-change` を足した (本物の wasm・ヘッドレスの Chrome の WebGL2)。待機
  モーション・目パチ・呼吸を止めた `dress-up.ugoe` は、読んだ後 1 回描くと描かなくなり (40 フレームで描いたのは 1 回)、描かない間も
  ページのスクリーンショットで画が残り (変化 0 画素)、`capture` は空でなく、パラメータを動かすと描いて画が変わり、ズームで 1 回描く。
  前の `#tick` では落ちることを確かめた。Player は呼吸を止める口を持たないので、ページが `load_bundle` を包んで止める。
- 文書: packages/player/README.md (描くのを省く・ブラウザのテスト)、docs/editor.md (物理のプレビュー・プレビューモードの「描くのを省く」)、
  crates/ugoe-wasm/README.md#描くのを省く-t1402、docs/spec/rendering.md#描画の省略、docs/testing.md (シナリオ)。
- 流した検査: `just ci` (通った)、`just player-browser` (25 シナリオすべて通った)、`just editor-browser` (4 シナリオ通った)、
  `just e2e --only editor` (16 件通った)、`just e2e --only viewer` (6 件通った)、エディタ・ビューア・スタジオ・player の vitest と svelte-check。

### やらなかったこと・確かめていないこと

- 実機のブラウザ・Tauri (WebView2・WKWebView・WebKitGTK) で GPU の使用率や電力が下がることは測っていない (描く・描かないの振る舞いだけ)。
  WebGPU の canvas で描かなかったフレームに前の画が残ることも、ヘッドレスの Chrome の WebGL2 だけで確かめた。
- wasm の `render` がサーフェスの作り直し・時間切れで描かずに返った (`Outdated`・`Lost`・`Timeout`) とき、`shape_changed` は下りないが
  Player の `#dirty` は下ろすので、見せ方だけを変えたフレームでそれが起きると次に形か見せ方が変わるまで描き直さない
  (`<ugoe-avatar>` も同じ)。ブラウザの canvas ではまず起きないので、そのままにした。
- スタジオの e2e は流していない (受け手のモードは変えていない。player と studio の vitest は通った)。

### 起票の候補

- `WebPuppet::render` が描けなかったことを返す (または描けなかったときは `shape_changed` を「描く必要がある」として立てたままにすると
  書く) ことで、見せ方の印だけで描こうとしたフレームでサーフェスを作り直したときも次のフレームで描き直す (Player・`<ugoe-avatar>`・エディタ)。
