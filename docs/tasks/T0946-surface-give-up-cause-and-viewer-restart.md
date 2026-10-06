---
id: T0946
title: "@ugoe/player: 作り直しを諦めた理由を GpuLostRepeatedlyError に持たせて文に出し、ビューアの作り直しのボタンを確かめる"
status: done
milestone: M9
crates: [packages/player, apps/editor, apps/viewer, docs]
depends_on: [T0945]
assignee: claude-worker-T0946
---

# T0946: @ugoe/player: 作り直しを諦めた理由を GpuLostRepeatedlyError に持たせて文に出し、ビューアの作り直しのボタンを確かめる

## 目的

T0941 で、`@ugoe/player` とエディタは `CreateError` を訳して出すようになった。残ったもの (T0941 の候補):

- `PuppetSurface.restart`・`#recreate` が諦めたとき、`GpuLostRepeatedlyError` は最後の `CreateError` を持たない。最初の作成の失敗から作り直して
  また作れなかったときも、作れなかった理由 (コンテキストを作れない・上限など) の文が出ず「何度も失った」の文になる (Player・エディタ)。
- ビューアの `Viewport.svelte` のコメントとテストに、`CreateError` のときも「作り直す」のボタンが出ることが書かれていない (振る舞いは `Player.canRestart` を見るので正しい)。
- T0479 の候補「`<ugoe-avatar>` の `#restoreTimer` の上書き」は T0480 で直った。`PuppetSurface` に同じ形 (時間切れのタイマーを消さずに置き直す経路) が無いかは確かめていない。

## 受け入れ条件

- [x] 作り直しを諦めたとき、最後の `CreateError` を `GpuLostRepeatedlyError` の `cause` に持たせ、Player とエディタのビューポートが `cause` の理由の文
      (T0941 の訳) を添えて出す。`surface.test.ts`・`player.test.ts`・エディタの `controller.test.ts` で確かめる
- [x] ビューアの `Viewport.svelte` のコメントとテストに、`CreateError` で作れなかったときも「作り直す」のボタンが出ることがある
- [x] `PuppetSurface` の復元の時間切れのタイマーを置く経路をすべて調べ、前のタイマーを消さずに置き直す経路があれば先に失敗するテストを書いて直した (無ければメモに書いた)
- [x] docs/ui.md#webpuppet-の寿命 と packages/player/README.md を直した
- [x] `just ci` と `just player-browser` が通る

## 参考

- T0941 のメモ (起票の候補)、T0480 (`<ugoe-avatar>` の `#setRestoreTimer`)
- packages/player/src/surface.ts、apps/editor の `ViewportController`、apps/viewer/src/lib/Viewport.svelte

## メモ

- T0040 で起票した (元の候補: T0941 (`cause`)・T0941 (ビューアの `Viewport.svelte`)・T0479 (PuppetSurface も同じか確かめる))。
  同じ Player の作り直しを触る T0945 の後に着手する。
- T0946 (やったこと):
  - `GpuLostRepeatedlyError` は `cause` を受ける (`new GpuLostRepeatedlyError(cause?)`)。`PuppetSurface.#recreate` はその作り直しの中で作れなかった
    最後の失敗を覚え、諦めるときに `cause` に渡す。どの作り直しも作れていた (作っては失った) なら `cause` は無い。そのため `#create` は
    `failed` を呼ばずに失敗 (`{ error }`) を返し、最初の作成の失敗の `failed` は `attach` が呼ぶようにした。
  - Player は `surfaceFailureText` で、`cause` があれば `errors.recreateFailed` (「<理由の文> (作り直しを何度か試しましたが、作れませんでした)」)
    に理由の文 (`createError.*`、ほかの例外は `errors.renderStart`) を埋める。エディタは `viewportErrors.ts` の `surfaceFailureText` で
    `viewport.recreateFailed` に `createErrorText` の文を埋める。`cause` が無ければ今までどおり「GPU を続けて失った」。ボタンは今までどおり出す。
  - テスト: `surface.test.ts` (諦めたときの `cause`・`cause` が無いこと)・`player.test.ts`・エディタの `controller.test.ts`・`viewportErrors.test.ts`
    (最初の作成の失敗 → 「作り直す」→ 作れないまま上限 → 理由の文)。ブラウザのテストに `restart-on-lost-context-gives-up-with-cause`
    (本物の wasm の `CreateError` が `cause` になる) と、`repeated-loss-gives-up` に `cause` が無いことの確かめを足した。
  - ビューアの `Viewport.svelte` のコメントとテストに、`CreateError` で作れなかったときもボタンが出ること (どちらも `Player.canRestart`) を足した。
  - `#restoreTimer` を置く経路を調べた: 置くのは `#onContextLost` だけで、`#awaitingRestore` が偽のときだけ (待っている間の 2 つ目の知らせ・
    作り直している間・古い canvas の知らせでは置かない)。時間切れ (`#onRestoreTimeout` は先に `null` にする)・復元・`#release` で消すので、
    前のタイマーを消さずに置き直す経路は無かった (直すところは無い)。確かめるテスト (`sets one restore timer …`。`vi.getTimerCount`) を足した。
    docs/ui.md#webpuppet-の寿命 にも書いた。
  - docs/ui.md (諦めたときの `cause`・失敗の文の表に 1 行)・packages/player/README.md・docs/editor.md を直した。
  - `just ci` と `just player-browser` (20 のシナリオ) が通った。
- やらなかったこと: 実際の GPU のリセットでの確認はしていない (偽の wasm と、ヘッドレスの Chrome で失わせたコンテキストだけ)。
  スタジオの出力ウィンドウは `error` を出さないので触っていない。
- 起票の候補: なし。
