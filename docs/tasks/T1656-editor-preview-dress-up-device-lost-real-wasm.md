---
id: T1656
title: "エディタ: プレビューの着せ替えの「読み込みの途中に GPU を失う → 作り直して付け直す」を本物の wasm で通すシナリオを足す"
status: in-progress
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

- [ ] エディタの browser-test (ヘッドレスの Chrome と本物の wasm) か e2e に、プレビューの着せ替えでモジュールを読み込む途中に GPU を失わせる
      (WebGL の `WEBGL_lose_context` など) シナリオがある。作り直した後にモジュールが付いて描かれ、行に失敗の文が出ないことを確かめる
- [ ] 失っている間の付け外しが、作り直した後に付けたい組で付くことも確かめる
- [ ] どちらの仕組み (browser-test か e2e) を選んだかと理由をメモに書いた。シナリオの動かし方を、そのテストの README か docs/testing.md に書いた
- [ ] 足したシナリオと `just ci` が通る

## 参考

- T1644 のメモ (やらなかったこと・起票の候補)、T0955 (Player の読み込みの途中の喪失)
- apps/editor/src/lib/preview/dressUp.svelte.ts、docs/ui.md#webpuppet-の寿命、docs/testing.md

## メモ

- T0046 で起票した (元の候補: T1644 (本物の wasm での喪失のシナリオ))。
