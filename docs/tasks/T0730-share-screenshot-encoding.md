---
id: T0730
title: スクリーンショットの PNG の変換と保存の重複をまとめる (エディタ・ビューア)
status: in-progress
milestone: M9
crates: [apps/viewer, apps/editor, packages/player, docs]
depends_on: [T0724]
assignee: claude-worker-T0730
---

# T0730: スクリーンショットの PNG の変換と保存の重複をまとめる (エディタ・ビューア)

## 目的

スクリーンショット (T0420) の処理が、エディタとビューアにほぼ同じ形で 2 つある。

- 画面: `apps/viewer/src/lib/screenshot.ts` と `apps/editor/src/lib/viewport/screenshot.ts` (`checkScreenshot`・`encodePng`・
  `PngCanvas` などの型・`PNG_PATH_HEADER`。既定のファイル名もほぼ同じ)
- バックエンド: ビューアの `write_png_file`・`percent_decode` (`apps/viewer/src-tauri/src/lib.rs`) と `apps/editor/src-tauri/src/screenshot.rs`

片方だけ直る (例: T0724 でビューアの例外の文を訳した。エディタは英語の固定の文) ことを避けるため、画面の分を共有のパッケージ
(`Screenshot` を持つ `@ugoe/player` など) に寄せる。バックエンドの分は、共有できる場所があるか (GPL のアプリで共有するクレート) を決める。
コードレビュー (T0724) で見つけた。

## 受け入れ条件

- [ ] PNG への変換と画素の検査を 1 か所にし、エディタとビューアがそれを使う (例外の文はカタログで訳す)
- [ ] バックエンドの PNG の書き込みとパスのヘッダーの扱いを共有するか、しない理由を README に書く
- [ ] 既存のテストを移し、両方のアプリで画像の保存が動く
- [ ] `just ci` が通る

## 参考

- T0724 のメモの所見の表
- apps/viewer/README.md#画像の保存-t0420

## メモ
