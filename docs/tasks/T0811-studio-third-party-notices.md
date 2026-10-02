---
id: T0811
title: スタジオ: 第三者のライセンスの表記 (MediaPipe などの同梱物) を出す画面
status: in-progress
milestone: M8
crates: [apps/studio, docs]
depends_on: [T0803]
assignee: claude-worker-T0811
---

# T0811: スタジオのライセンスの表記

## 目的

スタジオは T0803 で MediaPipe (Apache-2.0) のモデルと wasm を同梱するようになったが、ビューアやエディタのようなライセンスの表記の画面が無い (T0803 のメモ)。

## 受け入れ条件

- [ ] ビューアの LicenseDialog と同じ形で、スタジオ自身 (GPL-3.0-only) と同梱する第三者 (MediaPipe・bits-ui・i18next など) のライセンスを出す画面を操作ウィンドウに置く
- [ ] third-party-notices に同梱する JS と MediaPipe の資産を載せる (ビューアの T0723 の作り方にそろえる)
- [ ] 文言は @ugoe/i18n、部品は @ugoe/ui。`just ci` が通る

## 参考

- apps/viewer の LicenseDialog と third-party-notices.txt、packages/face-tracking/third-party-notices.txt

## メモ
