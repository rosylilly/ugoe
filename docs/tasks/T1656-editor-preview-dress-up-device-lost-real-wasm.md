---
id: T1656
title: "エディタ: プレビューの着せ替えの「読み込みの途中に GPU を失う → 作り直して付け直す」を本物の wasm で通すシナリオを足す"
status: done
milestone: M10
crates: [apps/editor, tests, docs]
depends_on: []
assignee: claude-worker-T1656
---

# T1656: エディタ: プレビューの着せ替えの「読み込みの途中に GPU を失う → 作り直して付け直す」を本物の wasm で通すシナリオを足す

## 目的

T1644 で、プレビューの着せ替えのモジュールの読み込みの途中で GPU を失ったとき、文を出さずに取っておき、作り直した WebPuppet に付け直すようにした。
確かめたのは偽の WebPuppet の単体テストだけで、本物の wasm (`WebPuppet` の `is_lost`・`load_module` の `device_lost`) では通していない (T1644)。

## 受け入れ条件

- [x] エディタの browser-test (ヘッドレスの Chrome と本物の wasm) か e2e に、プレビューの着せ替えでモジュールを読み込む途中に GPU を失わせる
      (WebGL の `WEBGL_lose_context` など) シナリオがある。作り直した後にモジュールが付いて描かれ、行に失敗の文が出ないことを確かめる
- [x] 失っている間の付け外しが、作り直した後に付けたい組で付くことも確かめる
- [x] どちらの仕組み (browser-test か e2e) を選んだかと理由をメモに書いた。シナリオの動かし方を、そのテストの README か docs/testing.md に書いた
- [x] 足したシナリオと `just ci` が通る

## 参考

- T1644 のメモ (やらなかったこと・起票の候補)、T0955 (Player の読み込みの途中の喪失)
- apps/editor/src/lib/preview/dressUp.svelte.ts、docs/ui.md#webpuppet-の寿命、docs/testing.md

## メモ

- T0046 で起票した (元の候補: T1644 (本物の wasm での喪失のシナリオ))。
- **やったこと (T1656):**
  - エディタのブラウザのテスト `apps/editor/browser-test/` (`just editor-browser`) を足した。ビルドしたエディタの画面 (`vite build`) を
    本物の wasm とヘッドレスの Chrome (WebGL2 は SwiftShader) で開き、Tauri の代わりに `just ui-audit` の偽のホスト
    (`tests/ui-audit/lib/editor-host.mjs`・`serve.mjs`。変えずに読むだけ) を置き、Playwright で操作する。本物の WebPuppet は、ページが
    wasm-bindgen のグルーのチャンクをエディタと同じ URL で読んで `WebPuppet` を包んで見る (描いているもの・生きているもの・作成の遅れ)。
    喪失と復元は canvas ごとに覚えたコンテキストの `WEBGL_lose_context`。
  - シナリオ `dress-up-loss-during-load`: 失っている間 (ビューポートが「GPU の復元を待っています」) に「Load .ugoemod…」で猫耳を読み込み、
    パネルに失敗の文 (`role=alert`) が出ず、復元の後に作り直した WebPuppet に付き (`attached_modules`・行のチェック)、描かれる
    (`read_pixels` で、背景と違う画素の一番上の行が猫耳のぶん約 48 px 上がる)。
  - シナリオ `dress-up-change-while-lost`: 猫耳とツインテールが付いた状態で、失っている間に猫耳のチェックを外し、スロット `Hair` の選択で
    長い髪に替える。作り直した後は長い髪だけが付き、行もそのとおりで、猫耳は描かれない。
  - 待ち方: 作り直しの完了 (新しい WebPuppet が描き、ビューポートの待ちの文が消える) を待つ。作り直しの `WebPuppet.create` を 500 ms
    遅らせて CI の遅さを再現する (T0960 と同じ)。3000 ms でも通り、5 回続けて流して 5 回とも 2 of 2 だった。
  - テストが壊れを見つけること: T1644 の前の `dressUp.svelte.ts` (5f0ef464) では 2 つとも落ちる (失っている間に「The GPU stopped working.」)。
    `DressUp.loaded` の `#loadPending` を呼ばないと `dress-up-loss-during-load` が作り直しの後に付かずに落ちる。
  - 選んだ仕組みは browser-test (e2e ではない)。理由: Chrome の `WEBGL_lose_context` で喪失と復元を決まった時機に起こせる (player-browser と
    同じ。WebKitGTK で同じく起こせるかは確かめていない)、本物の WebPuppet をページから直接見られる (付いたモジュールと読み戻しの画素)、
    e2e はエディタの筋書き全体を 1 回の起動で順に流し 1 つの手順を選べず重い。Tauri と WebView の間 (IPC・ダイアログ) は見ないが、
    着せ替えの流れそのものは e2e の手順が見る。apps/editor/browser-test/README.md と docs/testing.md#エディタのブラウザのテスト に書いた。
  - 範囲の外の小さな変更: `justfile` に `editor-browser` のレシピ、`.github/workflows/ci.yml` の `conformance` ジョブに
    `just editor-browser` の手順 (落ちたときのページのログと画像を成果物に) を足し、ジョブの上限を 30 分から 40 分にした
    (エディタの `vite build` と見本の作成が増えるため)。apps/editor/README.md のコマンドに 1 行。
- **検査:** main f162a6ad に rebase した後、`just editor-browser` (ビルドから) が 2 of 2、`just ci` が通った。
- **やらなかったこと・確かめていないこと:**
  - CI (GitHub Actions) の上では流していない (手元の WSL2 の Chrome と SwiftShader だけ)。CI での所要時間は見ていない。
  - 実機の e2e (WebKitGTK) には足していない。本物の GPU のリセットと WebGPU のデバイスの喪失は確かめていない (docs/manual-check.md の D09)。
  - 作り直しの途中 (古い WebPuppet を捨ててから新しいものができるまで。`PuppetSurface` の `puppet` が `null`) に読み込んだモジュールは、
    `DressUp.#load` が何もせずに捨てる (`#pending` に入らない)。今回のシナリオは復元を待っている間 (古い WebPuppet が残っている) だけを通す。
- **起票の候補:**
  - プレビューの着せ替えで、WebPuppet の作り直しの途中 (`host.puppet()` が `null`) に読み込んだモジュールも `#pending` に取っておき、
    作り直した後に読み込む (今は捨てる。ブラウザのテストに遅れを入れれば再現できる)。
