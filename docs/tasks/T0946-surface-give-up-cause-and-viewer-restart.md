---
id: T0946
title: "@ugoe/player: 作り直しを諦めた理由を GpuLostRepeatedlyError に持たせて文に出し、ビューアの作り直しのボタンを確かめる"
status: in-progress
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

- [ ] 作り直しを諦めたとき、最後の `CreateError` を `GpuLostRepeatedlyError` の `cause` に持たせ、Player とエディタのビューポートが `cause` の理由の文
      (T0941 の訳) を添えて出す。`surface.test.ts`・`player.test.ts`・エディタの `controller.test.ts` で確かめる
- [ ] ビューアの `Viewport.svelte` のコメントとテストに、`CreateError` で作れなかったときも「作り直す」のボタンが出ることがある
- [ ] `PuppetSurface` の復元の時間切れのタイマーを置く経路をすべて調べ、前のタイマーを消さずに置き直す経路があれば先に失敗するテストを書いて直した (無ければメモに書いた)
- [ ] docs/ui.md#webpuppet-の寿命 と packages/player/README.md を直した
- [ ] `just ci` と `just player-browser` が通る

## 参考

- T0941 のメモ (起票の候補)、T0480 (`<ugoe-avatar>` の `#setRestoreTimer`)
- packages/player/src/surface.ts、apps/editor の `ViewportController`、apps/viewer/src/lib/Viewport.svelte

## メモ

- T0040 で起票した (元の候補: T0941 (`cause`)・T0941 (ビューアの `Viewport.svelte`)・T0479 (PuppetSurface も同じか確かめる))。
  同じ Player の作り直しを触る T0945 の後に着手する。
