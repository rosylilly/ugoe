---
id: T0729
title: ビューアのコマンドのエラーをコードで返し、画面で訳す
status: in-progress
milestone: M9
crates: [apps/viewer]
depends_on: [T0724]
assignee: claude-worker-T0729
---

# T0729: ビューアのコマンドのエラーをコードで返し、画面で訳す

## 目的

ADR 0029 は「Tauri のコマンドのエラーは `code`・`args`・`message` で渡し、画面で訳す」と決めているが、ビューアのコマンド
(`apps/viewer/src-tauri/src/lib.rs`) は英語の文 (`format!("{}: file is too large ...")` など) を `String` で返し、画面はそれを
そのまま出す (ドロップで読めなかったものの知らせの `detail`、画像の保存の失敗、トラッキングの記録の保存の失敗)。
日本語の画面に英語の文が混ざる。コードレビュー (T0724) で見つけた。

## 受け入れ条件

- [ ] コマンドのエラーを ADR 0029 の形 (`code`・`args`・`message`) で返す。コードの例: `file.not_model`・`file.too_large`・
  `file.not_found`・`file.read_failed`・`file.not_json`・`file.not_png`・`file.write_failed`・`request.missing_path`
- [ ] 画面 (`host.ts` と、エラーを出すところ) が `errors.<code>` を引いて訳す (カタログは en と ja)。カタログに無いコードは `message`
- [ ] エディタとスタジオのエラーの扱い (どこで訳すか) と揃える
- [ ] テストがある (Rust のコードと引数、画面の訳)
- [ ] `just ci` が通る

## 参考

- docs/adr/0029-shared-ui-and-i18n.md (エラー)
- T0724 のメモの所見の表

## メモ
