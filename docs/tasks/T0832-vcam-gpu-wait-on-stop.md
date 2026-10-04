---
id: T0832
title: "スタジオ: 仮想カメラの GPU の待ちの時間切れを見て、受け手の GPU を作り直す"
status: todo
milestone: M9
crates: [apps/studio, docs]
depends_on: []
assignee:
---

# T0832: スタジオ: 仮想カメラの GPU の待ちの時間切れを見て、受け手の GPU を作り直す

## 目的

T0218 でヘッドレス描画の GPU を待つ時間に上限を設けた。スタジオの仮想カメラ (`apps/studio/src-tauri/src/virtual_camera.rs`) は、
止めるときの `poll(Wait::All)` の結果を捨てている。毎フレームの `submit`・`poll(Wait::No)` は待たないので止まらないが、
GPU が止まると読み戻しが来なくなり、仮想カメラの画が止まったまま理由が分からない。時間切れを見て、受け手の GPU を作り直すか止まった理由を出す。

## 受け入れ条件

- [ ] 止めるときの待ちの結果を見て、時間切れならログに書き、次の開始で GPU を作り直す
- [ ] 動いている間に読み戻しが一定の時間来ないことを見分け、作り直すか、仮想カメラの止まった理由 (T0831 の理由の一覧) として操作ウィンドウに出す。どちらにするかを docs/studio.md に書いた
- [ ] 時間切れを模したテスト (GPU の待ちを差し替える) がある
- [ ] `just ci` が通る

## 参考

- T0218 のメモ (起票の候補)、T0831
- apps/studio/src-tauri/src/virtual_camera.rs、crates/ugoe-headless/src/gpu.rs

## メモ

- T0019 で起票した (元の候補: T0218 のスタジオ)。
