---
id: T0827
title: スタジオ: 仮想カメラの設定と刻みの速さをプロファイルに持つ
status: in-progress
milestone: M9
crates: [apps/studio, docs]
depends_on: [T0815]
assignee: claude-worker-T0827
---

# T0827: スタジオ: 仮想カメラの設定と刻みの速さをプロファイルに持つ

## 目的

仮想カメラの設定 (解像度・フレームレート・透過を埋める色。`lib/virtualCamera.ts`) と刻みの速さ (`lib/settings.ts`) は、
操作ウィンドウの localStorage に覚えている。コメントには「プロファイル (T0802) ができたらそちらへ移す」とあるが、プロファイルは
できている (T0815 のレビューで見つけた。docs/studio.md も「プロファイルへ移すときも省ける項目として足す」としている)。
配信ごとに出力の形を切り替えられるよう、プロファイルの省ける項目として持つ。古いプロファイルは既定の値で読む。

## 受け入れ条件

- [ ] プロファイルの形式 (profile.rs・lib/profile.ts・docs/studio.md#プロファイルの形式) に省ける項目として足した。版の扱いを決めた
- [ ] localStorage に覚えた値は、初めてプロファイルに持つときに引き継ぐか捨てるかを決め、docs/studio.md に書いた
- [ ] 外部 API・ブラウザソースへの影響を確かめた
- [ ] `just ci` が通る

## 参考

- docs/studio.md#プロファイルの形式・#刻みの速さと正確さ・#仮想カメラ

## メモ
