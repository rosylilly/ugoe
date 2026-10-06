---
id: T1655
title: "エディタ: 空のプロジェクトでビューポートの「Screenshot」を押せないようにする"
status: done
milestone: M10
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1655
---

# T1655: エディタ: 空のプロジェクトでビューポートの「Screenshot」を押せないようにする

## 目的

空のプロジェクト (描くものが無い) でも、ビューポートの「Screenshot」が押せる (T0919)。押すと空の画像を保存するか、失敗の知らせになる。
描くものが無いときは押せないようにする。

## 受け入れ条件

- [x] 描くものが無いとき (空のプロジェクト・モデルを読み込む前・読み込みの失敗) は「Screenshot」が押せない (`disabled`。Tooltip で理由が分かる)。
      モデルがあれば今のまま押せる
- [x] ショートカットや別の道からも、描くものが無いときは保存の処理に入らない
- [x] 部品のテスト (Viewport の空の状態でボタンが押せない・モデルを読むと押せる)
- [x] docs/editor.md (スクリーンショット) を直した。マニュアルに書く必要があればマニュアルの更新の候補をメモに書いた
- [x] `just ui-audit --only editor` の `--compare` で増えた指摘が無く、`just e2e --only editor` と `just ci` が通る

## 参考

- T0919 のメモ (起票の候補)・docs/roadmap.md の M10 の「後続の候補 (T0919)」
- apps/editor/src/lib/viewport/Viewport.svelte (`screenshot`)・saveScreenshot.ts

## メモ

- T0046 で起票した (元の候補: T0919 (空のプロジェクトでスクリーンショットのボタンが押せる))。
- やったこと (T1655):
  - `ViewportController.hasModel` を足した (`#load` の後に `WebPuppet.has_model()`、読み込みの失敗で `false`)。
  - Viewport.svelte の「Screenshot」は `status === "ready" && hasModel && !empty` のときだけ押せる (`canCapture`)。
    押せない間は Tooltip を理由の文 (`viewport.screenshot.unavailable`。en・ja) にし、ボタンの `aria-describedby` で見えない文
    (`ugoe-visually-hidden`) を指す。保存の処理 (`screenshot()`) も `canCapture` で入らない (ショートカットなど別の道は今は無いが、
    足しても空の画像を保存しない)。
  - テスト: Viewport.svelte.test.ts (モデルが無い・読み込みの失敗・空のプロジェクトで押せず理由が付く、モデルを読むと押せる)、
    controller.test.ts (`hasModel` が読み込みと失敗で変わる)。
  - docs/editor.md#スクリーンショット に「押せないとき」を足し、マニュアル (docs/manual/screen.md) にも 1 文足した。
  - `just ui-audit --only editor --compare` (前は main の cfa313a8): 増えたのは `ja-light-zoom200` の場面の失敗 (`locator.click` の時間切れ。
    外での変更のダイアログが残って押せなかった、混んだ機械での揺れ) の 1 件だけ。その組だけ `--variants ja-light-zoom200 --no-build` で
    流し直すと 40 場面で 0 件だった。`just e2e --only editor` は 15 件すべて通った。`just ci` は通った (main に rebase した後)。
- やらなかったこと: 押せない理由を状態ごとに分ける (GPU を待つ・読み込みの失敗は HUD に別の文が出ているので、1 つの文にした)。
- 起票の候補: なし。
