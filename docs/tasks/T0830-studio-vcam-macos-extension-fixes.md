---
id: T0830
title: macOS の Camera Extension の直しと実機での確かめ
status: todo
milestone: M9
crates: [apps/studio, docs]
depends_on: [T0816]
assignee:
---

# T0830: macOS の Camera Extension の直しと実機での確かめ

## 目的

T0816 のレビューで、macOS の仮想カメラ (`apps/studio/vcam/macos/Extension/Camera.swift` と `src/cmio.rs`) に、Swift のコンパイルか
実機が要るために直せなかった所見が残った。macOS の上で直し、`docs/studio.md#macos-camera-extension` の「未確認」を確かめる。

1. `CameraStream` の `activeFormatIndex`・`frameRate` は、CoreMediaIO のクライアントのキュー (`setStreamProperties`) で書かれ、
   画のキュー (`Camera.queue` の `tick`・`startSource`) で読まれる。同期が無い (データ競合)。`Camera.queue` の上に寄せる。
2. 出す側が動いている間にフレームの長さ (`streamFrameDuration`) が変わっても、タイマーの間隔を作り直さない。
3. スタジオ (`cmio.rs`) は受ける側のストリームの書式を設定の大きさに合わせない (既定は 0 番の 1280×720)。1920×1080 の画を積んで
   受け取られるかを確かめ、受け取られなければ `kCMIOStreamPropertyFormatDescription` を設定に合わせる (`docs/studio.md` の未確認の 9.)。
4. 拡張が入れ替わった・落ちたときに、スタジオの持つキューが取り出されないまま残り、「カメラを使っているアプリがありません」が
   出続ける (機器を失ったことが分からない)。機器が消えたことを知る方法 (`kCMIOHardwarePropertyDevices` の変化など) を調べる。
5. `docs/studio.md#macos-camera-extension` の「未確認」の 1.〜9. を確かめ、結果を書く。

## 受け入れ条件

- [ ] 上の 1.〜4. を直した (直さないものは理由をメモに書いた)
- [ ] `pnpm vcam:macos` で拡張が作れる
- [ ] 「未確認」の項目を確かめ、`docs/studio.md` を更新した
- [ ] `just ci` が通る

## 参考

- docs/studio.md#macos-camera-extension、ADR 0028 の 8.
- T0816 のメモ

## メモ

- macOS の実機と Apple Developer Program のアカウントが要る (ADR 0028 の 8.)。
