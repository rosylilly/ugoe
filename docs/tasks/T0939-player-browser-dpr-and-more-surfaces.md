---
id: T0939
title: "PuppetSurface のブラウザのテスト: 倍率の変化のシナリオと、エディタの ViewportController・Player への広げ方"
status: in-progress
milestone: M9
crates: [packages/player, apps/editor, docs]
depends_on: []
assignee: claude-worker-T0939
---

# T0939: PuppetSurface のブラウザのテスト: 倍率の変化のシナリオと、エディタの ViewportController・Player への広げ方

## 目的

T0936 で、本物の wasm とヘッドレスの Chrome で `PuppetSurface` を確かめる `just player-browser` (`packages/player/browser-test/`) を作った。
次が残っている (T0936 の候補)。

1. `devicePixelRatio` だけが変わったときの追従のシナリオ (CDP の `Emulation.setDeviceMetricsOverride` で倍率を変え、
   `matchMedia` の `change` が本物のブラウザで届き、描画バッファが新しい倍率になるか)。
2. エディタの `ViewportController` と Player (`player.svelte.ts`) は、偽の wasm の Vitest でしか確かめていない。
   同じ仕組み (本物の wasm とヘッドレスの Chrome) で確かめるか。Svelte のコンポーネントや Tauri の IPC が要るなら、
   どこまでを対象にするか (喪失と作り直し・倍率など、surface を通る経路) を決めて書く。

同じ `PuppetSurface` を触る T0938 (判断が要る) と前後するときは、後に入る側がシナリオを合わせる。

## 受け入れ条件

- [ ] `just player-browser` に倍率の変化のシナリオがあり、通る。追従の処理を壊すと落ちることを確かめた
- [ ] エディタの `ViewportController` と Player のうち、本物の wasm で確かめるものにシナリオを足した。足さないものは理由を docs/testing.md に書いた
- [ ] docs/testing.md (「PuppetSurface のブラウザのテスト」) と packages/player/README.md を直した
- [ ] `just ci` が通る (`just player-browser` は `just ci` の外。手元で通したことをメモに)

## 参考

- T0936 のメモ (決めたこと・確かめていないこと・起票の候補)
- docs/testing.md#puppetsurface-のブラウザのテスト

## メモ

- T0036 で起票した (元の候補: T0936 (2 つ))。
