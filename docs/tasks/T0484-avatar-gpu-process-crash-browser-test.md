---
id: T0484
title: "適合テスト: <ugoe-avatar> で GPU プロセスが落ちても作り直して描く"
status: todo
milestone: M9
crates: [tests/conformance, bindings/web, docs]
depends_on: []
assignee:
---

# T0484: 適合テスト: <ugoe-avatar> で GPU プロセスが落ちても作り直して描く

## 目的

T0944 で、GPU プロセスを落とすブラウザのテスト (CDP の `Browser.crashGpuProcess`) を `@ugoe/player` (`just player-browser`) に足し、作成の途中で落ちたときの
glow と wgpu の panic を直した。`<ugoe-avatar>` (bindings/web) の側には同じシナリオが無い (T0944 の候補)。wasm は同じなので panic は直っているが、
`<ugoe-avatar>` の復元の待ちと作り直しが GPU プロセスの喪失でも働くことを確かめる。

## 受け入れ条件

- [ ] 適合テストの `avatar` (tests/conformance/run.mjs) に、描いている間に GPU プロセスを落とすと `<ugoe-avatar>` が作り直して描くシナリオと、
      作成の途中に落としても panic せずコードを持つ例外 (と作り直し) で終わるシナリオがある (起動の引数と落とし方は player-browser と揃える)
- [ ] 壊れを見つけることを一度確かめた (例: 作り直しを止めるとシナリオが落ちる) とメモに書いた
- [ ] docs/testing.md と tests/conformance/README.md を直した
- [ ] `just conformance --only avatar` と `just ci` が通る

## 参考

- T0944 のメモ (起票の候補)、docs/testing.md (GPU プロセスの落とし方)
- tests/conformance/run.mjs、bindings/web/src/avatar.ts

## メモ

- T0040 で起票した (元の候補: T0944 (`<ugoe-avatar>` のブラウザのテスト))。
