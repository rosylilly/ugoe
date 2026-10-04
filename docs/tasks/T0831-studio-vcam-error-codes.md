---
id: T0831
title: 仮想カメラの止まった理由に、再起動が要る・機器を開く権限が無いを足す
status: todo
milestone: M9
crates: [apps/studio, docs]
depends_on: [T0816]
assignee:
---

# T0831: 仮想カメラの止まった理由に、再起動が要る・機器を開く権限が無いを足す

## 目的

T0816 のレビューで、利用者のすべきことが決まっているのに、汎用の理由や誤った案内になる場合が見つかった。
`ugoe_studio_vcam::Error` に理由を足し、操作ウィンドウで文言と案内を出す (ADR 0029 の 4.: 文言はコードで選ぶ)。

1. **macOS で再起動が要る:** `OSSystemExtensionRequest` が `willCompleteAfterReboot` で終わると、いまは `extensionFailed` に
   英語の詳細 ("the camera extension will be activated after a restart") を付けるだけ (`apps/studio/vcam/src/cmio.rs` の `finished`)。
   `needsRestart` のような理由にし、「再起動してください」を出す。
2. **Linux で機器を開く権限が無い:** `/dev/video*` を開けない機器は飛ばすので (`apps/studio/vcam/src/v4l2.rs` の `find_device`)、
   v4l2loopback の機器があっても権限が無ければ `noDevice` になり、v4l2loopback の入れ方を案内してしまう。開けなかった理由が
   権限 (`EACCES`) なら、`video` のグループに入る案内を出す。

## 受け入れ条件

- [ ] `Error` に 2 つの理由を足し、`Error::code` とテストを更新した
- [ ] `apps/studio/locales/*.json` の `virtualCamera.errors.*` と `VirtualCameraPanel.svelte` の案内を足した
- [ ] `docs/studio.md#仮想カメラ` の知らせの表と、Linux・macOS の節を更新した
- [ ] `just ci` が通る

## 参考

- docs/studio.md#仮想カメラ、ADR 0029
- T0816 のメモ

## メモ
