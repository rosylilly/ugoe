---
id: T0944
title: "PuppetSurface のブラウザのテスト: 作成の後に GPU プロセスが落ちても wasm が panic しないことを確かめる"
status: todo
milestone: M9
crates: [packages/player, ugoe-wasm, docs]
depends_on: []
assignee:
---

# T0944: PuppetSurface のブラウザのテスト: 作成の後に GPU プロセスが落ちても wasm が panic しないことを確かめる

## 目的

T0475 で、失ったコンテキストを `getContext` が返したときに `WebPuppet.create` が glow の中で panic する不具合を直した。確かめたのは作成の入口だけで、
作成の後 (アダプタを求めた後・`configure` の途中・描画中) に GPU プロセスごとコンテキストを失ったときに glow・wgpu の中で panic しないかは
確かめていない (作成は同期的に進むので JS の `loseContext` は割り込めない。T0475 の候補)。

## 受け入れ条件

- [ ] `packages/player/browser-test` に、作成の後に GPU プロセスを落とす (CDP の `Browser.crashGpuProcess` など。ヘッドレスの Chrome と SwiftShader で
      落とせるかを確かめる) シナリオを足し、`PuppetSurface` と `Player` が作り直すか諦めて、page.log に panic が無いことを確かめる。落とせないなら理由と代わりの確かめ方をメモに書いた
- [ ] panic が見つかったら ugoe-wasm で失敗として返すように直し、先に失敗するシナリオを入れた (見つからなければメモに書く)
- [ ] docs/testing.md#puppetsurface-のブラウザのテスト を直した
- [ ] `just player-browser`・`just ci` が通る (wasm を直したら `just wasm-check` も)

## 参考

- T0475 のメモ (確かめていないこと・起票の候補・上流への報告の案)
- packages/player/browser-test/run.mjs、crates/ugoe-wasm/src/create.rs

## メモ

- T0038 で起票した (元の候補: T0475 (作成の後の喪失))。本物の GPU での確かめは docs/manual-check.md の D09。
