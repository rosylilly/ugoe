---
id: T0955
title: "Player: 読み込みの途中に GPU を失ったら、エラーの文を出さず、開こうとしたファイルを作り直した後に開く"
status: done
milestone: M9
crates: [packages/player, apps/viewer, apps/editor, docs]
depends_on: [T0486]
assignee: claude-worker-T0955
---

# T0955: Player: 読み込みの途中に GPU を失ったら、エラーの文を出さず、開こうとしたファイルを作り直した後に開く

## 目的

T0486 で利用者が `<ugoe-avatar>` の扱いを案 A に決めた (2026-10-06): 読み込みで `device_lost` を受けたら `error` を送らず、モデルを取っておいて
作り直した後に読む。`error` は作り直しを諦めたときだけ。

`@ugoe/player` の Player は今、読み込みで `device_lost` を受けるとエラーの文 (「GPU が止まりました」) を出し、開こうとしたファイルを捨て、
作り直した後は前のモデルを読み直す (T0486 の調べ)。前のモデルが無ければエラーの文が残る。`<ugoe-avatar>` と同じ扱いに揃える。

## 受け入れ条件

- [x] Player は読み込み (開く・着せ替えを含むなら同じく) で `device_lost` を受けてもエラーの文を出さず、開こうとしたファイルを作り直した後に開く
- [x] 作り直しを諦めたときだけエラーの文を出す (T0946 の理由の文)
- [x] `player.test.ts` に偽物の順序のテスト、browser-test に読み込み中の喪失のシナリオがある
- [x] エディタのビューポート (`ViewportController`) も同じ扱いか確かめ、違えば揃えた
- [x] docs/ui.md#webpuppet-の寿命・packages/player/README.md を直した。`just player-browser`・`just ci` が通る

## 参考

- T0486 のメモ (調べたことと判断)、T0938・T0941・T0946

## メモ

- オーケストレーターが利用者の判断を受けて直接起票した (T0486 の候補)。
- やったこと (T0955):
  - Player (`packages/player/src/player.svelte.ts`): `load` の読み込みが投げたとき `puppet.is_lost()` が真なら (`device_lost`)、文を出さず、
    WebPuppet が無い間に開いたものと同じ道 (`#openLater` → `#setUp`) で、作り直した後にそのファイル (と一緒に開くモジュール) を開く
    (fit で見せ、値は既定から)。`#setUp` の読み直しの途中にまた失ったときは利用者が開いたものではないので、次の作り直しでもビューと値を保つ
    (`#keepView` の間は `#openedWhileDown` を立てない)。`loadModule` も同じく作り直した後に読む。付け外し (`#dressUp`) は WebPuppet が
    無い間と同じく何もせず `false` で、`dressUpError` を出さない。`load` の Promise は作り直しを待たずに返る (WebPuppet が無い間と同じ。
    `<ugoe-avatar>` の `load()` のように作り直しまで待たせることはしていない)。エラーの文は作り直しを諦めたとき (`failed`。T0946 の文) だけ。
  - player.test.ts に 7 つ (WebGL2・初めのモデル・WebGPU・モジュール・付け外し・読み直しの途中にまた失ってもビューと値とモジュールを保つ・
    諦めたときだけ T0946 の文で、`restart` で開こうとしたファイルを開く)。偽物の WebPuppet に `state.lost` (読み込みと着せ替えが
    `device_lost` を投げる) を足した。直す前には 6 つが落ちることを確かめた (付け外しのテストは後から足した)。
  - browser-test にシナリオ `player-loss-during-load` (本物の wasm で、コンテキストを失っている間に初めのモデル、続いて別のモデルとモジュールを
    開く)。直す前の player.svelte.ts では落ちることを確かめた。
  - エディタのビューポート (`ViewportController`): 違っていた (読み込みの `device_lost` で `status` を `error`・`viewport.deviceLost` にし、
    WebGPU では次に操作するまで作り直さなかった)。`#load` が投げて `is_lost()` が真なら、文を出さずに `requestRender` で次のフレームの
    `checkLost()` へ回す (WebGL2 は「GPU の復元を待っています」のまま)。作り直した WebPuppet に `#setUp` が今のモデルを読む。
    controller.test.ts に 2 つ (WebGL2・WebGPU。直す前に落ちることを確かめた)。
  - 文書: docs/ui.md#webpuppet-の寿命 (「読み込みの途中の喪失」と失敗の文の表)・packages/player/README.md・docs/editor.md (読み込みの失敗)・
    docs/testing.md (ブラウザのテストのシナリオ)。
  - `just player-browser` (21/21)・`just e2e --only viewer`・`just e2e --only editor`・`just ci` が通った。
- やらなかったこと・確かめていないこと:
  - 本物の WebGPU のデバイスの喪失 (ページから起こせない。偽物のテストだけ)。browser-test は WebGL2 のコンテキストの喪失だけ。
  - スタジオ (`apps/studio`) の e2e は流していない (Player を使うがアプリのコードは変えていない。スタジオの出力ウィンドウは `error` を出さない)。
  - 付け外しの操作は、失っている間は効かない (作り直した後に繰り返さない)。WebPuppet が無い間と同じ扱いにした。
- 起票の候補:
  - エディタのプレビューの着せ替え (`apps/editor/src/lib/preview/dressUp.svelte.ts`) は `load_module` などの `device_lost` を
    `preview.dressUp.problem.deviceLost` の文で出す。作り直しの後に読み直すかどうかを同じ扱いに揃えるか確かめる。
