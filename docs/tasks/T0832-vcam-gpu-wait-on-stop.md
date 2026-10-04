---
id: T0832
title: "スタジオ: 仮想カメラの GPU の待ちの時間切れを見て、受け手の GPU を作り直す"
status: done
milestone: M9
crates: [apps/studio, docs]
depends_on: []
assignee: claude-worker-T0832
---

# T0832: スタジオ: 仮想カメラの GPU の待ちの時間切れを見て、受け手の GPU を作り直す

## 目的

T0218 でヘッドレス描画の GPU を待つ時間に上限を設けた。スタジオの仮想カメラ (`apps/studio/src-tauri/src/virtual_camera.rs`) は、
止めるときの `poll(Wait::All)` の結果を捨てている。毎フレームの `submit`・`poll(Wait::No)` は待たないので止まらないが、
GPU が止まると読み戻しが来なくなり、仮想カメラの画が止まったまま理由が分からない。時間切れを見て、受け手の GPU を作り直すか止まった理由を出す。

## 受け入れ条件

- [x] 止めるときの待ちの結果を見て、時間切れならログに書き、次の開始で GPU を作り直す
- [x] 動いている間に読み戻しが一定の時間来ないことを見分け、作り直すか、仮想カメラの止まった理由 (T0831 の理由の一覧) として操作ウィンドウに出す。どちらにするかを docs/studio.md に書いた
- [x] 時間切れを模したテスト (GPU の待ちを差し替える) がある
- [ ] `just ci` が通る

## 参考

- T0218 のメモ (起票の候補)、T0831
- apps/studio/src-tauri/src/virtual_camera.rs、crates/ugoe-headless/src/gpu.rs

## メモ

- T0019 で起票した (元の候補: T0218 のスタジオ)。
- やったこと (`apps/studio/src-tauri/src/virtual_camera.rs`):
  - GPU の置き場を `OnceLock` から `GpuCell` (`Mutex`。作った GPU は `Box::leak` で破棄しない・作れなかったことも覚える) にし、
    `discard` で手放すと次の開始 (`get`) で作り直す。MCP サーバ (T0218) と同じく手放すだけで破棄はしない。
  - 止めるとき: `drain` が `feed.poll(gpu, Wait::All)` の結果を見て、時間切れ (`is_gpu_wait_timeout`) なら標準エラーに書いて GPU を手放す
    (常駐の描画も `mem::forget` で手放す)。ほかのエラーはログに書くだけ。
  - 動いている間: `Stall` が、読み戻しを待っている (`Feed::waiting` = `Resident::in_flight() > 0`) のに `STALL_LIMIT` (10 秒) 来ないことを見分け、
    GPU を手放して `Status::Failed { code: "gpuTimeout" }` で止める。**自動では作り直さない方にした** (本当に止まったドライバでは繰り返すうえ、
    受け手の物理の状態を持ち越せない。止まった仮想カメラは始め直さないという既存の規則とも揃う)。docs/studio.md#仮想カメラ に理由と長さを書いた。
  - `Feed::render`・`Feed::poll` のエラーを `String` から `ugoe_headless::Error` にした (時間切れを見分けるため。呼び手の `run` で文字列にする)。
  - 操作ウィンドウ: `VirtualCameraPanel.svelte` の `errorText` と `locales/{en,ja}.json` に `virtualCamera.errors.gpuTimeout` を足した (案内は無し)。
  - テスト: `a_hung_gpu_is_recreated_on_the_next_start` (待ちを差し替えて時間切れ・ほかのエラー・成功を渡し、時間切れのときだけ次で作り直す)、
    `a_failed_gpu_is_remembered`、`a_stall_is_readbacks_not_coming_back_for_the_limit` (時刻を渡して判定)。
    `just crate ugoe-studio`・`gpu-tests` の `virtual_camera_gpu` (lavapipe) が通る。
- 確かめていないこと: 実際に止まる GPU での振る舞い (実機のハングを起こせない)。`Wait::All` の上限そのものは T0218 で確かめたもの。
- やらなかったこと: 動いている間のほかの GPU のエラー (時間切れでないもの) は従来どおり `os` で止め、GPU は使い回す。
  `run` の描画・読み戻しのエラーの理由が `os` (「OS がエラーを返しました」) なのは不正確だが、このタスクでは変えていない。
- `just ci` (rebase 後、main は `d125d725`): `web-check` の `@ugoe/editor` の `ToolOverlay.test.ts` の「deletes the selected vertices with Delete」
  だけが落ちた (既知。main でも落ち、別のワーカーが直している。このタスクは apps/editor を触っていない)。`web-check` の残り
  (viewer・studio・face-tracking・player・i18n・runtime・ui の check と test) と `just i18n app-build-check script-check` は流し直して通った。
  それより前の `fmt-check lint test layers refs groups licenses publish-check spec` は 1 回目で通っている。
- 起票の候補:
  - 仮想カメラの描画・読み戻しのエラー (GPU の検証エラー・デバイスの喪失など) の理由を `os` でなく GPU の理由にする (今の文言は誤解を招く)。
