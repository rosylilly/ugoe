---
id: T0939
title: "PuppetSurface のブラウザのテスト: 倍率の変化のシナリオと、エディタの ViewportController・Player への広げ方"
status: done
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

- [x] `just player-browser` に倍率の変化のシナリオがあり、通る。追従の処理を壊すと落ちることを確かめた
- [x] エディタの `ViewportController` と Player のうち、本物の wasm で確かめるものにシナリオを足した。足さないものは理由を docs/testing.md に書いた
- [x] docs/testing.md (「PuppetSurface のブラウザのテスト」) と packages/player/README.md を直した
- [x] `just ci` が通る (`just player-browser` は `just ci` の外。手元で通したことをメモに)

## 参考

- T0936 のメモ (決めたこと・確かめていないこと・起票の候補)
- docs/testing.md#puppetsurface-のブラウザのテスト

## メモ

- T0036 で起票した (元の候補: T0936 (2 つ))。

### 決めたこと (広げる範囲)

- **Player は足した、ViewportController は足さない。** Player は作り直しでモデル・モジュール・パラメータの値を読み直し・付け直し・戻すという
  自分の処理を持ち、倍率の換算 (`view`・`zoomBy`) も本物の wasm の `view_transform`・`zoom_at` に頼るので、本物で確かめる値打ちがある。
  同じパッケージの中なので、`@ugoe/player` を Vite でまとめるだけで足せた (手間は小さい)。
  ViewportController の `PuppetSurface` を通る経路は Player と同じ形で薄く (渡し直す・状態を出す・描き直しを頼む・換算)、本物で確かめたい
  部分は `PuppetSurface` と Player のシナリオで通る。ブラウザで動かすには `@ugoe/player` のテストがアプリのコードを読む (向きが逆) か、
  エディタの側に仕組みを作り直してエディタの wasm の読み込み (`import.meta.glob`) と文言を差し替える必要があり、手間に比べて得るものが小さい。
  理由は docs/testing.md#puppetsurface-のブラウザのテスト に書いた。
- **倍率の変化の起こし方:** CDP の `Emulation.setDeviceMetricsOverride` で倍率だけを変える。ヘッドレスの Chrome 154 は倍率の上書きだけでは
  メディアクエリを評価し直さず、`matches` は変わるが `change` が届かない (ページの大きさも変えると届く。小さなページで確かめた)。
  上書きのたびにエミュレートするメディアの型を `""` と `"screen"` で交互に切り替えて評価し直させる (ページから見える値は倍率のほか変わらない)。
- surface.ts を `transpileModule` で JS にするのをやめ、`src/index.ts` 全体を Vite でまとめて配る (surface と Player が同じまとめから来る)。
  そのため `vite` (`^8.3.1`。アプリと同じ) を `@ugoe/player` の devDependencies に足した (pnpm-lock.yaml も変わる。ライセンスは MIT)。

### やったこと

- シナリオ `pixel-ratio-change` (PuppetSurface: 1 → 2 → 1.5 → 1 で WebPuppet と canvas の描画バッファ・`pixelRatioChanged`・描けること、
  倍率 2 での `resize`、`detach` の後は知らせない)、`player-loss-restore` (dress-up の見本と cat-ears を付けて `MouthForm` を動かし、
  2 回失って復元: モデル・付けたモジュール・値・`capture` の読み戻し・生きている WebPuppet が 1 つ)、`player-pixel-ratio`
  (倍率 2・1.5 で canvas と `capture` の大きさ、`view` が動かない、CSS の点を中心にしたズームでその点が動かない)。
- 壊れを見つけることを確かめた (戻して 7 つとも通る): surface.ts で `attach` の `#watchPixelRatio` を消す・`change` の後に聞き直さない・
  `detach` の `removeEventListener` を消す・`#onPixelRatio` の `resize` を消す → `pixel-ratio-change` が落ちる。player.svelte.ts で
  作り直しの後の `set_parameter` (carry) を消す・`#reloadModules` を消す → `player-loss-restore`、`zoomBy` の換算の倍率を 1 にする →
  `player-pixel-ratio` が落ちる。
- 文書: docs/testing.md (表の行と節「PuppetSurface のブラウザのテスト」: シナリオ・倍率の起こし方・壊れの確認・ViewportController を
  対象にしない理由)、packages/player/README.md。
- 範囲の外の小さな変更: `justfile` (`player-browser` のコメント)・`.github/workflows/ci.yml` (`conformance` ジョブのコメント)・
  `pnpm-lock.yaml` (vite の devDependency)。

### 確かめていないこと

- 倍率の違う画面へウィンドウを移したときに、本物のブラウザ・Tauri の WebView が `matchMedia` の `change` を届けること
  (ヘッドレスではメディアの型の切り替えで評価し直させている)。
- Player の `pixelRatioChanged` → `#syncView` を消しても `player-pixel-ratio` は落ちない。今の wasm は描画バッファの大きさが変わっても
  CSS ピクセルで見たビューを保つので、読み直さなくても `view` が同じになるため (害は無い)。
- Player の諦め (`unavailable` と `errors.gpuLostRepeatedly`) は本物では流していない (PuppetSurface の諦めは `repeated-loss-gives-up` で、
  Player の文と状態は偽の wasm の Vitest で確かめている)。
- CI の上での実行 (push していない)。手元 (WSL2・Google Chrome 154・SwiftShader) で `just player-browser` が 7 つとも通った (約 7 秒)。

### 検査

- `git rebase main` の後に `just player-browser` (wasm のビルドから。7 つとも ok) と `just ci` (前面、終了コード 0)。ランタイム層は触っていない。

### 起票の候補

- エディタが作り直しのまわりに自分の処理を持ったとき (T0938 の作り直すボタンなど) に、エディタの側にブラウザのテストを足す
  (run.mjs の仕組みを共有するか、apps/editor に置くか)。
- 作り直しの後に Player がズーム・パンを `fit` に戻す (`#setUp` → `load` → `#afterLoad` → `fit()`)。利用者のビューを保つかを決める。

